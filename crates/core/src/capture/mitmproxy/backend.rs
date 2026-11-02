use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use mitmproxy::intercept_conf::InterceptConf;
use mitmproxy::messages::{TransportCommand, TransportEvent, TunnelInfo};
use mitmproxy::packet_sources::{PacketSourceConf, PacketSourceTask};
use mitmproxy::shutdown;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::capture::{CaptureConfig, CapturedConnection, CapturedTransport, RemoteEndpoint};

use super::{CapturedDatagram, CapturedStream};

const EVENT_QUEUE_CAPACITY: usize = 256;
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

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
            "Capture is already running",
        );

        Ok(Self { shutdown })
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());
        ACTIVE.store(false, Ordering::Release);
    }
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
            "Capture filter must not be empty",
        );

        // Upstream sockets created by this process must not be intercepted.
        let spec = format!("{},!{}", filter.trim(), std::process::id(),);

        let intercept = InterceptConf::try_from(spec.as_str()).context("Invalid capture filter")?;

        #[cfg(target_os = "macos")]
        let source = {
            let _ = helper_executable;

            let executable = std::path::Path::new(
                "/Applications/Mitmproxy Redirector.app/Contents/MacOS/Mitmproxy Redirector",
            );

            ensure!(
                executable.is_file(),
                "Mitmproxy Redirector.app is not installed in /Applications",
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

        #[cfg(not(any(target_os = "macos", target_os = "linux", windows,)))]
        compile_error!("Local capture supports only macOS, Linux and Windows");

        let (events_tx, events_rx) = mpsc::channel(EVENT_QUEUE_CAPACITY);

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

        // macOS supplies a separate command channel for each flow.
        let commands = command_tx.unwrap_or_else(|| self.commands.clone());

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

        let transport = if connection_id.is_tcp() {
            CapturedTransport::Tcp(CapturedStream::new(connection_id, commands))
        } else {
            CapturedTransport::Udp(CapturedDatagram::new(connection_id, commands, dst_addr))
        };

        let id = self.next_id;

        self.next_id = self
            .next_id
            .checked_add(1)
            .context("Capture connection ID exhausted")?;

        Ok(Some(CapturedConnection {
            id,
            pid,
            process_name,
            remote: RemoteEndpoint { host, port },
            transport,
        }))
    }

    pub async fn stop(&mut self) -> Result<()> {
        let _ = self.filters.send(InterceptConf::disabled());
        let _ = self.shutdown.send(());

        let Some(mut task) = self.task.take() else {
            return Ok(());
        };

        match timeout(STOP_TIMEOUT, &mut task).await {
            Ok(result) => {
                result.context("Capture backend task panicked")??;

                Ok(())
            }
            Err(_) => {
                task.abort();
                let _ = task.await;

                bail!("Capture backend did not stop within 5 seconds",);
            }
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.filters.send(InterceptConf::disabled());
        let _ = self.shutdown.send(());
    }
}
