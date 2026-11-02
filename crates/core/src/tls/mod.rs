mod ca;
mod setup;

use crate::app_paths::AppPaths;

pub use setup::installation_instructions;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, ensure};
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::{
    LazyConfigAcceptor, TlsConnector,
    rustls::{
        self, ClientConfig, RootCertStore, ServerConfig, crypto::CryptoProvider,
        pki_types::ServerName,
    },
};

use crate::capture::CapturedConnection;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub ports: Vec<u16>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self { ports: vec![443] }
    }
}

type CacheKey = (String, Option<Vec<u8>>);

struct CachedConfig {
    created: Instant,
    config: Arc<ServerConfig>,
}

pub(crate) struct TlsMitm {
    authority: ca::Authority,
    ports: Vec<u16>,
    provider: Arc<CryptoProvider>,
    client_config: ClientConfig,
    certificates: Mutex<HashMap<CacheKey, CachedConfig>>,
}

impl TlsMitm {
    pub(crate) fn load(config: TlsConfig) -> Result<Self> {
        let paths = AppPaths::discover()?;

        let public_certificate = ca::initialize_ca(&paths)?;
        let authority = ca::Authority::load(&paths.ca_file())?;

        log::info!("TLS CA certificate: {}", public_certificate.display(),);

        let provider = Arc::new(rustls::crypto::ring::default_provider());

        let native = rustls_native_certs::load_native_certs();

        let ca_found = native
            .certs
            .iter()
            .any(|certificate| certificate.as_ref() == authority.certificate_der());

        for error in native.errors {
            log::warn!("Failed to load a system certificate: {error}");
        }

        if ca_found {
            log::info!("TLS CA found in loaded system roots");
        } else {
            log::warn!(
                "TLS CA was not found in loaded system roots. \
                 If it is not trusted yet, install it:\n{}",
                installation_instructions(&public_certificate),
            );
        }

        let mut roots = RootCertStore::empty();
        let (accepted, _) = roots.add_parsable_certificates(native.certs);

        ensure!(accepted > 0, "No system root certificates were loaded");

        let client_config = ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();

        Ok(Self {
            authority,
            ports: config.ports,
            provider,
            client_config,
            certificates: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn intercepts_port(&self, port: u16) -> bool {
        self.ports.contains(&port)
    }

    fn server_config(&self, host: &str, alpn: Option<Vec<u8>>) -> Result<Arc<ServerConfig>> {
        let key = (host.to_owned(), alpn.clone());
        let mut cache = self
            .certificates
            .lock()
            .map_err(|_| anyhow!("TLS certificate cache is poisoned"))?;

        cache.retain(|_, entry| entry.created.elapsed() < Duration::from_secs(24 * 60 * 60));

        if let Some(entry) = cache.get(&key) {
            return Ok(entry.config.clone());
        }

        let config = Arc::new(
            self.authority
                .server_config(host, alpn, self.provider.clone())?,
        );

        if cache.len() >= 256 {
            cache.clear();
        }

        cache.insert(
            key,
            CachedConfig {
                created: Instant::now(),
                config: config.clone(),
            },
        );

        Ok(config)
    }

    pub(crate) async fn relay(
        &self,
        connection: CapturedConnection,
        upstream: TcpStream,
    ) -> Result<(u64, u64)> {
        let CapturedConnection {
            id, remote, stream, ..
        } = connection;

        let handshake = async {
            let start = LazyConfigAcceptor::new(rustls::server::Acceptor::default(), stream)
                .await
                .context("Failed to read TLS ClientHello")?;

            let (host, offered_alpn) = {
                let hello = start.client_hello();

                let host = hello
                    .server_name()
                    .unwrap_or(remote.host.as_str())
                    .to_owned();

                let alpn = hello
                    .alpn()
                    .map(|protocols| protocols.map(|protocol| protocol.to_vec()).collect())
                    .unwrap_or_else(Vec::new);

                (host, alpn)
            };

            let mut client_config = self.client_config.clone();
            client_config.alpn_protocols = offered_alpn;

            let connector = TlsConnector::from(Arc::new(client_config));
            let server_name =
                ServerName::try_from(host.clone()).context("Invalid TLS server name")?;

            let upstream = connector
                .connect(server_name, upstream)
                .await
                .context("Upstream TLS handshake failed")?;

            let selected_alpn = upstream
                .get_ref()
                .1
                .alpn_protocol()
                .map(|protocol| protocol.to_vec());

            let config = self.server_config(&host, selected_alpn.clone())?;

            let downstream = start
                .into_stream(config)
                .await
                .context("Client TLS handshake failed")?;

            log::info!(
                "[{id}] TLS established: name={host:?}, alpn={:?}",
                selected_alpn.as_deref().map(String::from_utf8_lossy),
            );

            Ok::<_, anyhow::Error>((downstream, upstream))
        };

        let (mut downstream, mut upstream) = timeout(Duration::from_secs(20), handshake)
            .await
            .context("TLS handshake timed out")??;

        match copy_bidirectional(&mut downstream, &mut upstream).await {
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                Err(error).context(UncleanTlsClose)
            }
            result => result.context("TLS relay failed"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct UncleanTlsClose;

impl std::fmt::Display for UncleanTlsClose {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TLS connection closed without close_notify")
    }
}
