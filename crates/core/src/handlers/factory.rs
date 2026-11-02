use std::sync::Arc;

use anyhow::{Result, ensure};

use crate::protocol::bnet::BnetContext;
use crate::session_keys::SessionKeys;

use super::bnet::BnetHandler;
use super::passthrough::PassthroughHandler;
use super::{Handler, HandlerKind, Transport};

pub(crate) struct HandlerFactory {
    bnet: BnetContext,
}

impl HandlerFactory {
    pub(crate) fn new(keys: Arc<SessionKeys>) -> Self {
        Self {
            bnet: BnetContext::new(keys),
        }
    }

    /// The caller has already selected the handler kind from connection configuration.
    pub(crate) fn create(
        &self,
        kind: HandlerKind,
        transport: Transport,
        connection_id: u64,
    ) -> Result<Box<dyn Handler>> {
        match kind {
            HandlerKind::Passthrough => Ok(Box::new(PassthroughHandler)),

            HandlerKind::Bnet => {
                ensure!(transport == Transport::Tcp, "Battle.net requires TCP",);

                Ok(Box::new(BnetHandler::new(
                    connection_id,
                    self.bnet.tracker(connection_id),
                )))
            }
        }
    }
}
