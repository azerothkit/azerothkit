#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub ports: Vec<u16>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            ports: vec![443, 1119],
        }
    }
}
