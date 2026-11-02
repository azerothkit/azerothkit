use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::capture::{CapturedStream, RemoteEndpoint};
use crate::handlers::{Direction, Handler, HandlerInput, HandlerOutput};
use crate::tls::{TlsMitm, UncleanTlsClose};

use super::relay::output_chunks;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_BUFFER_SIZE: usize = 16 * 1024;

pub(super) async fn relay(
    connection_id: u64,
    remote: &RemoteEndpoint,
    downstream: CapturedStream,
    tls: Option<Arc<TlsMitm>>,
    handler: Box<dyn Handler>,
) -> Result<(u64, u64)> {
    let upstream = timeout(
        CONNECT_TIMEOUT,
        TcpStream::connect((remote.host.as_str(), remote.port)),
    )
    .await
    .context("Upstream TCP connection timed out")?
    .context("Failed to connect to upstream TCP server")?;

    upstream
        .set_nodelay(true)
        .context("Failed to enable TCP_NODELAY")?;

    if let Some(tls) = tls {
        let (downstream, upstream) = tls
            .handshake(connection_id, remote, downstream, upstream)
            .await?;

        match relay_streams(downstream, upstream, handler).await {
            Err(error)
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|error| error.kind() == io::ErrorKind::UnexpectedEof) =>
            {
                Err(error).context(UncleanTlsClose)
            }
            result => result.context("TLS relay failed"),
        }
    } else {
        relay_streams(downstream, upstream, handler)
            .await
            .context("TCP relay failed")
    }
}

async fn relay_streams<A, B>(
    downstream: A,
    upstream: B,
    handler: Box<dyn Handler>,
) -> Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let (client_reader, client_writer) = tokio::io::split(downstream);
    let (server_reader, server_writer) = tokio::io::split(upstream);

    // Both directions share one handler. The lock is never held across await.
    let handler = Mutex::new(handler);

    tokio::try_join!(
        relay_direction(
            client_reader,
            server_writer,
            Direction::ClientToServer,
            &handler,
        ),
        relay_direction(
            server_reader,
            client_writer,
            Direction::ServerToClient,
            &handler,
        ),
    )
}

async fn relay_direction<R, W>(
    mut reader: R,
    mut writer: W,
    direction: Direction,
    handler: &Mutex<Box<dyn Handler>>,
) -> Result<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = vec![0u8; READ_BUFFER_SIZE];
    let mut transferred = 0u64;

    loop {
        let size = reader
            .read(&mut buffer)
            .await
            .with_context(|| format!("{direction:?}: read failed"))?;

        let output = {
            let mut handler = handler
                .lock()
                .map_err(|_| anyhow!("Connection handler is poisoned"))?;

            if size == 0 {
                handler.finish(direction)
            } else {
                handler.handle(HandlerInput {
                    direction,
                    data: Bytes::copy_from_slice(&buffer[..size]),
                })
            }
        }
        .with_context(|| format!("{direction:?}: handler failed"))?;

        transferred += write_output(&mut writer, output)
            .await
            .with_context(|| format!("{direction:?}: write failed"))?;

        if size == 0 {
            writer
                .shutdown()
                .await
                .with_context(|| format!("{direction:?}: write shutdown failed"))?;

            return Ok(transferred);
        }
    }
}

async fn write_output<W>(writer: &mut W, output: HandlerOutput) -> Result<u64>
where
    W: AsyncWrite + Unpin,
{
    let mut transferred = 0u64;
    let mut wrote_data = false;

    for data in output_chunks(output) {
        writer.write_all(&data).await?;
        transferred += data.len() as u64;
        wrote_data = true;
    }

    if wrote_data {
        writer.flush().await?;
    }

    Ok(transferred)
}
