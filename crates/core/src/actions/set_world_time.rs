use anyhow::Result;

use crate::engine::EngineConfig;
use crate::features::WorldTimeConfig;

/// Configures the next engine run. None disables the feature.
pub fn set_world_time(config: &mut EngineConfig, settings: Option<WorldTimeConfig>) -> Result<()> {
    if let Some(settings) = &settings {
        settings.validate()?;
    }

    config.features.world_time = settings;
    Ok(())
}
