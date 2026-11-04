use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use crate::constant::APP_NAME;

#[derive(Debug, Clone)]
pub struct AppPaths {
    config_dir: PathBuf,
    data_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let project_dirs = ProjectDirs::from("app", "ozd0", APP_NAME)
            .context("Failed to determine application directories")?;

        Ok(Self {
            config_dir: project_dirs.config_dir().to_path_buf(),
            data_dir: project_dirs.data_local_dir().to_path_buf(),
        })
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn tls_dir(&self) -> PathBuf {
        self.config_dir.join("tls")
    }

    pub fn ca_file(&self) -> PathBuf {
        self.tls_dir().join("azerothkit-ca.pem")
    }

    pub fn ca_certificate(&self) -> PathBuf {
        self.tls_dir().join("azerothkit-ca-cert.pem")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }
}
