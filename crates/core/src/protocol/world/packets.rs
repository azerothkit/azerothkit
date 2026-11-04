use anyhow::{Result, ensure};
use bytes::{BufMut, Bytes, BytesMut};

use super::{MAX_BODY_SIZE, WorldProtocol};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldPacket {
    pub opcode: u32,
    pub payload: Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedTime {
    pub raw: u32,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl From<u32> for PackedTime {
    fn from(raw: u32) -> Self {
        Self {
            raw,
            year: 2000 + ((raw >> 24) & 31) as u16,
            month: 1 + ((raw >> 20) & 15) as u8,
            day: 1 + ((raw >> 14) & 63) as u8,
            hour: ((raw >> 6) & 31) as u8,
            minute: (raw & 63) as u8,
        }
    }
}

impl PackedTime {
    pub fn with_clock(self, hour: u8, minute: u8) -> Result<Self> {
        ensure!(hour < 24, "Hour must be between 0 and 23");
        ensure!(minute < 60, "Minute must be between 0 and 59");

        let raw =
            (self.raw & !0x7ff)
                | (u32::from(hour) << 6)
                | u32::from(minute);

        Ok(raw.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KnownPacket {
    LoginTime {
        server: PackedTime,
        game: PackedTime,
        speed: f32,
        server_holiday_offset: i32,
        game_holiday_offset: i32,
    },
    GameTime {
        server: PackedTime,
        game: PackedTime,
        server_holiday_offset: i32,
        game_holiday_offset: i32,
    },
    Notification(String),
    Unknown,
}

impl WorldPacket {
    pub fn from_body(body: Bytes) -> Result<Self> {
        ensure!(body.len() >= 4, "Short World packet body");

        Ok(Self {
            opcode: u32::from_le_bytes(body[..4].try_into()?),
            payload: body.slice(4..),
        })
    }

    pub fn to_body(&self) -> Result<Bytes> {
        ensure!(
            self.payload.len() <= MAX_BODY_SIZE - 4,
            "World payload exceeds limit",
        );

        let mut body = BytesMut::with_capacity(4 + self.payload.len());

        body.put_u32_le(self.opcode);
        body.extend_from_slice(&self.payload);

        Ok(body.freeze())
    }

    pub fn decode_known(
        &self,
        protocol: &WorldProtocol,
    ) -> Result<KnownPacket> {
        protocol.messages.decode(self, &protocol.opcodes)
    }

    pub fn set_time_fields(
        &mut self,
        protocol: &WorldProtocol,
        server: PackedTime,
        game: PackedTime,
        speed: Option<f32>,
    ) -> Result<()> {
        protocol.messages.set_time(
            self,
            &protocol.opcodes,
            server,
            game,
            speed,
        )
    }
}

pub(super) fn split_messages(
    packet: WorldPacket,
    multiple: u32,
) -> Result<Vec<WorldPacket>> {
    let mut output = Vec::new();

    split(packet, multiple, 0, &mut output)?;

    Ok(output)
}

fn split(
    packet: WorldPacket,
    multiple: u32,
    depth: usize,
    output: &mut Vec<WorldPacket>,
) -> Result<()> {
    ensure!(depth <= 8, "Too many nested World MULTIPLE packets");

    if packet.opcode != multiple {
        ensure!(
            output.len() < 65_536,
            "Too many messages in a World frame",
        );

        output.push(packet);

        return Ok(());
    }

    let mut offset = 0usize;

    while offset < packet.payload.len() {
        ensure!(
            packet.payload.len() - offset >= 6,
            "Incomplete World MULTIPLE header",
        );

        let size = usize::from(u16::from_le_bytes(
            packet.payload[offset..offset + 2].try_into()?,
        ));

        let opcode = u32::from_le_bytes(
            packet.payload[offset + 2..offset + 6].try_into()?,
        );

        offset += 6;

        ensure!(
            size <= packet.payload.len() - offset,
            "Incomplete World MULTIPLE body",
        );

        split(
            WorldPacket {
                opcode,
                payload: packet.payload.slice(offset..offset + size),
            },
            multiple,
            depth + 1,
            output,
        )?;

        offset += size;
    }

    Ok(())
}

pub(super) fn bundle_messages(
    packets: &[WorldPacket],
    force_multiple: bool,
    multiple: u32,
) -> Result<Bytes> {
    ensure!(
        !packets.is_empty(),
        "World frame cannot contain zero messages",
    );

    if packets.len() == 1 && !force_multiple {
        return packets[0].to_body();
    }

    let mut output = BytesMut::new();

    output.put_u32_le(multiple);

    for packet in packets {
        let size = u16::try_from(packet.payload.len())?;

        ensure!(
            output.len() + 6 + usize::from(size) <= MAX_BODY_SIZE,
            "World MULTIPLE body exceeds limit",
        );

        output.put_u16_le(size);
        output.put_u32_le(packet.opcode);
        output.extend_from_slice(&packet.payload);
    }

    Ok(output.freeze())
}
