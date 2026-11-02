use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::capture::CapturedConnection;
use crate::tls::TlsMitm;

pub(crate) async fn relay(
    mut connection: CapturedConnection,
    tls: Option<Arc<TlsMitm>>,
) -> Result<()> {
    let id = connection.id;

    log::info!(
        "[{id}] process={:?}, pid={:?}, remote={}:{}",
        connection.process_name,
        connection.pid,
        connection.remote.host,
        connection.remote.port,
    );

    let mut upstream = timeout(
        Duration::from_secs(10),
        TcpStream::connect((connection.remote.host.as_str(), connection.remote.port)),
    )
    .await
    .context("Upstream connection timed out")?
    .context("Failed to connect to upstream")?;

    upstream.set_nodelay(true)?;

    let tls = tls.filter(|tls| tls.intercepts_port(connection.remote.port));

    let uses_tls = tls.is_some();

    let (uploaded, downloaded) = if let Some(tls) = tls {
        tls.relay(connection, upstream).await?
    } else {
        copy_bidirectional(&mut connection.stream, &mut upstream)
            .await
            .context("TCP relay failed")?
    };

    log::info!("[{id}] closed: tls={uses_tls}, uploaded={uploaded}, downloaded={downloaded}",);

    Ok(())
}
