use anyhow::{Result, ensure};

use crate::clients::TimeOverrideCapabilities;
use crate::handlers::Direction;
use crate::protocol::world::{
    KnownPacket, WorldPacket, WorldProtocol,
};

use super::WorldTimeConfig;
use super::pipeline::WorldFeature;

pub(super) struct WorldTimeFeature {
    config: WorldTimeConfig,
    protocol: WorldProtocol,
    capabilities: TimeOverrideCapabilities,
    opcodes: Vec<u32>,
    speed: Option<f32>,
}

impl WorldTimeFeature {
    pub(super) fn new(
        config: WorldTimeConfig,
        protocol: WorldProtocol,
        capabilities: TimeOverrideCapabilities,
    ) -> Result<Self> {
        let config = config
            .supported(capabilities)
            .unwrap_or_default();

        config.validate()?;

        let speed = config.packet_speed()?;
        let mut opcodes = Vec::new();
        let time = protocol.opcodes.time;

        if speed.is_some()
            || (capabilities.entry_time && config.start_time.is_some())
        {
            let login = time.login.ok_or_else(|| {
                anyhow::anyhow!(
                    "World time requires a login-time opcode",
                )
            })?;

            opcodes.push(login);
        }

        if config.pin_on_sync {
            ensure!(
                time.set.is_some() || time.update.is_some(),
                "World time synchronization requires a set/update opcode",
            );

            opcodes.extend(time.set);
            opcodes.extend(time.update);
        }

        opcodes.sort_unstable();
        opcodes.dedup();

        Ok(Self {
            config,
            protocol,
            capabilities,
            opcodes,
            speed,
        })
    }
}

impl WorldFeature for WorldTimeFeature {
    fn direction(&self) -> Direction {
        Direction::ServerToClient
    }

    fn opcodes(&self) -> &[u32] {
        &self.opcodes
    }

    fn apply(&mut self, packet: &mut WorldPacket) -> Result<()> {
        let opcode = Some(packet.opcode);
        let time_opcodes = self.protocol.opcodes.time;

        let login = opcode == time_opcodes.login;
        let sync =
            opcode == time_opcodes.set
                || opcode == time_opcodes.update;

        if !login && !sync {
            return Ok(());
        }

        let apply_clock = if login {
            self.capabilities.entry_time
        } else {
            self.config.pin_on_sync
        };

        let time = self.config.start_time.filter(|_| apply_clock);
        let speed = if login { self.speed } else { None };

        if time.is_none() && speed.is_none() {
            return Ok(());
        }

        let (mut server, mut game) =
            match packet.decode_known(&self.protocol)? {
                KnownPacket::LoginTime { server, game, .. }
                | KnownPacket::GameTime { server, game, .. } => {
                    (server, game)
                }
                _ => return Ok(()),
            };

        if let Some(time) = time {
            server = server.with_clock(time.hour(), time.minute())?;
            game = game.with_clock(time.hour(), time.minute())?;
        }

        packet.set_time_fields(
            &self.protocol,
            server,
            game,
            speed,
        )
    }
}

#[cfg(test)]
mod tests;
