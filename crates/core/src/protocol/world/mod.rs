mod codec;
mod compression;
mod crypto;
mod framing;
mod handshake;
mod packets;

pub use codec::{DecodedFrame, WorldCodec, WorldEvent};
pub use crypto::KeyCandidate;
pub use handshake::AuthRequest;
pub use packets::{
    AUTH_CHALLENGE, AUTH_CONTINUED, AUTH_SESSION, COMPRESSED, ENCRYPTED_ACK, ENTER_ENCRYPTED,
    GAME_TIME_SET, GAME_TIME_UPDATE, KnownPacket, LOGIN_TIME, MULTIPLE, PRINT_NOTIFICATION,
    PackedTime, WorldPacket,
};

pub(super) const MAX_BODY_SIZE: usize = 16 * 1024 * 1024;
