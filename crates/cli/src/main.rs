use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use azerothkit_core::{
    capture::CaptureConfig,
    engine::{self, EngineConfig},
    tls::TlsConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut args = std::env::args().skip(1);

    let filter = args.next().context(
        "Usage: azerothkit-cli <filter> \
         [--tls] [--helper <executable>]",
    )?;

    let mut helper_executable = None;
    let mut tls_enabled = false;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--tls" => tls_enabled = true,
            "--helper" => {
                helper_executable = Some(PathBuf::from(
                    args.next().context("--helper requires an executable")?,
                ));
            }
            value if !value.starts_with('-') && helper_executable.is_none() => {
                helper_executable = Some(PathBuf::from(value));
            }
            _ => bail!("Unexpected argument: {argument}"),
        }
    }

    let config = EngineConfig {
        capture: CaptureConfig {
            filter,
            helper_executable,
        },
        tls: tls_enabled.then(TlsConfig::default),
        handlers: Default::default(),
    };

    engine::run(config, async {
        tokio::signal::ctrl_c()
            .await
            .context("Failed to receive Ctrl+C")
    })
    .await
}
