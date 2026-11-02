use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub filter: String,

    /// Redirector executable used by the Linux and Windows backends.
    pub helper_executable: Option<PathBuf>,
}
