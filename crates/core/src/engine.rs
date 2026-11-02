use std::future::Future;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::task::JoinSet;

use crate::capture::{Capture, CaptureConfig, CapturedTransport};
use crate::config::{HandlerConfig, Transport};
use crate::connection::relay::relay;
use crate::handlers::HandlerFactory;
use crate::session_keys::SessionKeys;
use crate::tls::{TlsConfig, TlsMitm, UncleanTlsClose};
use crate::capture::is_capture_flow_closed;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub capture: CaptureConfig,
    pub tls: Option<TlsConfig>,
    pub handlers: HandlerConfig,
}

pub async fn run(config: EngineConfig, shutdown: impl Future<Output = Result<()>>) -> Result<()> {
    let EngineConfig {
        capture: capture_config,
        tls: tls_config,
        handlers,
    } = config;

    handlers.validate()?;

    let keys = Arc::new(SessionKeys::default());
    let factory = HandlerFactory::new(keys);

    let tls = tls_config.map(TlsMitm::load).transpose()?.map(Arc::new);

    log::info!("Starting capture for {:?}", capture_config.filter,);

    let mut capture = Capture::start(capture_config).await?;
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

                    let transport = match &connection.transport {
                        CapturedTransport::Tcp(_) => Transport::Tcp,
                        CapturedTransport::Udp(_) => Transport::Udp,
                    };

                    let kind = handlers.select(
                        transport,
                        connection.remote.port,
                    );

                    let handler = factory.create(
                        kind,
                        transport,
                        connection.id,
                    )?;

                    let connection_tls =
                        if transport == Transport::Tcp {
                            tls.as_ref()
                                .filter(|tls| {
                                    tls.intercepts_port(
                                        connection.remote.port,
                                    )
                                })
                                .cloned()
                        } else {
                            None
                        };

                    connections.spawn(async move {
                        let id = connection.id;

                        if let Err(error) =
                            relay(connection, connection_tls, handler).await
                        {
                            if is_capture_flow_closed(&error)
                                || error.is::<UncleanTlsClose>()
                            {
                                log::debug!("[{id}] {error:#}");
                            } else {
                                log::warn!("[{id}] {error:#}");
                            }
                        }
                    });
                }

                Some(result) = connections.join_next() => {
                    if let Err(error) = result {
                        log::error!(
                            "Connection task panicked: {error}",
                        );
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
            log::error!("Failed to stop capture: {stop_error:#}",);
        }

        return Err(error);
    }

    stop_result
}
