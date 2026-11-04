use anyhow::{Result, ensure};

use super::{KnownPacket, PackedTime, WorldPacket};

#[derive(Debug, Clone, Copy, Default)]
pub struct TimeOpcodes {
    pub login: Option<u32>,
    pub set: Option<u32>,
    pub update: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct WorldOpcodes {
    pub auth_session: u32,
    pub auth_continued: u32,
    pub encrypted_ack: u32,

    pub auth_challenge: u32,
    pub enter_encrypted: u32,
    pub multiple: u32,
    pub compressed: u32,

    pub time: TimeOpcodes,
    pub notification: Option<u32>,
}

impl WorldOpcodes {
    pub fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();

        // Opcode namespaces are independent in each direction.
        for opcodes in [
            vec![
                self.auth_session,
                self.auth_continued,
                self.encrypted_ack,
            ],
            [
                Some(self.auth_challenge),
                Some(self.enter_encrypted),
                Some(self.multiple),
                Some(self.compressed),
                self.time.login,
                self.time.set,
                self.time.update,
                self.notification,
            ]
                .into_iter()
                .flatten()
                .collect(),
        ] {
            seen.clear();

            for opcode in opcodes {
                ensure!(
                    seen.insert(opcode),
                    "Duplicate World opcode: {opcode:#x}",
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MessageCodec {
    Modern,
}

impl MessageCodec {
    pub(super) fn decode(
        self,
        packet: &WorldPacket,
        opcodes: &WorldOpcodes,
    ) -> Result<KnownPacket> {
        match self {
            Self::Modern => super::messages::modern::decode(packet, opcodes),
        }
    }

    pub(super) fn set_time(
        self,
        packet: &mut WorldPacket,
        opcodes: &WorldOpcodes,
        server: PackedTime,
        game: PackedTime,
        speed: Option<f32>,
    ) -> Result<()> {
        match self {
            Self::Modern => super::messages::modern::set_time(
                packet,
                opcodes,
                server,
                game,
                speed,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorldProtocol {
    pub opcodes: WorldOpcodes,
    pub messages: MessageCodec,
}
