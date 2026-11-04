use anyhow::{Result, ensure};

use crate::clients::ClientId;

pub use crate::handlers::{HandlerKind, Transport};

#[derive(Debug, Clone)]
pub struct HandlerRule {
    pub transport: Transport,
    pub port: u16,
    pub handler: HandlerKind,
}

#[derive(Debug, Clone)]
pub struct HandlerConfig {
    pub rules: Vec<HandlerRule>,
}

impl Default for HandlerConfig {
    fn default() -> Self {
        Self::for_client(ClientId::Retail)
    }
}

impl HandlerConfig {
    pub fn for_client(client: ClientId) -> Self {
        let profile = client.profile();

        let rules = profile.bnet_ports.iter()
            .map(|&port| HandlerRule {
                transport: Transport::Tcp,
                port,
                handler: HandlerKind::Bnet,
            })
            .chain(
                profile.world_ports.iter().map(|&port| HandlerRule {
                    transport: Transport::Tcp,
                    port,
                    handler: HandlerKind::World,
                }),
            )
            .collect();

        Self { rules }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        for rule in &self.rules {
            ensure!(
                rule.handler == HandlerKind::Passthrough
                    || rule.transport == Transport::Tcp,
                "{:?} handler requires TCP: port {}",
                rule.handler,
                rule.port,
            );
        }

        Ok(())
    }

    /// Later rules override earlier rules.
    /// Unmatched connections use passthrough.
    pub(crate) fn select(
        &self,
        transport: Transport,
        port: u16,
    ) -> HandlerKind {
        self.rules.iter()
            .rev()
            .find(|rule| {
                rule.transport == transport && rule.port == port
            })
            .map(|rule| rule.handler)
            .unwrap_or_default()
    }
}
