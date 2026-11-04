use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnginePhase {
    WaitingForGame,
    WaitingForKeys,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldPhase {
    WaitingForKeys,
    Running,
}

pub(crate) struct Lifecycle {
    connections: Mutex<BTreeMap<u64, WorldPhase>>,
    phase: watch::Sender<EnginePhase>,
}

impl Lifecycle {
    pub(crate) fn new() -> (Arc<Self>, watch::Receiver<EnginePhase>) {
        let (phase, receiver) = watch::channel(EnginePhase::WaitingForGame);

        (
            Arc::new(Self {
                connections: Mutex::new(BTreeMap::new()),
                phase,
            }),
            receiver,
        )
    }

    pub(crate) fn register_world(
        self: &Arc<Self>,
        connection_id: u64,
    ) -> WorldLifecycle {
        self.update(|connections| {
            connections.insert(connection_id, WorldPhase::WaitingForKeys);
        });

        WorldLifecycle {
            lifecycle: Arc::clone(self),
            connection_id,
        }
    }

    fn update(
        &self,
        change: impl FnOnce(&mut BTreeMap<u64, WorldPhase>),
    ) {
        // Lifecycle observation must not interrupt traffic.
        let mut connections = self
            .connections
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        change(&mut connections);

        let next = if connections
            .values()
            .any(|phase| *phase == WorldPhase::Running)
        {
            EnginePhase::Running
        } else if !connections.is_empty() {
            EnginePhase::WaitingForKeys
        } else {
            EnginePhase::WaitingForGame
        };

        // Publish under the same lock to preserve update order.
        self.phase.send_if_modified(|current| {
            if *current == next {
                return false;
            }

            *current = next;
            true
        });
    }
}

pub(crate) struct WorldLifecycle {
    lifecycle: Arc<Lifecycle>,
    connection_id: u64,
}

impl WorldLifecycle {
    pub(crate) fn waiting_for_keys(&self) {
        self.set_phase(WorldPhase::WaitingForKeys);
    }

    pub(crate) fn running(&self) {
        self.set_phase(WorldPhase::Running);
    }

    fn set_phase(&self, phase: WorldPhase) {
        self.lifecycle.update(|connections| {
            if let Some(current) = connections.get_mut(&self.connection_id) {
                *current = phase;
            }
        });
    }
}

impl Drop for WorldLifecycle {
    fn drop(&mut self) {
        self.lifecycle.update(|connections| {
            connections.remove(&self.connection_id);
        });
    }
}