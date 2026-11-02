mod mitmproxy;

use std::path::PathBuf;

pub use mitmproxy::{Capture, CapturedStream};

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub filter: String,

    pub helper_executable: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RemoteEndpoint {
    pub host: String,
    pub port: u16,
}

pub struct CapturedConnection {
    pub id: u64,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub remote: RemoteEndpoint,
    pub stream: CapturedStream,
}
