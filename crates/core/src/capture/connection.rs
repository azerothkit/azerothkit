use super::{CapturedDatagram, CapturedStream};

#[derive(Debug, Clone)]
pub struct RemoteEndpoint {
    pub host: String,
    pub port: u16,
}

pub enum CapturedTransport {
    Tcp(CapturedStream),
    Udp(CapturedDatagram),
}

pub struct CapturedConnection {
    pub id: u64,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub remote: RemoteEndpoint,
    pub transport: CapturedTransport,
}
