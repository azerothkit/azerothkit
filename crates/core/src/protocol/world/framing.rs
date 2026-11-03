use anyhow::{Result, ensure};
use bytes::{BufMut, Bytes, BytesMut};

use super::MAX_BODY_SIZE;

const CLIENT_GREETING: &[u8] =
    b"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n";

const SERVER_GREETING: &[u8] =
    b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n";

pub(super) struct WireFrame {
    pub counter: u64,
    pub tag: [u8; 12],
    pub body: Bytes,
    pub wire: Bytes,
}

pub(super) enum WireMessage {
    Greeting(Bytes),
    Frame(WireFrame),
}

pub(super) struct FrameBuffer {
    buffer: BytesMut,
    greeting: &'static [u8],
    greeted: bool,
    counter: u64,
}

impl FrameBuffer {
    pub fn new(from_client: bool) -> Self {
        Self {
            buffer: BytesMut::new(),
            greeting: if from_client {
                CLIENT_GREETING
            } else {
                SERVER_GREETING
            },
            greeted: false,
            counter: 0,
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn next(&mut self) -> Result<Option<WireMessage>> {
        if !self.greeted {
            let compared = self.buffer.len().min(self.greeting.len());

            ensure!(
                self.buffer[..compared] == self.greeting[..compared],
                "Invalid World V2 greeting",
            );

            if self.buffer.len() < self.greeting.len() {
                return Ok(None);
            }

            let greeting =
                self.buffer.split_to(self.greeting.len()).freeze();

            self.greeted = true;

            return Ok(Some(WireMessage::Greeting(greeting)));
        }

        if self.buffer.len() < 4 {
            return Ok(None);
        }

        let size = u32::from_le_bytes(
            self.buffer[..4].try_into()?,
        ) as usize;

        ensure!(
            (4..=MAX_BODY_SIZE).contains(&size),
            "Invalid World frame size: {size}",
        );

        let total = 16 + size;

        if self.buffer.len() < total {
            return Ok(None);
        }

        let counter = self.counter;

        self.counter = self
            .counter
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("World counter exhausted"))?;

        let wire = self.buffer.split_to(total).freeze();
        let tag = wire[4..16].try_into()?;
        let body = wire.slice(16..);

        Ok(Some(WireMessage::Frame(WireFrame {
            counter,
            tag,
            body,
            wire,
        })))
    }

    pub fn finish(&self) -> Result<()> {
        ensure!(
            self.buffer.is_empty(),
            "World stream ended with {} buffered bytes",
            self.buffer.len(),
        );

        Ok(())
    }
}

pub(super) fn encode_frame(
    tag: &[u8; 12],
    body: &[u8],
) -> Result<Bytes> {
    ensure!(
        (4..=MAX_BODY_SIZE).contains(&body.len()),
        "Invalid outgoing World body size: {}",
        body.len(),
    );

    let mut output = BytesMut::with_capacity(16 + body.len());

    output.put_u32_le(body.len() as u32);
    output.extend_from_slice(tag);
    output.extend_from_slice(body);

    Ok(output.freeze())
}
