use super::{
    ClientCapabilities, ClientProfile, TimeOverrideCapabilities,
};
use crate::protocol::world::{
    MessageCodec, TimeOpcodes, WorldOpcodes, WorldProtocol,
};

pub(super) const PROFILE: ClientProfile = ClientProfile {
    label: "WoW Retail",
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

    world: Some(WorldProtocol {
        opcodes: WorldOpcodes {
            auth_session: 0x440001,
            auth_continued: 0x440003,
            encrypted_ack: 0x440005,

            auth_challenge: 0x4c0000,
            enter_encrypted: 0x4c0004,
            multiple: 0x4c000d,
            compressed: 0x4c000e,

            time: TimeOpcodes {
                login: Some(0x4501b8),
                set: Some(0x4501b7),
                update: Some(0x4501b6),
            },

            notification: Some(0x45006e),
        },

        messages: MessageCodec::Modern,
    }),
};
