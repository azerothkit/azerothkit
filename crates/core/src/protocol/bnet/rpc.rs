use std::collections::HashMap;

use anyhow::Result;

use super::framing::{Frame, FrameBuffer};
use super::messages::{Attribute, decode_attributes};
use super::methods::{self, Method};

pub(super) struct RpcPacket {
    pub token: u32,
    pub attributes: Vec<Attribute>,
}

pub(super) struct RpcSession {
    buffers: [FrameBuffer; 2],
    pending: HashMap<(bool, u32), Method>,
}

impl RpcSession {
    pub(super) fn new() -> Self {
        Self {
            buffers: [FrameBuffer::new(), FrameBuffer::new()],
            pending: HashMap::new(),
        }
    }

    pub(super) fn feed(&mut self, from_client: bool, data: &[u8]) -> Result<Vec<RpcPacket>> {
        let frames = self.buffers[usize::from(from_client)].feed(data)?;

        frames
            .into_iter()
            .map(|frame| self.decode(from_client, frame))
            .collect()
    }

    fn decode(&mut self, from_client: bool, frame: Frame) -> Result<RpcPacket> {
        let token = frame.header.token;

        let response =
            frame.header.service_id == 0xfe || frame.header.is_response.unwrap_or_default();

        let method = if response {
            self.pending.remove(&(!from_client, token))
        } else {
            let method = methods::lookup(
                frame.header.service_hash.unwrap_or_default(),
                frame.header.method_id.unwrap_or_default(),
            );

            let key = (from_client, token);

            if let Some(method) = method {
                self.pending.insert(key, method);
            } else {
                self.pending.remove(&key);
            }

            method
        };

        let Some(method) = method else {
            return Ok(RpcPacket {
                token,
                attributes: Vec::new(),
            });
        };

        if response && frame.header.status.unwrap_or_default() != 0 && frame.body.is_empty() {
            return Ok(RpcPacket {
                token,
                attributes: Vec::new(),
            });
        }

        let attributes = match decode_attributes(method, response, &frame.body) {
            Ok(attributes) => attributes,
            Err(error) => {
                log::debug!(
                    "Battle.net body decode failed: \
                         method={method:?}, offset={}, error={error:#}",
                    frame.offset,
                );

                Vec::new()
            }
        };

        Ok(RpcPacket { token, attributes })
    }
}
