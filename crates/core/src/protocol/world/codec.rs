use anyhow::{Result, ensure};
use bytes::Bytes;

use crate::sessions::WorldSessionKey;

use super::compression::{Deflater, Inflater};
use super::crypto::{CryptoState, KeyCandidate};
use super::framing::{FrameBuffer, WireMessage, encode_frame};
use super::handshake::{AuthRequest, Handshake};
use super::packets::{
    COMPRESSED, ENCRYPTED_ACK, MULTIPLE, WorldPacket, bundle_messages, split_messages,
};

pub enum WorldEvent {
    Greeting(Bytes),
    Packet(DecodedFrame),
}

pub struct DecodedFrame {
    pub counter: u64,
    pub encrypted: bool,
    pub compressed: bool,
    pub packets: Vec<WorldPacket>,
    pub auth: Option<AuthRequest>,
    pub verified_session: Option<WorldSessionKey>,

    wire: Bytes,
    was_multiple: bool,
}

pub struct WorldCodec {
    buffers: [FrameBuffer; 2],
    handshake: Handshake,
    crypto: CryptoState,
    inflater: Inflater,
    deflater: Deflater,
    compression_rewritten: bool,
}

impl Default for WorldCodec {
    fn default() -> Self {
        Self {
            buffers: [FrameBuffer::new(false), FrameBuffer::new(true)],
            handshake: Handshake::default(),
            crypto: CryptoState::default(),
            inflater: Inflater::default(),
            deflater: Deflater::default(),
            compression_rewritten: false,
        }
    }
}

impl WorldCodec {
    pub fn push(&mut self, from_client: bool, data: &[u8]) {
        self.buffers[usize::from(from_client)].push(data);
    }

    pub fn bind_candidates(&mut self, candidates: Vec<KeyCandidate>) -> Result<()> {
        self.crypto.bind(candidates)
    }

    /// Process one event at a time so authentication results can be
    /// applied before decoding subsequent frames from the same chunk.
    pub fn next(&mut self, from_client: bool) -> Result<Option<WorldEvent>> {
        let Some(message) = self.buffers[usize::from(from_client)].next()? else {
            return Ok(None);
        };

        let frame = match message {
            WireMessage::Greeting(wire) => {
                return Ok(Some(WorldEvent::Greeting(wire)));
            }
            WireMessage::Frame(frame) => frame,
        };

        let encrypted = self.handshake.encrypted();

        let (plain, verified_session) = if encrypted {
            self.crypto
                .decrypt(from_client, frame.counter, &frame.tag, &frame.body)?
        } else {
            ensure!(
                frame.tag == [0u8; 12],
                "Nonzero World tag before encryption ACK",
            );

            (frame.body.clone(), None)
        };

        let mut root = WorldPacket::from_body(plain)?;
        let compressed = !from_client && root.opcode == COMPRESSED;

        if compressed {
            root = WorldPacket::from_body(self.inflater.decode(&root.payload)?)?;
        }

        let auth = if encrypted {
            None
        } else {
            if from_client && root.opcode == ENCRYPTED_ACK {
                ensure!(
                    self.crypto.is_ready(),
                    "World encryption ACK without bound keys",
                );
            }

            self.handshake.observe(from_client, &root)?
        };

        let was_multiple = !from_client && root.opcode == MULTIPLE;

        let packets = if from_client {
            vec![root]
        } else {
            split_messages(root)?
        };

        Ok(Some(WorldEvent::Packet(DecodedFrame {
            counter: frame.counter,
            encrypted,
            compressed,
            packets,
            auth,
            verified_session,
            wire: frame.wire,
            was_multiple,
        })))
    }

    /// Keeps one outgoing outer frame per incoming frame.
    /// Server message expansion is encoded inside MULTIPLE.
    ///
    /// After modifying a compressed frame, every subsequent compressed
    /// frame must pass through this method to preserve output history.
    pub fn encode(
        &mut self,
        from_client: bool,
        frame: &DecodedFrame,
        packets: &[WorldPacket],
    ) -> Result<Bytes> {
        let unchanged = packets == frame.packets.as_slice();

        if unchanged && (!frame.compressed || !self.compression_rewritten) {
            return Ok(frame.wire.clone());
        }

        ensure!(
            frame.encrypted,
            "Modification of World authentication frames is unsupported",
        );

        ensure!(
            !from_client || packets.len() == 1,
            "Client World message expansion is unsupported",
        );

        let body = bundle_messages(packets, frame.was_multiple)?;

        let body = if frame.compressed {
            let payload = self.deflater.encode(&body)?;
            self.compression_rewritten = true;

            WorldPacket {
                opcode: COMPRESSED,
                payload,
            }
            .to_body()?
        } else {
            body
        };

        let (tag, ciphertext) = self.crypto.encrypt(from_client, frame.counter, &body)?;

        encode_frame(&tag, &ciphertext)
    }

    pub fn finish(&self, from_client: bool) -> Result<()> {
        self.buffers[usize::from(from_client)].finish()
    }
}
