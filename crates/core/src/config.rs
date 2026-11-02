use anyhow::{Result, ensure};

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
        Self {
            rules: vec![HandlerRule {
                transport: Transport::Tcp,
                port: 1119,
                handler: HandlerKind::Bnet,
            }],
        }
    }
}

impl HandlerConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        for rule in &self.rules {
            ensure!(
                rule.handler != HandlerKind::Bnet || rule.transport == Transport::Tcp,
                "Battle.net handler requires TCP: port {}",
                rule.port,
            );
        }

        Ok(())
    }

    /// Later rules override earlier rules.
    /// Unmatched connections use passthrough.
    pub(crate) fn select(&self, transport: Transport, port: u16) -> HandlerKind {
        self.rules
            .iter()
            .rev()
            .find(|rule| rule.transport == transport && rule.port == port)
            .map(|rule| rule.handler)
            .unwrap_or_default()
    }
}
