use std::fmt;

#[derive(Debug)]
pub(crate) struct UncleanTlsClose;

impl fmt::Display for UncleanTlsClose {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TLS connection closed without close_notify")
    }
}

impl std::error::Error for UncleanTlsClose {}
