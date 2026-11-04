use std::sync::Arc;

use anyhow::{Context, Result, ensure};

use crate::clients::ClientId;
use crate::features::{FeatureConfig, WorldPipeline};
use crate::lifecycle::Lifecycle;
use crate::protocol::bnet::BnetContext;
use crate::protocol::world::WorldCodec;
use crate::sessions::SessionStore;

use super::bnet::BnetHandler;
use super::passthrough::PassthroughHandler;
use super::world::WorldHandler;
use super::{Handler, HandlerKind, Transport};

pub(crate) struct HandlerFactory {
    client: ClientId,
    bnet: BnetContext,
    sessions: Arc<SessionStore>,
    features: FeatureConfig,
    lifecycle: Arc<Lifecycle>,
}

impl HandlerFactory {
    pub(crate) fn new(
        client: ClientId,
        sessions: Arc<SessionStore>,
        features: FeatureConfig,
        lifecycle: Arc<Lifecycle>,
    ) -> Result<Self> {
        // Validate feature routes before accepting connections.
        if let Some(protocol) = client.profile().world {
            WorldPipeline::new(
                &features,
                protocol,
                client.capabilities().time_override,
            )?;
        }

        Ok(Self {
            client,
            bnet: BnetContext::new(sessions.clone()),
            sessions,
            features,
            lifecycle,
        })
    }

    pub(crate) fn create(
        &self,
        kind: HandlerKind,
        transport: Transport,
        connection_id: u64,
    ) -> Result<Box<dyn Handler>> {
        ensure!(
            kind == HandlerKind::Passthrough
                || transport == Transport::Tcp,
            "{kind:?} handler requires TCP",
        );

        match kind {
            HandlerKind::Passthrough => {
                Ok(Box::new(PassthroughHandler))
            }

            HandlerKind::Bnet => Ok(Box::new(BnetHandler::new(
                connection_id,
                self.bnet.tracker(connection_id),
            ))),

            HandlerKind::World => {
                let protocol = self.client.profile().world
                    .context("World protocol is not configured")?;

                let pipeline = WorldPipeline::new(
                    &self.features,
                    protocol,
                    self.client.capabilities().time_override,
                )?;

                let lifecycle =
                    self.lifecycle.register_world(connection_id);

                Ok(Box::new(WorldHandler::new(
                    connection_id,
                    self.sessions.clone(),
                    WorldCodec::new(protocol),
                    pipeline,
                    lifecycle,
                )))
            }
        }
    }
}
