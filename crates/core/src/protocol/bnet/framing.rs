use anyhow::{Context, Result, ensure};
use bytes::{Bytes, BytesMut};
use prost::Message;

use super::generated::bgs::protocol::Header;

const MAX_BODY_SIZE: usize = 16 * 1024 * 1024;

pub(super) struct Frame {
    pub offset: u64,
    pub header: Header,
    pub body: Bytes,
}

pub(super) struct FrameBuffer {
    buffer: BytesMut,
    offset: u64,
}

impl FrameBuffer {
    pub(super) fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
            offset: 0,
        }
    }

    pub(super) fn feed(&mut self, data: &[u8]) -> Result<Vec<Frame>> {
        self.buffer.extend_from_slice(data);

        let mut frames = Vec::new();

        loop {
            if self.buffer.len() < 2 {
                break;
            }

            let header_size = usize::from(u16::from_be_bytes([self.buffer[0], self.buffer[1]]));

            ensure!(
                header_size != 0,
                "Empty Battle.net header at offset {}",
                self.offset,
            );

            let body_start = 2 + header_size;

            if self.buffer.len() < body_start {
                break;
            }

            let header = Header::decode(&self.buffer[2..body_start])
                .with_context(|| format!("Invalid Battle.net header at offset {}", self.offset,))?;

            let body_size = header.size.unwrap_or_default() as usize;

            ensure!(
                body_size <= MAX_BODY_SIZE,
                "Battle.net body size {body_size} exceeds limit {MAX_BODY_SIZE}",
            );

            let frame_size = body_start + body_size;

            if self.buffer.len() < frame_size {
                break;
            }

            let bytes = self.buffer.split_to(frame_size).freeze();

            frames.push(Frame {
                offset: self.offset,
                header,
                body: bytes.slice(body_start..),
            });

            self.offset += frame_size as u64;
        }

        Ok(frames)
    }
}
