use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::app_paths::AppPaths;
use anyhow::{Context, Result, ensure};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    Issuer, KeyPair, KeyUsagePurpose,
};
use time::{Duration, OffsetDateTime};
use tokio_rustls::rustls::{
    ServerConfig,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};

pub(super) struct Authority {
    issuer: Issuer<'static, KeyPair>,
    certificate: CertificateDer<'static>,
}

pub(super) fn initialize_ca(paths: &AppPaths) -> Result<PathBuf> {
    fs::create_dir_all(paths.tls_dir()).context("Failed to create CA directory")?;

    let ca_file = paths.ca_file();
    let public_file = paths.ca_certificate();

    if !ca_file.try_exists()? {
        let key = KeyPair::generate()?;
        let mut params = CertificateParams::default();

        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, "AzerothKit Local CA");

        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::days(1);
        params.not_after = now + Duration::days(3650);

        let certificate = params.self_signed(&key)?;
        let contents = key.serialize_pem() + &certificate.pem();

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        match options.open(&ca_file) {
            Ok(mut file) => {
                if let Err(error) = file
                    .write_all(contents.as_bytes())
                    .and_then(|_| file.sync_all())
                {
                    drop(file);
                    let _ = fs::remove_file(&ca_file);
                    return Err(error).context("Failed to save CA");
                }
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).context("Failed to create CA file");
            }
        }
    }

    let _ = Authority::load(&ca_file)?;

    let blocks = pem::parse_many(fs::read(&ca_file)?)?;
    let certificate = blocks
        .iter()
        .find(|block| block.tag() == "CERTIFICATE")
        .context("CA certificate is missing")?;

    fs::write(&public_file, pem::encode(certificate))
        .context("Failed to export public CA certificate")?;

    Ok(public_file)
}

impl Authority {
    pub(super) fn certificate_der(&self) -> &[u8] {
        self.certificate.as_ref()
    }

    pub(super) fn load(path: &Path) -> Result<Self> {
        let contents =
            fs::read(path).with_context(|| format!("Failed to read CA: {}", path.display()))?;

        let blocks = pem::parse_many(contents)?;

        let certificate = blocks
            .iter()
            .find(|block| block.tag() == "CERTIFICATE")
            .context("CA certificate is missing")?;

        let private_key = blocks
            .iter()
            .find(|block| block.tag() == "PRIVATE KEY")
            .context("CA PKCS#8 private key is missing")?;

        let key = KeyPair::from_pem(&pem::encode(private_key))?;
        let certificate = CertificateDer::from(certificate.contents().to_vec());

        let issuer = Issuer::from_ca_cert_der(&certificate, key)?;

        ensure!(
            issuer.key_usages().contains(&KeyUsagePurpose::KeyCertSign),
            "CA does not permit certificate signing"
        );

        Ok(Self {
            issuer,
            certificate,
        })
    }

    pub(super) fn server_config(
        &self,
        host: &str,
        alpn: Option<Vec<u8>>,
        provider: Arc<CryptoProvider>,
    ) -> Result<ServerConfig> {
        let key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![host.to_owned()])?;

        params.distinguished_name = DistinguishedName::new();
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.use_authority_key_identifier_extension = true;

        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::days(1);
        params.not_after = now + Duration::days(199);

        let certificate = params.signed_by(&key, &self.issuer)?;

        let mut config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![certificate.der().clone()],
                PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
            )?;

        config.alpn_protocols = alpn.into_iter().collect();

        Ok(config)
    }
}
