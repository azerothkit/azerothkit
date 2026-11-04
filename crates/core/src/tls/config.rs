use crate::clients::ClientId;

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub ports: Vec<u16>,
}

impl TlsConfig {
    pub fn for_client(client: ClientId) -> Self {
        Self {
            ports: client.profile().tls_ports.to_vec(),
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self::for_client(ClientId::Retail)
    }
}
