use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use tokio_rustls::rustls::{
    self, DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use x509_parser::extensions::GeneralName;

pub(super) fn insecure_verifier(provider: Arc<CryptoProvider>) -> Arc<dyn ServerCertVerifier> {
    Arc::new(InsecureServerVerifier { provider })
}

#[derive(Debug)]
struct InsecureServerVerifier {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for InsecureServerVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub(super) fn certificate_names(
    certificate: &CertificateDer<'_>,
    fallback: &str,
) -> Result<Vec<String>> {
    let (_, certificate) = x509_parser::parse_x509_certificate(certificate.as_ref())
        .map_err(|_| anyhow!("Invalid upstream X.509 certificate"))?;

    let mut names = Vec::new();

    if fallback.parse::<IpAddr>().is_err() {
        names.push(fallback.to_owned());
    }

    for attribute in certificate.subject().iter_common_name() {
        if let Ok(name) = attribute.as_str() {
            names.push(name.to_owned());
        }
    }

    if let Some(extension) = certificate
        .subject_alternative_name()
        .context("Invalid upstream certificate SAN")?
    {
        for name in &extension.value.general_names {
            match name {
                GeneralName::DNSName(name) => {
                    names.push((*name).to_owned());
                }

                GeneralName::IPAddress(bytes) if bytes.len() == 4 => {
                    let address: [u8; 4] = (*bytes).try_into()?;
                    names.push(Ipv4Addr::from(address).to_string());
                }

                GeneralName::IPAddress(bytes) if bytes.len() == 16 => {
                    let address: [u8; 16] = (*bytes).try_into()?;
                    names.push(Ipv6Addr::from(address).to_string());
                }

                _ => {}
            }
        }
    }

    names.push(fallback.to_owned());

    let mut seen = HashSet::new();
    names.retain(|name| !name.is_empty() && seen.insert(name.clone()));

    Ok(names)
}
