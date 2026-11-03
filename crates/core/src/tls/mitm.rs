use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, ensure};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::{
    LazyConfigAcceptor, TlsConnector,
    rustls::{
        self, ClientConfig, RootCertStore, ServerConfig, crypto::CryptoProvider,
        pki_types::ServerName,
    },
};

use crate::app_paths::AppPaths;
use crate::capture::{CapturedStream, RemoteEndpoint};

use super::{TlsConfig, ca, identity, installation_instructions};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);
const CERTIFICATE_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const CERTIFICATE_CACHE_CAPACITY: usize = 256;

type CapturedTlsStream = tokio_rustls::server::TlsStream<CapturedStream>;

type UpstreamTlsStream = tokio_rustls::client::TlsStream<TcpStream>;

type CacheKey = (Vec<String>, Option<Vec<u8>>);

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
            log::warn!("Failed to load a system certificate: {error}",);
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

        ensure!(accepted > 0, "No system root certificates were loaded",);

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

    pub(crate) async fn handshake(
        &self,
        connection_id: u64,
        remote: &RemoteEndpoint,
        downstream: CapturedStream,
        upstream: TcpStream,
    ) -> Result<(CapturedTlsStream, UpstreamTlsStream)> {
        timeout(
            HANDSHAKE_TIMEOUT,
            self.handshake_inner(connection_id, remote, downstream, upstream),
        )
        .await
        .context("TLS handshake timed out")?
    }

    async fn handshake_inner(
        &self,
        connection_id: u64,
        remote: &RemoteEndpoint,
        downstream: CapturedStream,
        upstream: TcpStream,
    ) -> Result<(CapturedTlsStream, UpstreamTlsStream)> {
        let start = LazyConfigAcceptor::new(rustls::server::Acceptor::default(), downstream)
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
                .map(|protocols| {
                    protocols
                        .map(|protocol| protocol.to_vec())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            (host, alpn)
        };

        log::debug!(
            "[{connection_id}] TLS ClientHello parsed: \
             name={host:?}, remote={}:{}",
            remote.host,
            remote.port,
        );

        let mut client_config = self.client_config.clone();
        client_config.alpn_protocols = offered_alpn;

        if remote.port == 1119 {
            client_config
                .dangerous()
                .set_certificate_verifier(identity::insecure_verifier(self.provider.clone()));
        }

        let connector = TlsConnector::from(Arc::new(client_config));

        let server_name = ServerName::try_from(host.clone()).context("Invalid TLS server name")?;

        let upstream = connector
            .connect(server_name, upstream)
            .await
            .context("Upstream TLS handshake failed")?;

        let selected_alpn = upstream
            .get_ref()
            .1
            .alpn_protocol()
            .map(|protocol| protocol.to_vec());

        let certificate = upstream
            .get_ref()
            .1
            .peer_certificates()
            .and_then(|certificates| certificates.first())
            .context("Upstream did not provide a certificate")?;

        let names = identity::certificate_names(certificate, &host)?;

        let config = self.server_config(&names, selected_alpn.clone())?;

        log::debug!(
            "[{connection_id}] Upstream TLS established; \
             starting client handshake: name={host:?}",
        );

        let downstream = match start.into_stream(config).await {
            Ok(stream) => stream,
            Err(error) => {
                return Err(error).context("Client TLS handshake failed");
            }
        };

        log::info!(
            "[{connection_id}] TLS established: \
             name={host:?}, alpn={:?}",
            selected_alpn.as_deref().map(String::from_utf8_lossy),
        );

        Ok((downstream, upstream))
    }

    fn server_config(&self, names: &[String], alpn: Option<Vec<u8>>) -> Result<Arc<ServerConfig>> {
        let key = (names.to_vec(), alpn.clone());

        let mut cache = self
            .certificates
            .lock()
            .map_err(|_| anyhow!("TLS certificate cache is poisoned"))?;

        cache.retain(|_, entry| entry.created.elapsed() < CERTIFICATE_CACHE_TTL);

        if let Some(entry) = cache.get(&key) {
            return Ok(entry.config.clone());
        }

        let config = Arc::new(
            self.authority
                .server_config(names, alpn, self.provider.clone())?,
        );

        if cache.len() >= CERTIFICATE_CACHE_CAPACITY {
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
}
