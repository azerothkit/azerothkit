use azerothkit_core::{
    actions,
    app_paths::AppPaths,
    engine::EngineHandle,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, watch};

const ENGINE_STATUS_CHANGED: &str = "engine-status-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum EngineStatus {
    Stopped,
    Starting,
    WaitingForGame,
    WaitingForKeys,
    Running,
    Stopping,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EngineSnapshot {
    pub status: EngineStatus,
    pub revision: u32,
}

type CommandResult = Result<EngineSnapshot, String>;

enum Operation {
    Start,
    Stop,
    Restart,
}

struct Command {
    operation: Operation,
    reply: oneshot::Sender<CommandResult>,
}

pub struct EngineState {
    snapshot: watch::Receiver<EngineSnapshot>,
    commands: mpsc::Sender<Command>,
}

impl EngineState {
    pub fn new(app: AppHandle, paths: AppPaths) -> Self {
        let initial = EngineSnapshot {
            status: EngineStatus::Stopped,
            revision: 0,
        };

        let (snapshot_tx, snapshot_rx) = watch::channel(initial);
        let (command_tx, command_rx) = mpsc::channel(16);

        let controller = EngineController {
            app,
            paths,
            snapshot: snapshot_tx,
            engine: None,
        };

        tauri::async_runtime::spawn(controller.run(command_rx));

        Self {
            snapshot: snapshot_rx,
            commands: command_tx,
        }
    }

    pub fn snapshot(&self) -> CommandResult {
        Ok(*self.snapshot.borrow())
    }

    pub async fn start(&self) -> CommandResult {
        self.request(Operation::Start).await
    }

    pub async fn stop(&self) -> CommandResult {
        self.request(Operation::Stop).await
    }

    pub async fn restart(&self) -> CommandResult {
        self.request(Operation::Restart).await
    }

    async fn request(&self, operation: Operation) -> CommandResult {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.commands
            .send(Command {
                operation,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "Engine controller is unavailable".to_owned())?;

        // The controller keeps executing even if this request is dropped.
        reply_rx
            .await
            .map_err(|_| "Engine controller stopped before replying".to_owned())?
    }
}

struct EngineController {
    app: AppHandle,
    paths: AppPaths,
    snapshot: watch::Sender<EngineSnapshot>,
    engine: Option<EngineHandle>,
}

impl EngineController {
    async fn run(mut self, mut commands: mpsc::Receiver<Command>) {
        loop {
            let command = if let Some(engine) = self.engine.as_mut() {
                tokio::select! {
                    biased;

                    result = engine.wait() => {
                        self.engine = None;
                        let _ = self.finish(result);
                        continue;
                    }

                    command = commands.recv() => command,
                }
            } else {
                commands.recv().await
            };

            let Some(command) = command else {
                let _ = self.stop().await;
                break;
            };

            let result = match command.operation {
                Operation::Start => self.start().await,
                Operation::Stop => self.stop().await,
                Operation::Restart => self.restart().await,
            };

            let _ = command.reply.send(result);
        }
    }

    async fn start(&mut self) -> CommandResult {
        if self.engine.is_some() {
            return Ok(self.current_snapshot());
        }

        self.publish(EngineStatus::Starting);
        log::info!("Starting engine from saved settings");

        match actions::start_from_settings(&self.paths).await {
            Ok(engine) => {
                self.engine = Some(engine);
                Ok(self.publish(EngineStatus::WaitingForGame))
            }
            Err(error) => {
                let message = format!("Failed to start engine: {error:#}");

                log::error!("{message}");
                self.publish(EngineStatus::Stopped);

                Err(message)
            }
        }
    }

    async fn stop(&mut self) -> CommandResult {
        let Some(engine) = self.engine.take() else {
            return Ok(self.current_snapshot());
        };

        self.publish(EngineStatus::Stopping);
        log::info!("Stopping engine");

        let result = actions::stop(engine).await;
        self.finish(result)
    }

    async fn restart(&mut self) -> CommandResult {
        self.stop().await?;
        self.start().await
    }

    fn finish(&mut self, result: anyhow::Result<()>) -> CommandResult {
        let error = result.err().map(|error| {
            let message = format!("Engine terminated with an error: {error:#}");
            log::error!("{message}");
            message
        });

        if error.is_none() {
            log::info!("Engine stopped");
        }

        let snapshot = self.publish(EngineStatus::Stopped);

        match error {
            Some(error) => Err(error),
            None => Ok(snapshot),
        }
    }

    fn current_snapshot(&self) -> EngineSnapshot {
        *self.snapshot.borrow()
    }

    fn publish(&mut self, status: EngineStatus) -> EngineSnapshot {
        let current = self.current_snapshot();

        if current.status == status {
            return current;
        }

        let next = EngineSnapshot {
            status,
            revision: current
                .revision
                .checked_add(1)
                .expect("Engine state revision overflow"),
        };

        self.snapshot.send_replace(next);

        // A single controller preserves publication order.
        // Notification failure does not cancel the engine operation.
        if let Err(error) = self.app.emit(ENGINE_STATUS_CHANGED, next) {
            log::warn!("Failed to emit engine status: {error}");
        }

        next
    }
}