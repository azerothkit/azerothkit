mod forever;
mod retail;

use serde::{Deserialize, Serialize};

use crate::protocol::world::WorldProtocol;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
    Serialize, Deserialize,
)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ClientId {
    Forever,
    Retail,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClientDescriptor {
    pub id: ClientId,
    pub label: &'static str,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    pub time_override: Option<TimeOverrideCapabilities>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TimeOverrideCapabilities {
    pub entry_time: bool,
    pub reset_on_server_sync: bool,
    pub day_length: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ClientProfile {
    pub label: &'static str,
    pub bnet_ports: &'static [u16],
    pub world_ports: &'static [u16],
    pub tls_ports: &'static [u16],
    pub capabilities: ClientCapabilities,
    pub world: Option<WorldProtocol>,
}

impl ClientId {
    pub fn profile(self) -> &'static ClientProfile {
        match self {
            Self::Forever => &forever::PROFILE,
            Self::Retail => &retail::PROFILE,
        }
    }

    pub fn capabilities(self) -> ClientCapabilities {
        self.profile().capabilities
    }

    pub fn descriptor(self) -> ClientDescriptor {
        let profile = self.profile();

        ClientDescriptor {
            id: self,
            label: profile.label,
            capabilities: profile.capabilities,
        }
    }
}

pub fn available_clients() -> [ClientDescriptor; 2] {
    [
        ClientId::Forever.descriptor(),
        ClientId::Retail.descriptor(),
    ]
}
