use anyhow::{Result, ensure};
use bytes::Bytes;

use super::super::{
    KnownPacket, PackedTime, WorldOpcodes, WorldPacket,
};

pub(crate) fn decode(
    packet: &WorldPacket,
    opcodes: &WorldOpcodes,
) -> Result<KnownPacket> {
    let data = &packet.payload;
    let opcode = Some(packet.opcode);

    if opcode == opcodes.time.login {
        ensure!(
            data.len() == 20,
            "Invalid LOGIN_SET_TIME_SPEED size: {}",
            data.len(),
        );

        return Ok(KnownPacket::LoginTime {
            server: u32::from_le_bytes(data[0..4].try_into()?).into(),
            game: u32::from_le_bytes(data[4..8].try_into()?).into(),
            speed: f32::from_le_bytes(data[8..12].try_into()?),
            server_holiday_offset: i32::from_le_bytes(
                data[12..16].try_into()?,
            ),
            game_holiday_offset: i32::from_le_bytes(
                data[16..20].try_into()?,
            ),
        });
    }

    if opcode == opcodes.time.set || opcode == opcodes.time.update {
        ensure!(
            data.len() == 16,
            "Invalid GAME_TIME size: {}",
            data.len(),
        );

        return Ok(KnownPacket::GameTime {
            server: u32::from_le_bytes(data[0..4].try_into()?).into(),
            game: u32::from_le_bytes(data[4..8].try_into()?).into(),
            server_holiday_offset: i32::from_le_bytes(
                data[8..12].try_into()?,
            ),
            game_holiday_offset: i32::from_le_bytes(
                data[12..16].try_into()?,
            ),
        });
    }

    if opcode == opcodes.notification {
        ensure!(data.len() >= 2, "Short notification");

        let length = usize::from(
            u16::from_be_bytes(data[..2].try_into()?) >> 4,
        );

        ensure!(
            data.len() == length + 2,
            "Invalid notification length",
        );

        return Ok(KnownPacket::Notification(
            String::from_utf8_lossy(&data[2..]).into_owned(),
        ));
    }

    Ok(KnownPacket::Unknown)
}

pub(crate) fn set_time(
    packet: &mut WorldPacket,
    opcodes: &WorldOpcodes,
    server: PackedTime,
    game: PackedTime,
    speed: Option<f32>,
) -> Result<()> {
    let opcode = Some(packet.opcode);
    let login = opcode == opcodes.time.login;

    let expected = if login {
        20
    } else if opcode == opcodes.time.set || opcode == opcodes.time.update {
        16
    } else {
        anyhow::bail!("Packet does not contain World time fields");
    };

    ensure!(
        packet.payload.len() == expected,
        "Invalid World time payload size: {}",
        packet.payload.len(),
    );

    if let Some(speed) = speed {
        ensure!(
            login,
            "This World time packet has no speed field",
        );

        ensure!(
            speed.is_finite() && speed >= 0.0,
            "Invalid World time speed",
        );
    }

    let mut payload = packet.payload.to_vec();

    payload[0..4].copy_from_slice(&server.raw.to_le_bytes());
    payload[4..8].copy_from_slice(&game.raw.to_le_bytes());

    if let Some(speed) = speed {
        payload[8..12].copy_from_slice(&speed.to_le_bytes());
    }

    packet.payload = Bytes::from(payload);

    Ok(())
}
