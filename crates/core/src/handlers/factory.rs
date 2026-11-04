use std::sync::Arc;

use anyhow::{Result, ensure};

use crate::features::{FeatureConfig, WorldPipeline};
use crate::lifecycle::Lifecycle;
use crate::protocol::bnet::BnetContext;
use crate::sessions::SessionStore;

use super::bnet::BnetHandler;
use super::passthrough::PassthroughHandler;
use super::world::WorldHandler;
use super::{Handler, HandlerKind, Transport};

pub(crate) struct HandlerFactory {
    bnet: BnetContext,
    sessions: Arc<SessionStore>,
    features: FeatureConfig,
    lifecycle: Arc<Lifecycle>,
}

impl HandlerFactory {
    pub(crate) fn new(
        sessions: Arc<SessionStore>,
        features: FeatureConfig,
        lifecycle: Arc<Lifecycle>,
    ) -> Self {
        Self {
            bnet: BnetContext::new(sessions.clone()),
            sessions,
            features,
            lifecycle,
        }
    }

    pub(crate) fn create(
        &self,
        kind: HandlerKind,
        transport: Transport,
        connection_id: u64,
    ) -> Result<Box<dyn Handler>> {
        ensure!(
            kind == HandlerKind::Passthrough || transport == Transport::Tcp,
            "{kind:?} handler requires TCP",
        );

        match kind {
            HandlerKind::Passthrough => Ok(Box::new(PassthroughHandler)),
            HandlerKind::Bnet => Ok(Box::new(BnetHandler::new(
                connection_id,
                self.bnet.tracker(connection_id),
            ))),
            HandlerKind::World => {
                let pipeline = WorldPipeline::new(&self.features)?;
                let lifecycle = self.lifecycle.register_world(connection_id);

                Ok(Box::new(WorldHandler::new(
                    connection_id,
                    self.sessions.clone(),
                    pipeline,
                    lifecycle,
                )))
            }
        }
    }
}
