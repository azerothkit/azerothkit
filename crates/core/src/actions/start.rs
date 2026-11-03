use anyhow::Result;

use crate::engine::{self, EngineConfig, EngineHandle};

pub async fn start(config: EngineConfig) -> Result<EngineHandle> {
    engine::start(config).await
}
