use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;

#[derive(Debug, Clone)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let directories =
            BaseDirs::new().context("Failed to determine the user's home directory")?;

        Ok(Self {
            root: directories.home_dir().join(".azerothkit"),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn tls_dir(&self) -> PathBuf {
        self.root.join("tls")
    }

    pub fn ca_file(&self) -> PathBuf {
        self.tls_dir().join("azerothkit-ca.pem")
    }

    pub fn ca_certificate(&self) -> PathBuf {
        self.tls_dir().join("azerothkit-ca-cert.pem")
    }
}
