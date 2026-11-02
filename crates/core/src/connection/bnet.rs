use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, copy_bidirectional};

use crate::protocol::bnet::AuthTracker;

pub(super) async fn relay<A, B>(
    downstream: A,
    upstream: B,
    tracker: AuthTracker,
) -> io::Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let tracker = Arc::new(Mutex::new(tracker));

    let mut downstream = ObservedStream {
        inner: downstream,
        from_client: true,
        tracker: tracker.clone(),
    };

    let mut upstream = ObservedStream {
        inner: upstream,
        from_client: false,
        tracker,
    };

    copy_bidirectional(&mut downstream, &mut upstream).await
}

struct ObservedStream<S> {
    inner: S,
    from_client: bool,
    tracker: Arc<Mutex<AuthTracker>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for ObservedStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let previous_size = output.filled().len();

        ready!(Pin::new(&mut this.inner).poll_read(cx, output))?;

        let data = &output.filled()[previous_size..];

        if !data.is_empty() {
            let mut tracker = this
                .tracker
                .lock()
                .map_err(|_| io::Error::other("Battle.net tracker is poisoned"))?;

            tracker.feed(this.from_client, data).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Battle.net processing failed: {error:#}"),
                )
            })?;
        }

        Poll::Ready(Ok(()))
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for ObservedStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, data)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}
