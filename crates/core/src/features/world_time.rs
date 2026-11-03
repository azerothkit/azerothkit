use anyhow::Result;

use crate::handlers::Direction;
use crate::protocol::world::{
    GAME_TIME_SET, GAME_TIME_UPDATE, KnownPacket, LOGIN_TIME, WorldPacket,
};

use super::WorldTimeConfig;
use super::pipeline::WorldFeature;

const OPCODES: &[u32] = &[LOGIN_TIME, GAME_TIME_SET, GAME_TIME_UPDATE];

pub(super) struct WorldTimeFeature {
    config: WorldTimeConfig,
    speed: Option<f32>,
}

impl WorldTimeFeature {
    pub(super) fn new(config: WorldTimeConfig) -> Result<Self> {
        config.validate()?;
        let speed = config.packet_speed()?;

        Ok(Self { config, speed })
    }
}

impl WorldFeature for WorldTimeFeature {
    fn direction(&self) -> Direction {
        Direction::ServerToClient
    }

    fn opcodes(&self) -> &'static [u32] {
        OPCODES
    }

    fn apply(&mut self, packet: &mut WorldPacket) -> Result<()> {
        let login = packet.opcode == LOGIN_TIME;

        let time = self
            .config
            .start_time
            .filter(|_| login || self.config.pin_on_sync);

        let speed = if login { self.speed } else { None };

        if time.is_none() && speed.is_none() {
            return Ok(());
        }

        let (mut server, mut game) = match packet.decode_known()? {
            KnownPacket::LoginTime { server, game, .. }
            | KnownPacket::GameTime { server, game, .. } => (server, game),
            _ => return Ok(()),
        };

        if let Some(time) = time {
            server = server.with_clock(time.hour(), time.minute())?;
            game = game.with_clock(time.hour(), time.minute())?;
        }

        packet.set_time_fields(server, game, speed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::TimeOfDay;

    fn packet(opcode: u32) -> WorldPacket {
        let mut payload = Vec::new();

        payload.extend_from_slice(&0x1aab_c123u32.to_le_bytes());
        payload.extend_from_slice(&0x1aab_c456u32.to_le_bytes());

        if opcode == LOGIN_TIME {
            payload.extend_from_slice(&(1.0f32 / 60.0).to_le_bytes());
        }

        payload.extend_from_slice(&123i32.to_le_bytes());
        payload.extend_from_slice(&(-456i32).to_le_bytes());

        WorldPacket {
            opcode,
            payload: payload.into(),
        }
    }

    #[test]
    fn converts_day_duration_to_packet_speed() {
        for (minutes, expected) in [(1440.0, 1.0f32 / 60.0), (60.0, 0.4), (30.0, 0.8)] {
            let config = WorldTimeConfig {
                day_minutes: Some(minutes),
                ..Default::default()
            };

            assert!((config.packet_speed().unwrap().unwrap() - expected).abs() < 0.000001,);
        }
    }

    #[test]
    fn changes_login_clock_and_speed_preserving_other_fields() {
        let mut feature = WorldTimeFeature::new(WorldTimeConfig {
            day_minutes: Some(60.0),
            start_time: Some(TimeOfDay::new(18, 30).unwrap()),
            pin_on_sync: false,
        })
        .unwrap();

        let mut message = packet(LOGIN_TIME);
        let original = message.payload.clone();

        feature.apply(&mut message).unwrap();

        match message.decode_known().unwrap() {
            KnownPacket::LoginTime {
                server,
                game,
                speed,
                server_holiday_offset,
                game_holiday_offset,
            } => {
                assert_eq!((server.hour, server.minute), (18, 30));
                assert_eq!((game.hour, game.minute), (18, 30));
                assert_eq!(server.raw & !0x7ff, 0x1aab_c123 & !0x7ff);
                assert_eq!(game.raw & !0x7ff, 0x1aab_c456 & !0x7ff);
                assert!((speed - 0.4).abs() < 0.000001);
                assert_eq!(server_holiday_offset, 123);
                assert_eq!(game_holiday_offset, -456);
                assert_eq!(&message.payload[12..], &original[12..]);
            }
            _ => panic!("expected login time"),
        }
    }

    #[test]
    fn pinning_controls_subsequent_updates() {
        for pin_on_sync in [false, true] {
            let mut feature = WorldTimeFeature::new(WorldTimeConfig {
                start_time: Some(TimeOfDay::new(9, 15).unwrap()),
                pin_on_sync,
                ..Default::default()
            })
            .unwrap();

            let mut message = packet(GAME_TIME_UPDATE);
            let original = message.clone();

            feature.apply(&mut message).unwrap();

            if pin_on_sync {
                match message.decode_known().unwrap() {
                    KnownPacket::GameTime { server, game, .. } => {
                        assert_eq!((server.hour, server.minute), (9, 15));
                        assert_eq!((game.hour, game.minute), (9, 15));
                    }
                    _ => panic!("expected game time"),
                }
            } else {
                assert_eq!(message, original);
            }
        }
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert!(
            WorldTimeConfig {
                pin_on_sync: true,
                ..Default::default()
            }
            .validate()
            .is_err(),
        );

        for minutes in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                WorldTimeConfig {
                    day_minutes: Some(minutes),
                    ..Default::default()
                }
                .validate()
                .is_err(),
            );
        }
    }

    #[test]
    fn pipeline_does_not_modify_client_packets() {
        let config = super::super::FeatureConfig {
            world_time: Some(WorldTimeConfig {
                start_time: Some(TimeOfDay::new(18, 30).unwrap()),
                ..Default::default()
            }),
        };

        let mut pipeline = super::super::WorldPipeline::new(&config).unwrap();

        let message = packet(LOGIN_TIME);

        assert!(
            pipeline
                .process(Direction::ClientToServer, &[message])
                .unwrap()
                .is_none(),
        );
    }
}
