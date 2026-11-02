use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;

use crate::capture::{CapturedConnection, CapturedTransport};
use crate::handlers::{Handler, HandlerOutput};
use crate::tls::TlsMitm;

pub(crate) async fn relay(
    connection: CapturedConnection,
    tls: Option<Arc<TlsMitm>>,
    handler: Box<dyn Handler>,
) -> Result<()> {
    let CapturedConnection {
        id,
        pid,
        process_name,
        remote,
        transport,
    } = connection;

    let protocol = match &transport {
        CapturedTransport::Tcp(_) => "TCP",
        CapturedTransport::Udp(_) => "UDP",
    };

    let uses_tls = matches!(&transport, CapturedTransport::Tcp(_)) && tls.is_some();

    log::info!(
        "[{id}] transport={protocol}, process={process_name:?}, \
         pid={pid:?}, remote={}:{}",
        remote.host,
        remote.port,
    );

    let (uploaded, downloaded) = match transport {
        CapturedTransport::Tcp(downstream) => {
            super::tcp::relay(id, &remote, downstream, tls, handler).await?
        }
        CapturedTransport::Udp(downstream) => super::udp::relay(id, downstream, handler).await?,
    };

    log::info!(
        "[{id}] closed: transport={protocol}, tls={uses_tls}, \
         uploaded={uploaded}, downloaded={downloaded}",
    );

    Ok(())
}

pub(super) fn output_chunks(output: HandlerOutput) -> impl Iterator<Item = Bytes> {
    let (single, batch) = match output {
        HandlerOutput::None => (None, Vec::new()),
        HandlerOutput::Forward(data) => (Some(data), Vec::new()),
        HandlerOutput::Batch(data) => (None, data),
    };

    single.into_iter().chain(batch)
}
