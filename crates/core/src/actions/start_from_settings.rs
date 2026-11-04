use anyhow::{Context, Result};

use crate::{
    app_paths::AppPaths,
    capture::CaptureConfig,
    engine::{EngineConfig, EngineHandle},
    features::{TimeOfDay, WorldTimeConfig},
    settings::SettingsStore,
    tls::TlsConfig,
};
use crate::constants::DEFAULT_PROCESS_NAME;

pub async fn start_from_settings(paths: &AppPaths) -> Result<EngineHandle> {
    let paths = paths.clone();

    let config = tokio::task::spawn_blocking(move || -> Result<EngineConfig> {
        // Read the settings file on every start, including restart.
        let settings = SettingsStore::open(&paths)?.snapshot()?;

        let client_settings = settings
            .settings_by_client
            .get(&settings.selected_client)
            .context("Settings for the selected client are missing")?;

        let mut config = EngineConfig {
            capture: CaptureConfig {
                filter: DEFAULT_PROCESS_NAME.to_owned(),
                helper_executable: None,
            },
            tls: Some(TlsConfig::default()),
            handlers: Default::default(),
            features: Default::default(),
        };

        let time = &client_settings.features.time_override;

        if time.enabled {
            super::set_world_time(
                &mut config,
                Some(WorldTimeConfig {
                    day_minutes: Some(f64::from(time.day_length_ms) / 60_000.0),
                    start_time: Some(TimeOfDay::new(
                        (time.entry_time_minutes / 60) as u8,
                        (time.entry_time_minutes % 60) as u8,
                    )?),
                    pin_on_sync: time.reset_on_server_sync,
                }),
            )?;
        }

        Ok(config)
    })
        .await
        .context("Settings loading task failed")??;

    super::start(config).await
}
