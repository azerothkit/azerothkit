use std::io;
use std::net::SocketAddr;

use mitmproxy::messages::{ConnectionId, TransportCommand};
use tokio::sync::{mpsc, oneshot};

pub struct CapturedDatagram {
    id: ConnectionId,
    commands: mpsc::UnboundedSender<TransportCommand>,
    remote: SocketAddr,
    read_rx: Option<oneshot::Receiver<Vec<u8>>>,
}

impl CapturedDatagram {
    pub(super) fn new(
        id: ConnectionId,
        commands: mpsc::UnboundedSender<TransportCommand>,
        remote: SocketAddr,
    ) -> Self {
        debug_assert!(!id.is_tcp());

        Self {
            id,
            commands,
            remote,
            read_rx: None,
        }
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote
    }

    /// Receives one complete datagram. An empty datagram is valid data.
    /// The pending read survives cancellation of this future.
    pub async fn recv(&mut self) -> io::Result<Vec<u8>> {
        if self.read_rx.is_none() {
            let (tx, rx) = oneshot::channel();

            // The backend ignores the size argument for UDP.
            self.command(TransportCommand::ReadData(self.id, 0, tx))?;

            self.read_rx = Some(rx);
        }

        let result = self.read_rx.as_mut().expect("pending UDP read").await;

        self.read_rx = None;

        result.map_err(|_| disconnected())
    }

    /// Sends one complete datagram, including an empty datagram.
    pub async fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.command(TransportCommand::WriteData(self.id, data.to_vec()))?;

        let (tx, rx) = oneshot::channel();

        self.command(TransportCommand::DrainWriter(self.id, tx))?;

        rx.await.map_err(|_| disconnected())
    }

    fn command(&self, command: TransportCommand) -> io::Result<()> {
        self.commands.send(command).map_err(|_| disconnected())
    }
}

impl Drop for CapturedDatagram {
    fn drop(&mut self) {
        let _ = self
            .commands
            .send(TransportCommand::CloseConnection(self.id, false));
    }
}

fn disconnected() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "Capture backend closed the UDP flow",
    )
}
