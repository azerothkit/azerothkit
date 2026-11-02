use std::future::Future;

use anyhow::{Context, Result};
use tokio::task::JoinSet;

use crate::capture::{Capture, CaptureConfig};
use crate::connection::relay::relay;
use crate::tls::UncleanTlsClose;

use std::sync::Arc;

use crate::tls::{TlsConfig, TlsMitm};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub capture: CaptureConfig,
    pub tls: Option<TlsConfig>,
}

pub async fn run(config: EngineConfig, shutdown: impl Future<Output = Result<()>>) -> Result<()> {
    let tls = config.tls.map(TlsMitm::load).transpose()?.map(Arc::new);

    log::info!("Starting capture for {:?}", config.capture.filter,);

    let mut capture = Capture::start(config.capture).await?;
    let mut connections = JoinSet::new();

    tokio::pin!(shutdown);

    log::info!("Capture started");

    let run_result: Result<()> = async {
        loop {
            tokio::select! {
                result = &mut shutdown => {
                    result.context("Shutdown signal failed")?;
                    break;
                }

                connection = capture.next_connection() => {
                    let Some(connection) = connection? else {
                        break;
                    };

                    let tls = tls.clone();

                    connections.spawn(async move {
                        let id = connection.id;

                        if let Err(error) = relay(connection, tls).await {
                            if error.is::<UncleanTlsClose>() {
                                log::debug!("[{id}] {error:#}");
                            } else {
                                log::warn!("[{id}] {error:#}");
                            }
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

    if let Err(error) = run_result {
        if let Err(stop_error) = stop_result {
            log::error!("Failed to stop capture: {stop_error:#}");
        }

        return Err(error);
    }

    stop_result
}
