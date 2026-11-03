use anyhow::{Result, ensure};
use bytes::{BufMut, Bytes, BytesMut};

use super::MAX_BODY_SIZE;

pub const AUTH_SESSION: u32 = 0x440001;
pub const AUTH_CONTINUED: u32 = 0x440003;
pub const ENCRYPTED_ACK: u32 = 0x440005;

pub const AUTH_CHALLENGE: u32 = 0x4c0000;
pub const ENTER_ENCRYPTED: u32 = 0x4c0004;
pub const MULTIPLE: u32 = 0x4c000d;
pub const COMPRESSED: u32 = 0x4c000e;

pub const LOGIN_TIME: u32 = 0x4501b8;
pub const GAME_TIME_SET: u32 = 0x4501b7;
pub const GAME_TIME_UPDATE: u32 = 0x4501b6;
pub const PRINT_NOTIFICATION: u32 = 0x45006e;

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

impl PackedTime {
    pub fn with_clock(self, hour: u8, minute: u8) -> Result<Self> {
        ensure!(hour < 24, "Hour must be between 0 and 23");
        ensure!(minute < 60, "Minute must be between 0 and 59");

        let raw = (self.raw & !0x7ff) | (u32::from(hour) << 6) | u32::from(minute);

        Ok(raw.into())
    }
}

impl WorldPacket {
    pub fn set_time_fields(
        &mut self,
        server: PackedTime,
        game: PackedTime,
        speed: Option<f32>,
    ) -> Result<()> {
        let expected = match self.opcode {
            LOGIN_TIME => 20,
            GAME_TIME_SET | GAME_TIME_UPDATE => 16,
            _ => anyhow::bail!("Packet does not contain World time fields"),
        };

        ensure!(
            self.payload.len() == expected,
            "Invalid World time payload size: {}",
            self.payload.len(),
        );

        if let Some(speed) = speed {
            ensure!(
                self.opcode == LOGIN_TIME,
                "This World time packet has no speed field",
            );

            ensure!(
                speed.is_finite() && speed >= 0.0,
                "Invalid World time speed",
            );
        }

        let mut payload = self.payload.to_vec();

        payload[0..4].copy_from_slice(&server.raw.to_le_bytes());
        payload[4..8].copy_from_slice(&game.raw.to_le_bytes());

        if let Some(speed) = speed {
            payload[8..12].copy_from_slice(&speed.to_le_bytes());
        }

        self.payload = Bytes::from(payload);
        Ok(())
    }
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

    pub fn decode_known(&self) -> Result<KnownPacket> {
        let data = &self.payload;

        match self.opcode {
            LOGIN_TIME => {
                ensure!(
                    data.len() == 20,
                    "Invalid LOGIN_SET_TIME_SPEED size: {}",
                    data.len(),
                );

                Ok(KnownPacket::LoginTime {
                    server: u32::from_le_bytes(data[0..4].try_into()?).into(),
                    game: u32::from_le_bytes(data[4..8].try_into()?).into(),
                    speed: f32::from_le_bytes(data[8..12].try_into()?),
                    server_holiday_offset: i32::from_le_bytes(data[12..16].try_into()?),
                    game_holiday_offset: i32::from_le_bytes(data[16..20].try_into()?),
                })
            }

            GAME_TIME_SET | GAME_TIME_UPDATE => {
                ensure!(data.len() == 16, "Invalid GAME_TIME size: {}", data.len(),);

                Ok(KnownPacket::GameTime {
                    server: u32::from_le_bytes(data[0..4].try_into()?).into(),
                    game: u32::from_le_bytes(data[4..8].try_into()?).into(),
                    server_holiday_offset: i32::from_le_bytes(data[8..12].try_into()?),
                    game_holiday_offset: i32::from_le_bytes(data[12..16].try_into()?),
                })
            }

            PRINT_NOTIFICATION => {
                ensure!(data.len() >= 2, "Short notification");

                let length = usize::from(u16::from_be_bytes(data[..2].try_into()?) >> 4);

                ensure!(data.len() == length + 2, "Invalid notification length",);

                Ok(KnownPacket::Notification(
                    String::from_utf8_lossy(&data[2..]).into_owned(),
                ))
            }

            _ => Ok(KnownPacket::Unknown),
        }
    }
}

pub(super) fn split_messages(packet: WorldPacket) -> Result<Vec<WorldPacket>> {
    let mut output = Vec::new();
    split(packet, 0, &mut output)?;
    Ok(output)
}

fn split(packet: WorldPacket, depth: usize, output: &mut Vec<WorldPacket>) -> Result<()> {
    ensure!(depth <= 8, "Too many nested World MULTIPLE packets");

    if packet.opcode != MULTIPLE {
        ensure!(output.len() < 65_536, "Too many messages in a World frame",);

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

        let opcode = u32::from_le_bytes(packet.payload[offset + 2..offset + 6].try_into()?);

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
            depth + 1,
            output,
        )?;

        offset += size;
    }

    Ok(())
}

pub(super) fn bundle_messages(packets: &[WorldPacket], force_multiple: bool) -> Result<Bytes> {
    ensure!(
        !packets.is_empty(),
        "World frame cannot contain zero messages"
    );

    if packets.len() == 1 && !force_multiple {
        return packets[0].to_body();
    }

    let mut output = BytesMut::new();
    output.put_u32_le(MULTIPLE);

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
