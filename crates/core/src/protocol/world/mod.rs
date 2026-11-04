mod codec;
mod compression;
mod crypto;
mod framing;
mod handshake;
mod messages;
mod packets;
mod profile;

pub use codec::{DecodedFrame, WorldCodec, WorldEvent};
pub use crypto::KeyCandidate;
pub use handshake::AuthRequest;
pub use packets::{KnownPacket, PackedTime, WorldPacket};
pub use profile::{
    MessageCodec, TimeOpcodes, WorldOpcodes, WorldProtocol,
};

pub(super) const MAX_BODY_SIZE: usize = 16 * 1024 * 1024;
