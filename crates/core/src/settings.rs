use std::{
    collections::BTreeMap,
    fs,
    io::{ErrorKind, Write},
    path::PathBuf,
    sync::Mutex,
};

use anyhow::{Context, Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::{
    app_paths::AppPaths,
    clients::{ClientId, available_clients},
};

const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub selected_client: ClientId,
    pub settings_by_client: BTreeMap<ClientId, ClientSettings>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            selected_client: ClientId::Forever,
            settings_by_client: available_clients()
                .into_iter()
                .map(|client| (client.id, ClientSettings::default()))
                .collect(),
        }
    }
}

impl AppSettings {
    fn fill_missing_clients(&mut self) {
        for client in available_clients() {
            self.settings_by_client.entry(client.id).or_default();
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.settings_by_client.contains_key(&self.selected_client),
            "Settings for the selected client are missing"
        );

        for (client_id, settings) in &self.settings_by_client {
            settings
                .validate()
                .with_context(|| {
                    format!("Invalid settings for client {client_id:?}")
                })?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct ClientSettings {
    pub features: FeatureSettings,
}

impl ClientSettings {
    pub fn validate(&self) -> Result<()> {
        self.features.time_override.validate()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct FeatureSettings {
    pub time_override: TimeOverrideSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct TimeOverrideSettings {
    pub enabled: bool,

    /// Minutes since midnight in the game world.
    pub entry_time_minutes: u16,

    pub reset_on_server_sync: bool,

    /// Real-world duration of a full in-game day.
    pub day_length_ms: u32,

    /// Preferred unit for displaying the duration in the UI.
    pub day_length_unit: DurationUnit,
}

impl Default for TimeOverrideSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            entry_time_minutes: 18 * 60,
            reset_on_server_sync: false,
            day_length_ms: 24 * 60 * 60 * 1_000,
            day_length_unit: DurationUnit::Hours,
        }
    }
}

impl TimeOverrideSettings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.entry_time_minutes < 24 * 60,
            "Entry time must be between 00:00 and 23:59"
        );

        ensure!(
            self.day_length_ms > 0,
            "Day length must be greater than zero"
        );

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum DurationUnit {
    Seconds,
    Minutes,
    Hours,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsFile {
    version: u32,
    settings: AppSettings,
}

#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    settings: Mutex<AppSettings>,
}

impl SettingsStore {
    pub fn open(paths: &AppPaths) -> Result<Self> {
        let path = paths.settings_file();

        let mut settings = match fs::read(&path) {
            Ok(contents) => {
                let document: SettingsFile =
                    serde_json::from_slice(&contents)
                        .with_context(|| {
                            format!(
                                "Failed to parse settings file: {}",
                                path.display()
                            )
                        })?;

                ensure!(
                    document.version == SETTINGS_VERSION,
                    "Unsupported settings version: {}",
                    document.version
                );

                document.settings
            }

            Err(error) if error.kind() == ErrorKind::NotFound => {
                AppSettings::default()
            }

            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "Failed to read settings file: {}",
                        path.display()
                    )
                });
            }
        };

        settings.fill_missing_clients();
        settings.validate()?;

        Ok(Self {
            path,
            settings: Mutex::new(settings),
        })
    }

    pub fn snapshot(&self) -> Result<AppSettings> {
        let settings = self
            .settings
            .lock()
            .map_err(|_| anyhow!("Settings lock is poisoned"))?;

        Ok(settings.clone())
    }

    pub fn save_client_settings(
        &self,
        client_id: ClientId,
        client_settings: ClientSettings,
    ) -> Result<AppSettings> {
        self.update(move |settings| {
            settings
                .settings_by_client
                .insert(client_id, client_settings);
        })
    }

    pub fn set_selected_client(
        &self,
        client_id: ClientId,
    ) -> Result<AppSettings> {
        self.update(move |settings| {
            settings.selected_client = client_id;
        })
    }

    fn update(
        &self,
        change: impl FnOnce(&mut AppSettings),
    ) -> Result<AppSettings> {
        let mut current = self
            .settings
            .lock()
            .map_err(|_| anyhow!("Settings lock is poisoned"))?;

        let mut next = current.clone();
        change(&mut next);

        next.validate()?;
        self.persist(&next)?;

        // Commit in-memory state only after a successful write.
        *current = next;

        Ok(current.clone())
    }

    fn persist(&self, settings: &AppSettings) -> Result<()> {
        let directory = self
            .path
            .parent()
            .context("Settings path has no parent directory")?;

        fs::create_dir_all(directory).with_context(|| {
            format!(
                "Failed to create settings directory: {}",
                directory.display()
            )
        })?;

        let document = SettingsFile {
            version: SETTINGS_VERSION,
            settings: settings.clone(),
        };

        // Keep the temporary file on the same filesystem.
        let mut temporary = NamedTempFile::new_in(directory)
            .context("Failed to create temporary settings file")?;

        serde_json::to_writer_pretty(&mut temporary, &document)
            .context("Failed to serialize settings")?;

        temporary
            .write_all(b"\n")
            .context("Failed to finish writing settings")?;

        temporary
            .as_file()
            .sync_all()
            .context("Failed to sync temporary settings file")?;

        temporary.persist(&self.path).with_context(|| {
            format!(
                "Failed to replace settings file: {}",
                self.path.display()
            )
        })?;

        Ok(())
    }
}