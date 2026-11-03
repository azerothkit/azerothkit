use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use tokio::net::UdpSocket;
use tokio::time::timeout;

use crate::capture::CapturedDatagram;
use crate::handlers::{Direction, Handler, HandlerInput, HandlerOutput};

use super::relay::output_chunks;

const UDP_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_DATAGRAM_SIZE: usize = 65_536;

#[derive(Default)]
struct TransferStats {
    packets: u64,
    bytes: u64,
}

impl TransferStats {
    fn record(&mut self, connection_id: u64, direction: Direction, size: usize) {
        self.packets += 1;
        self.bytes += size as u64;

        if self.packets == 1 {
            log::info!(
                "[{connection_id}] UDP forwarded: \
                 direction={direction:?}, first_datagram_bytes={size}",
            );
        }
    }
}

pub(super) async fn relay(
    connection_id: u64,
    mut downstream: CapturedDatagram,
    mut handler: Box<dyn Handler>,
) -> Result<(u64, u64)> {
    let remote = downstream.remote_addr();

    let local = if remote.is_ipv4() {
        SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))
    } else {
        SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0))
    };

    let upstream = UdpSocket::bind(local)
        .await
        .context("Failed to bind upstream UDP socket")?;

    let mut buffer = vec![0u8; MAX_DATAGRAM_SIZE];
    let mut uploaded = TransferStats::default();
    let mut downloaded = TransferStats::default();

    let result: Result<()> = async {
        loop {
            let step = timeout(UDP_IDLE_TIMEOUT, async {
                tokio::select! {
                    packet = downstream.recv() => {
                        let packet = packet
                            .context("Captured UDP receive failed")?;

                        let output = handler.handle(HandlerInput {
                            direction: Direction::ClientToServer,
                            data: Bytes::from(packet),
                        })?;

                        send_upstream(
                            connection_id,
                            &upstream,
                            remote,
                            output,
                            &mut uploaded,
                        )
                        .await?;

                        Ok::<(), anyhow::Error>(())
                    }

                    size = recv_upstream(
                        &upstream,
                        remote,
                        &mut buffer,
                    ) => {
                        let size = size?;

                        let output = handler.handle(HandlerInput {
                            direction: Direction::ServerToClient,
                            data: Bytes::copy_from_slice(
                                &buffer[..size],
                            ),
                        })?;

                        send_downstream(
                            connection_id,
                            &mut downstream,
                            output,
                            &mut downloaded,
                        )
                        .await?;

                        Ok(())
                    }
                }
            })
            .await;

            match step {
                Ok(result) => result?,
                Err(_) => {
                    log::debug!("[{connection_id}] UDP flow timed out",);
                    break;
                }
            }
        }

        Ok(())
    }
    .await;

    log::info!(
        "[{connection_id}] UDP summary: remote={remote}, \
         to_server_packets={}, to_server_bytes={}, \
         to_client_packets={}, to_client_bytes={}, success={}",
        uploaded.packets,
        uploaded.bytes,
        downloaded.packets,
        downloaded.bytes,
        result.is_ok(),
    );

    result?;

    Ok((uploaded.bytes, downloaded.bytes))
}

async fn recv_upstream(
    upstream: &UdpSocket,
    remote: SocketAddr,
    buffer: &mut [u8],
) -> Result<usize> {
    loop {
        let (size, source) = upstream
            .recv_from(buffer)
            .await
            .with_context(|| format!("UDP receive failed for {remote}"))?;

        if source == remote {
            return Ok(size);
        }

        log::debug!("Ignoring UDP datagram from {source}; expected {remote}",);
    }
}

async fn send_upstream(
    connection_id: u64,
    upstream: &UdpSocket,
    remote: SocketAddr,
    output: HandlerOutput,
    stats: &mut TransferStats,
) -> Result<()> {
    for datagram in output_chunks(output) {
        let sent = upstream
            .send_to(&datagram, remote)
            .await
            .with_context(|| format!("UDP send failed to {remote}"))?;

        ensure!(
            sent == datagram.len(),
            "Partial UDP datagram sent to {remote}: {sent}/{}",
            datagram.len(),
        );

        stats.record(connection_id, Direction::ClientToServer, sent);
    }

    Ok(())
}

async fn send_downstream(
    connection_id: u64,
    downstream: &mut CapturedDatagram,
    output: HandlerOutput,
    stats: &mut TransferStats,
) -> Result<()> {
    for datagram in output_chunks(output) {
        downstream
            .send(&datagram)
            .await
            .context("Captured UDP send failed")?;

        stats.record(connection_id, Direction::ServerToClient, datagram.len());
    }

    Ok(())
}
