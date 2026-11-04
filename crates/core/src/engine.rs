use std::future::Future;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use tokio::sync::{oneshot, watch};

use crate::lifecycle::{EnginePhase, Lifecycle};
use tokio::task::{JoinHandle, JoinSet};
use crate::clients::ClientId;
use crate::constants::DEFAULT_PROCESS_NAME;

use crate::capture::{Capture, CaptureConfig, CapturedTransport, is_capture_flow_closed};
use crate::config::{HandlerConfig, HandlerKind, Transport};
use crate::connection::relay::relay;
use crate::features::FeatureConfig;
use crate::handlers::HandlerFactory;
use crate::sessions::SessionStore;
use crate::tls::{TlsConfig, TlsMitm, UncleanTlsClose};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub client: ClientId,
    pub capture: CaptureConfig,
    pub tls: Option<TlsConfig>,
    pub handlers: HandlerConfig,
    pub features: FeatureConfig,
}

impl EngineConfig {
    fn validate(&self) -> Result<()> {
        self.handlers.validate()?;
        self.features.validate()?;

        let protocol = self.client.profile().world.with_context(|| {
            format!(
                "World protocol is not configured for {:?}",
                self.client,
            )
        })?;

        protocol.opcodes.validate()?;

        let world_enabled = self.handlers.rules.iter().any(|rule| {
            rule.transport == Transport::Tcp
                && self.handlers.select(Transport::Tcp, rule.port) == HandlerKind::World
        });

        ensure!(
            self.features.world_time.is_none() || world_enabled,
            "World time requires an enabled World handler",
        );

        if world_enabled {
            let bnet_tls_enabled = self.handlers.rules.iter().any(|rule| {
                rule.transport == Transport::Tcp
                    && self.handlers.select(Transport::Tcp, rule.port) == HandlerKind::Bnet
                    && self
                        .tls
                        .as_ref()
                        .is_some_and(|tls| tls.ports.contains(&rule.port))
            });

            ensure!(
                bnet_tls_enabled,
                "World decoding requires TLS interception \
                 of a Battle.net handler",
            );
        }

        Ok(())
    }

    pub fn for_client(client: ClientId) -> Self {
        Self {
            client,
            capture: CaptureConfig {
                filter: DEFAULT_PROCESS_NAME.to_owned(),
                helper_executable: None,
            },
            tls: Some(TlsConfig::for_client(client)),
            handlers: HandlerConfig::for_client(client),
            features: FeatureConfig::default(),
        }
    }
}

#[must_use = "Keep the handle alive until the engine is stopped"]
pub struct EngineHandle {
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<()>>>,
    phase: watch::Receiver<EnginePhase>,
}

impl EngineHandle {
    /// Cancellation-safe: the task remains owned by this handle.
    pub async fn wait(&mut self) -> Result<()> {
        let result = match self.task.as_mut() {
            Some(task) => task.await,
            None => return Ok(()),
        };

        self.task = None;
        self.shutdown = None;

        result.context("Engine task panicked")?
    }

    pub(crate) async fn stop(mut self) -> Result<()> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }

        self.wait().await
    }

    pub fn subscribe_phase(&self) -> watch::Receiver<EnginePhase> {
        self.phase.clone()
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

pub(crate) async fn start(
    mut config: EngineConfig,
) -> Result<EngineHandle> {
    config.features = config.features.for_client(config.client);
    config.validate()?;

    let EngineConfig {
        client,
        capture: capture_config,
        tls: tls_config,
        handlers,
        features,
    } = config;

    let (lifecycle, phase) = Lifecycle::new();

    let sessions = Arc::new(SessionStore::default());

    let factory = HandlerFactory::new(
        client,
        sessions,
        features,
        lifecycle,
    )?;

    let tls = tls_config.map(TlsMitm::load).transpose()?.map(Arc::new);

    log::info!("Starting capture for {:?}", capture_config.filter,);

    let mut capture = Capture::start(capture_config).await?;

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let task = tokio::spawn(async move {
        let mut connections = JoinSet::new();

        let run_result: Result<()> = async {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
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

                            if let Err(error) = relay(
                                connection,
                                connection_tls,
                                handler,
                            )
                            .await
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

        stop_result?;
        log::info!("Capture stopped");

        Ok(())
    });

    log::info!("Capture started");

    Ok(EngineHandle {
        shutdown: Some(shutdown_tx),
        task: Some(task),
        phase,
    })
}

/// Compatibility entry point using the same start/stop implementation.
pub async fn run(config: EngineConfig, shutdown: impl Future<Output = Result<()>>) -> Result<()> {
    let mut engine = start(config).await?;

    tokio::pin!(shutdown);

    let result = tokio::select! {
        result = &mut shutdown => result,
        result = engine.wait() => return result,
    };

    let stop_result = engine.stop().await;

    result.context("Shutdown signal failed")?;
    stop_result
}
