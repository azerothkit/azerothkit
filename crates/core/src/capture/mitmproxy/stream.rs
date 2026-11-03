use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use mitmproxy::messages::{ConnectionId, TransportCommand};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{mpsc, oneshot};

#[cfg(target_os = "macos")]
use crate::capture::CaptureFlowClosed;

const MAX_CHUNK: usize = 64 * 1024;

pub struct CapturedStream {
    id: ConnectionId,
    commands: mpsc::UnboundedSender<TransportCommand>,

    read_rx: Option<oneshot::Receiver<Vec<u8>>>,
    read_buffer: Vec<u8>,
    read_position: usize,
    read_eof: bool,

    drain_rx: Option<oneshot::Receiver<()>>,
    write_closed: bool,
}

impl CapturedStream {
    pub(super) fn new(id: ConnectionId, commands: mpsc::UnboundedSender<TransportCommand>) -> Self {
        debug_assert!(id.is_tcp());

        Self {
            id,
            commands,
            read_rx: None,
            read_buffer: Vec::new(),
            read_position: 0,
            read_eof: false,
            drain_rx: None,
            write_closed: false,
        }
    }

    fn command(&self, command: TransportCommand) -> io::Result<()> {
        self.commands
            .send(command)
            .map_err(|_| disconnected("command channel closed"))
    }

    fn poll_drain(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Some(rx) = self.drain_rx.as_mut() else {
            return Poll::Ready(Ok(()));
        };

        let result = ready!(Pin::new(rx).poll(cx));
        self.drain_rx = None;

        Poll::Ready(result.map_err(|_| disconnected("write drain reply dropped")))
    }
}

impl AsyncRead for CapturedStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();

        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        if this.read_position == this.read_buffer.len() {
            if this.read_eof {
                return Poll::Ready(end_of_stream());
            }

            if this.read_rx.is_none() {
                let (tx, rx) = oneshot::channel();
                let size = output.remaining().min(MAX_CHUNK) as u32;

                this.command(TransportCommand::ReadData(this.id, size, tx))?;

                this.read_rx = Some(rx);
            }

            let rx = this.read_rx.as_mut().expect("pending TCP read");

            let result = ready!(Pin::new(rx).poll(cx));
            this.read_rx = None;

            this.read_buffer = result.map_err(|_| disconnected("read reply dropped"))?;

            this.read_position = 0;

            if this.read_buffer.is_empty() {
                this.read_eof = true;
                return Poll::Ready(end_of_stream());
            }
        }

        let remaining = &this.read_buffer[this.read_position..];

        let size = remaining.len().min(output.remaining());

        output.put_slice(&remaining[..size]);
        this.read_position += size;

        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for CapturedStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();

        if this.write_closed {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Captured TCP write side is closed",
            )));
        }

        ready!(this.poll_drain(cx))?;

        if data.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let size = data.len().min(MAX_CHUNK);

        this.command(TransportCommand::WriteData(this.id, data[..size].to_vec()))?;

        let (tx, rx) = oneshot::channel();

        this.command(TransportCommand::DrainWriter(this.id, tx))?;

        this.drain_rx = Some(rx);

        Poll::Ready(Ok(size))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_drain(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();

        ready!(this.poll_drain(cx))?;

        if !this.write_closed {
            this.command(TransportCommand::CloseConnection(this.id, true))?;

            this.write_closed = true;
        }

        Poll::Ready(Ok(()))
    }
}

impl Drop for CapturedStream {
    fn drop(&mut self) {
        let _ = self
            .commands
            .send(TransportCommand::CloseConnection(self.id, false));
    }
}

fn disconnected(operation: &'static str) -> io::Error {
    #[cfg(target_os = "macos")]
    {
        // Channel closure reports termination of the backend flow.
        // The backend remains responsible for logging its original error.
        io::Error::new(
            io::ErrorKind::ConnectionAborted,
            CaptureFlowClosed::new(operation),
        )
    }

    #[cfg(not(target_os = "macos"))]
    {
        io::Error::new(
            io::ErrorKind::BrokenPipe,
            format!("Capture TCP flow ended: {operation}"),
        )
    }
}

fn end_of_stream() -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        // TODO(macos): The signed Redirector closes the entire captured
        // flow when a copier reaches EOF. Treat capture EOF as termination
        // of both relay directions instead of waiting for upstream data.
        // This does not restore TCP half-close support.
        // Revisit after upstream fixes cleanup without this regression.
        // https://github.com/mitmproxy/mitmproxy/issues/7675
        Err(disconnected("read EOF"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}
