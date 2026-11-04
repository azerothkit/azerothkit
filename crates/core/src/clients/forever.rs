use super::{
    ClientCapabilities, ClientProfile, TimeOverrideCapabilities,
};

pub(super) const PROFILE: ClientProfile = ClientProfile {
    label: "WoW Forever",

    // Initial defaults; replace these if Forever uses different ports.
    bnet_ports: &[1119],
    world_ports: &[3724],
    tls_ports: &[443, 1119],

    capabilities: ClientCapabilities {
        time_override: Some(TimeOverrideCapabilities {
            entry_time: true,
            reset_on_server_sync: true,
            day_length: true,
        }),
    },

    // Set Some(WorldProtocol { ... }) after obtaining Forever's opcode table.
    // Use MessageCodec::Modern if its message layouts match Retail.
    world: None,
};
