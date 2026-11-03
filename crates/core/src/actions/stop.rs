use anyhow::Result;

use crate::engine::EngineHandle;

pub async fn stop(engine: EngineHandle) -> Result<()> {
    engine.stop().await
}
