use super::*;

use crate::clients::ClientId;
use crate::features::{
    FeatureConfig, TimeOfDay, WorldPipeline,
};

fn protocol() -> WorldProtocol {
    let mut protocol = ClientId::Retail.profile().world.unwrap();

    protocol.opcodes.time.login = Some(0x101);
    protocol.opcodes.time.set = Some(0x102);
    protocol.opcodes.time.update = Some(0x103);

    protocol
}

fn capabilities() -> TimeOverrideCapabilities {
    ClientId::Retail.capabilities().time_override.unwrap()
}

fn packet(opcode: u32, login: bool) -> WorldPacket {
    let mut payload = Vec::new();

    payload.extend_from_slice(&0x1aab_c123u32.to_le_bytes());
    payload.extend_from_slice(&0x1aab_c456u32.to_le_bytes());

    if login {
        payload.extend_from_slice(
            &(1.0f32 / 60.0).to_le_bytes(),
        );
    }

    payload.extend_from_slice(&123i32.to_le_bytes());
    payload.extend_from_slice(&(-456i32).to_le_bytes());

    WorldPacket {
        opcode,
        payload: payload.into(),
    }
}

#[test]
fn remapped_login_changes_clock_and_speed_preserving_other_fields() {
    let protocol = protocol();

    let mut feature = WorldTimeFeature::new(
        WorldTimeConfig {
            start_time: Some(TimeOfDay::new(18, 30).unwrap()),
            day_minutes: Some(60.0),
            pin_on_sync: false,
        },
        protocol,
        capabilities(),
    )
        .unwrap();

    let mut message = packet(
        protocol.opcodes.time.login.unwrap(),
        true,
    );

    let original = message.payload.clone();

    feature.apply(&mut message).unwrap();

    match message.decode_known(&protocol).unwrap() {
        KnownPacket::LoginTime {
            server,
            game,
            speed,
            ..
        } => {
            assert_eq!((server.hour, server.minute), (18, 30));
            assert_eq!((game.hour, game.minute), (18, 30));
            assert_eq!(
                server.raw & !0x7ff,
                0x1aab_c123 & !0x7ff,
            );
            assert_eq!(
                game.raw & !0x7ff,
                0x1aab_c456 & !0x7ff,
            );
            assert!((speed - 0.4).abs() < 0.000001);
            assert_eq!(&message.payload[12..], &original[12..]);
        }

        _ => panic!("expected login time"),
    }

    let retail = ClientId::Retail.profile().world.unwrap();

    let mut old_message = packet(
        retail.opcodes.time.login.unwrap(),
        true,
    );

    let original = old_message.clone();

    feature.apply(&mut old_message).unwrap();

    assert_eq!(old_message, original);
}

#[test]
fn sync_only_support_does_not_change_login() {
    let protocol = protocol();

    let caps = TimeOverrideCapabilities {
        entry_time: false,
        reset_on_server_sync: true,
        day_length: false,
    };

    let mut feature = WorldTimeFeature::new(
        WorldTimeConfig {
            start_time: Some(TimeOfDay::new(9, 15).unwrap()),
            day_minutes: Some(60.0),
            pin_on_sync: true,
        },
        protocol,
        caps,
    )
        .unwrap();

    let mut login = packet(
        protocol.opcodes.time.login.unwrap(),
        true,
    );

    let original = login.clone();

    feature.apply(&mut login).unwrap();

    assert_eq!(login, original);

    let mut sync = packet(
        protocol.opcodes.time.update.unwrap(),
        false,
    );

    feature.apply(&mut sync).unwrap();

    match sync.decode_known(&protocol).unwrap() {
        KnownPacket::GameTime { game, .. } => {
            assert_eq!((game.hour, game.minute), (9, 15));
        }

        _ => panic!("expected game time"),
    }
}

#[test]
fn unsupported_values_are_removed_without_changing_saved_configuration() {
    let config = WorldTimeConfig {
        start_time: Some(TimeOfDay::new(9, 15).unwrap()),
        day_minutes: Some(f64::NAN),
        pin_on_sync: true,
    };

    let supported = config.supported(
        TimeOverrideCapabilities {
            entry_time: true,
            reset_on_server_sync: false,
            day_length: false,
        },
    )
        .unwrap();

    supported.validate().unwrap();

    assert!(supported.day_minutes.is_none());
    assert!(!supported.pin_on_sync);

    assert!(config.day_minutes.unwrap().is_nan());
    assert!(config.pin_on_sync);
}

#[test]
fn missing_required_opcode_is_rejected_before_processing_packets() {
    let mut protocol = protocol();

    protocol.opcodes.time.login = None;

    assert!(
        WorldTimeFeature::new(
            WorldTimeConfig {
                day_minutes: Some(60.0),
                ..Default::default()
            },
            protocol,
            capabilities(),
        )
        .is_err(),
    );
}

#[test]
fn pipeline_does_not_modify_client_packets() {
    let protocol = protocol();

    let mut pipeline = WorldPipeline::new(
        &FeatureConfig {
            world_time: Some(WorldTimeConfig {
                start_time: Some(
                    TimeOfDay::new(18, 30).unwrap(),
                ),
                ..Default::default()
            }),
        },
        protocol,
        Some(capabilities()),
    )
        .unwrap();

    let message = packet(
        protocol.opcodes.time.login.unwrap(),
        true,
    );

    assert!(
        pipeline.process(
            Direction::ClientToServer,
            &[message],
        )
        .unwrap()
        .is_none(),
    );
}

#[test]
fn validates_duration_and_sync_dependencies() {
    assert!(
        WorldTimeConfig {
            pin_on_sync: true,
            ..Default::default()
        }
        .validate()
        .is_err(),
    );

    for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            WorldTimeConfig {
                day_minutes: Some(value),
                ..Default::default()
            }
            .validate()
            .is_err(),
        );
    }

    for (minutes, expected) in [
        (1440.0, 1.0f32 / 60.0),
        (60.0, 0.4),
        (30.0, 0.8),
    ] {
        let config = WorldTimeConfig {
            day_minutes: Some(minutes),
            ..Default::default()
        };

        assert!(
            (
                config.packet_speed().unwrap().unwrap()
                - expected
            )
            .abs()
                < 0.000001,
        );
    }
}
