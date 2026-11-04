use std::{sync::Mutex, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as AsyncMutex;

const ENGINE_STATUS_CHANGED: &str = "engine-status-changed";
const MOCK_TRANSITION_DELAY: Duration = Duration::from_secs(1);

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

pub struct EngineState {
    snapshot: Mutex<EngineSnapshot>,
    operation: AsyncMutex<()>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            snapshot: Mutex::new(EngineSnapshot {
                status: EngineStatus::Stopped,
                revision: 0,
            }),
            operation: AsyncMutex::new(()),
        }
    }
}

impl EngineState {
    pub fn snapshot(&self) -> Result<EngineSnapshot, String> {
        self.snapshot
            .lock()
            .map(|snapshot| *snapshot)
            .map_err(|_| "Engine state lock is poisoned".to_owned())
    }

    async fn start_inner(&self, app: &AppHandle) -> Result<EngineSnapshot, String> {
        self.transition(app, &[EngineStatus::Stopped], EngineStatus::Starting)?;

        tokio::time::sleep(MOCK_TRANSITION_DELAY).await;

        // Temporary: simulate an actual parsing failure during startup.
        // let initialization = serde_json::from_str::<serde_json::Value>(
        //     r#"{"sessionToken": }"#,
        // );

        // if let Err(error) = initialization {
        //     self.transition(
        //         app,
        //         &[EngineStatus::Starting],
        //         EngineStatus::Stopped,
        //     )?;
        //
        //     return Err(format!("Failed to initialize engine: {error}"));
        // }

        self.transition(app, &[EngineStatus::Starting], EngineStatus::WaitingForGame)
    }

    async fn stop_inner(&self, app: &AppHandle) -> Result<EngineSnapshot, String> {
        self.transition(
            app,
            &[
                EngineStatus::WaitingForGame,
                EngineStatus::WaitingForKeys,
                EngineStatus::Running,
            ],
            EngineStatus::Stopping,
        )?;

        // Replace this delay with the real engine shutdown.
        tokio::time::sleep(MOCK_TRANSITION_DELAY).await;

        self.transition(app, &[EngineStatus::Stopping], EngineStatus::Stopped)
    }

    fn transition(
        &self,
        app: &AppHandle,
        allowed: &[EngineStatus],
        next_status: EngineStatus,
    ) -> Result<EngineSnapshot, String> {
        let mut snapshot = self
            .snapshot
            .lock()
            .map_err(|_| "Engine state lock is poisoned".to_owned())?;

        if !allowed.contains(&snapshot.status) {
            return Err(format!(
                "Cannot transition engine from {:?} to {:?}",
                snapshot.status, next_status,
            ));
        }

        let revision = snapshot
            .revision
            .checked_add(1)
            .ok_or_else(|| "Engine state revision overflow".to_owned())?;

        let next = EngineSnapshot {
            status: next_status,
            revision,
        };

        *snapshot = next;

        // Keep publication ordered with the state transitions.
        // Notification failure must not interrupt the operation.
        if let Err(error) = app.emit(ENGINE_STATUS_CHANGED, next) {
            eprintln!("Failed to emit engine status: {error}");
        }

        Ok(next)
    }

    pub async fn start(&self, app: &AppHandle) -> Result<EngineSnapshot, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "Engine operation is already in progress".to_owned())?;

        self.start_inner(app).await
    }

    pub async fn stop(&self, app: &AppHandle) -> Result<EngineSnapshot, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "Engine operation is already in progress".to_owned())?;

        self.stop_inner(app).await
    }

    pub async fn restart(&self, app: &AppHandle) -> Result<EngineSnapshot, String> {
        let _operation = self
            .operation
            .try_lock()
            .map_err(|_| "Engine operation is already in progress".to_owned())?;

        if self.snapshot()?.status != EngineStatus::Stopped {
            self.stop_inner(app).await?;
        }

        self.start_inner(app).await
    }
}
