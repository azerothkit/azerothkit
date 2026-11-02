use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use azerothkit_core::capture::{Capture, CaptureConfig, CapturedConnection};
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut args = std::env::args().skip(1);

    let filter = args
        .next()
        .context("Usage: azerothkit-cli <process-filter> [helper-executable]")?;

    let helper_executable = args.next().map(PathBuf::from);

    log::info!("Starting capture for {filter:?}");

    let mut capture = Capture::start(CaptureConfig {
        filter,
        helper_executable,
    })
    .await?;

    log::info!("Capture started. Press Ctrl+C to stop.");

    let mut connections = JoinSet::new();
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);

    let run_result: Result<()> = async {
        loop {
            tokio::select! {
                result = &mut interrupt => {
                    result.context("Failed to receive Ctrl+C")?;
                    break;
                }

                connection = capture.next_connection() => {
                    let Some(connection) = connection? else {
                        break;
                    };

                    connections.spawn(async move {
                        let id = connection.id;

                        if let Err(error) = relay(connection).await {
                            log::warn!("[{id}] {error:#}");
                        }
                    });
                }

                Some(result) = connections.join_next() => {
                    if let Err(error) = result {
                        log::error!("Connection task panicked: {error}");
                    }
                }
            }
        }

        Ok(())
    }
    .await;

    connections.shutdown().await;
    let stop_result = capture.stop().await;

    run_result?;
    stop_result
}

async fn relay(mut connection: CapturedConnection) -> Result<()> {
    log::info!(
        "[{}] process={:?}, pid={:?}, remote={}:{}",
        connection.id,
        connection.process_name,
        connection.pid,
        connection.remote.host,
        connection.remote.port,
    );

    let mut upstream = timeout(
        Duration::from_secs(10),
        TcpStream::connect((connection.remote.host.as_str(), connection.remote.port)),
    )
    .await
    .context("Upstream connection timed out")?
    .context("Failed to connect to upstream")?;

    upstream.set_nodelay(true)?;

    let (uploaded, downloaded) = copy_bidirectional(&mut connection.stream, &mut upstream)
        .await
        .context("TCP relay failed")?;

    log::info!(
        "[{}] closed: uploaded={}, downloaded={}",
        connection.id,
        uploaded,
        downloaded,
    );

    Ok(())
}
