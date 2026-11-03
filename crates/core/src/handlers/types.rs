use bytes::Bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    ClientToServer,
    ServerToClient,
}

impl Direction {
    pub(crate) fn is_from_client(self) -> bool {
        matches!(self, Self::ClientToServer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HandlerKind {
    #[default]
    Passthrough,
    Bnet,
    World,
}

/// For TCP, an ordered stream chunk.
/// For UDP, one complete datagram, including an empty datagram.
pub(crate) struct HandlerInput {
    pub direction: Direction,
    pub data: Bytes,
}

/// Output is sent in the same direction as the input.
pub(crate) enum HandlerOutput {
    /// Nothing to send.
    /// The handler must retain any buffered input.
    None,

    /// One TCP chunk or one UDP datagram.
    /// Empty UDP datagrams must still be sent.
    Forward(Bytes),

    /// Ordered TCP chunks or separate UDP datagrams.
    /// UDP datagrams must not be merged.
    Batch(Vec<Bytes>),
}
