use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, Serialize,)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClientDescriptor {
    pub id: ClientId,
    pub label: &'static str,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Clone, Copy, Serialize,)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    pub time_override: Option<TimeOverrideCapabilities>,
}

#[derive(Debug, Clone, Copy, Serialize,)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TimeOverrideCapabilities {
    /// Setting the time upon entering the game world.
    pub entry_time: bool,

    /// Reset to a specified time during server synchronization.
    pub reset_on_server_sync: bool,

    /// Adjusting the duration of an in-game day.
    pub day_length: bool,
}

impl ClientId {
    pub fn capabilities(self) -> ClientCapabilities {
        match self {
            Self::Forever | Self::Retail => ClientCapabilities {
                time_override: Some(TimeOverrideCapabilities {
                    entry_time: true,
                    reset_on_server_sync: true,
                    day_length: true,
                }),
            },
        }
    }

    pub fn descriptor(self) -> ClientDescriptor {
        let label = match self {
            Self::Forever => "WoW Forever",
            Self::Retail => "WoW Retail",
        };

        ClientDescriptor {
            id: self,
            label,
            capabilities: self.capabilities(),
        }
    }
}

pub fn available_clients() -> [ClientDescriptor; 2] {
    [
        ClientId::Forever.descriptor(),
        ClientId::Retail.descriptor(),
    ]
}
