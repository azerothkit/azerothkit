mod stream;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use mitmproxy::intercept_conf::InterceptConf;
use mitmproxy::messages::{TransportCommand, TransportEvent, TunnelInfo};
use mitmproxy::packet_sources::{PacketSourceConf, PacketSourceTask};
use mitmproxy::shutdown;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use super::{CaptureConfig, CapturedConnection, RemoteEndpoint};

pub use stream::CapturedStream;

static ACTIVE: AtomicBool = AtomicBool::new(false);

struct SessionGuard {
    shutdown: watch::Sender<()>,
}

impl SessionGuard {
    fn acquire(shutdown: watch::Sender<()>) -> Result<Self> {
        ensure!(
            ACTIVE
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire,)
                .is_ok(),
            "Capture is already running"
        );

        let guard = Self { shutdown };

        #[cfg(target_os = "macos")]
        match std::fs::remove_file(socket_path()) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).context("Failed to remove stale capture socket");
            }
        }

        Ok(guard)
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());

        #[cfg(target_os = "macos")]
        if let Err(error) = std::fs::remove_file(socket_path()) {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to remove capture socket: {error}");
            }
        }

        ACTIVE.store(false, Ordering::Release);
    }
}

#[cfg(target_os = "macos")]
fn socket_path() -> std::path::PathBuf {
    format!("/tmp/mitmproxy-{}", std::process::id()).into()
}

pub struct Capture {
    events: mpsc::Receiver<TransportEvent>,
    commands: mpsc::UnboundedSender<TransportCommand>,
    filters: mpsc::UnboundedSender<InterceptConf>,
    shutdown: watch::Sender<()>,
    task: Option<JoinHandle<Result<()>>>,
    next_id: u64,
}

impl Capture {
    pub async fn start(config: CaptureConfig) -> Result<Self> {
        let CaptureConfig {
            filter,
            helper_executable,
        } = config;

        ensure!(
            !filter.trim().is_empty(),
            "Capture filter must not be empty"
        );

        let spec = format!("{},!{}", filter.trim(), std::process::id());
        let intercept = InterceptConf::try_from(spec.as_str()).context("Invalid capture filter")?;

        #[cfg(target_os = "macos")]
        let source = {
            let _ = helper_executable;

            let executable = std::path::Path::new(
                "/Applications/Mitmproxy Redirector.app/Contents/MacOS/Mitmproxy Redirector",
            );

            ensure!(
                executable.is_file(),
                "Mitmproxy Redirector.app is not installed in /Applications"
            );

            mitmproxy::packet_sources::macos::MacosConf
        };

        #[cfg(windows)]
        let source = mitmproxy::packet_sources::windows::WindowsConf {
            executable_path: helper_executable
                .context("Windows redirector executable path is required")?,
        };

        #[cfg(target_os = "linux")]
        let source = mitmproxy::packet_sources::linux::LinuxConf {
            executable_path: helper_executable
                .context("Linux redirector executable path is required")?,
        };

        let (events_tx, events_rx) = mpsc::channel(256);
        let (commands_tx, commands_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = shutdown::channel();

        let guard = SessionGuard::acquire(shutdown_tx.clone())?;

        let (source_task, filters) = source
            .build(events_tx, commands_rx, shutdown_rx)
            .await
            .context("Failed to initialize capture backend")?;

        filters
            .send(intercept)
            .context("Failed to configure process interception")?;

        let task = tokio::spawn(async move {
            let _guard = guard;
            source_task.run().await
        });

        Ok(Self {
            events: events_rx,
            commands: commands_tx,
            filters,
            shutdown: shutdown_tx,
            task: Some(task),
            next_id: 1,
        })
    }

    pub async fn next_connection(&mut self) -> Result<Option<CapturedConnection>> {
        loop {
            let Some(event) = self.events.recv().await else {
                self.stop().await?;
                return Ok(None);
            };

            let TransportEvent::ConnectionEstablished {
                connection_id,
                dst_addr,
                tunnel_info,
                command_tx,
                ..
            } = event;

            let commands = command_tx.unwrap_or_else(|| self.commands.clone());

            if !connection_id.is_tcp() {
                log::warn!("TCP prototype: closing captured UDP connection");

                let _ = commands.send(TransportCommand::CloseConnection(connection_id, false));

                continue;
            }

            let (pid, process_name, remote_endpoint) = match tunnel_info {
                TunnelInfo::LocalRedirector {
                    pid,
                    process_name,
                    remote_endpoint,
                } => (pid, process_name, remote_endpoint),
                _ => (None, None, None),
            };

            let (host, port) =
                remote_endpoint.unwrap_or_else(|| (dst_addr.ip().to_string(), dst_addr.port()));

            let id = self.next_id;
            self.next_id += 1;

            return Ok(Some(CapturedConnection {
                id,
                pid,
                process_name,
                remote: RemoteEndpoint { host, port },
                stream: CapturedStream::new(connection_id, commands),
            }));
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        let _ = self.filters.send(InterceptConf::disabled());
        let _ = self.shutdown.send(());

        if let Some(mut task) = self.task.take() {
            match timeout(Duration::from_secs(5), &mut task).await {
                Ok(result) => {
                    result.context("Capture backend task panicked")??;
                }
                Err(_) => {
                    task.abort();
                    let _ = task.await;
                    anyhow::bail!("Capture backend did not stop within 5 seconds");
                }
            }
        }

        Ok(())
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());
    }
}
