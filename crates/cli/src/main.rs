use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use azerothkit_core::{
    actions,
    capture::CaptureConfig,
    engine::EngineConfig,
    features::{TimeOfDay, WorldTimeConfig},
    tls::TlsConfig,
};

const HELP: &str = "\
Usage:
  azerothkit-cli <filter> [options]

Capture:
  --tls                  Enable TLS interception
  --helper <executable>  Redirector executable for Linux/Windows

World time:
  --day-minutes <value>  Real minutes per complete in-game day
  --time <HH:MM>         Initial in-game time
  --pin-time             Reapply initial time on server synchronization

Lifecycle:
  Ctrl+C                 Stop the engine and intercepted connections

Examples:
  azerothkit-cli \"World of Warcraft\" --tls
  azerothkit-cli \"World of Warcraft\" --tls --day-minutes 60
  azerothkit-cli \"World of Warcraft\" --tls --time 18:30 --pin-time
";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut args = std::env::args().skip(1);

    let filter = args.next().context(HELP)?;

    if matches!(filter.as_str(), "--help" | "-h") {
        print!("{HELP}");
        return Ok(());
    }

    let mut helper_executable = None;
    let mut tls_enabled = false;

    let mut day_minutes = None;
    let mut start_time = None;
    let mut pin_on_sync = false;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--tls" => {
                tls_enabled = true;
            }

            "--helper" => {
                helper_executable = Some(PathBuf::from(
                    args.next().context("--helper requires an executable")?,
                ));
            }

            "--day-minutes" => {
                let value = args.next().context("--day-minutes requires a value")?;

                day_minutes = Some(value.parse::<f64>().context("Invalid day duration")?);
            }

            "--time" => {
                let value = args.next().context("--time requires HH:MM")?;

                start_time = Some(value.parse::<TimeOfDay>()?);
            }

            "--pin-time" => {
                pin_on_sync = true;
            }

            "--help" | "-h" => {
                print!("{HELP}");
                return Ok(());
            }

            value if !value.starts_with('-') && helper_executable.is_none() => {
                helper_executable = Some(PathBuf::from(value));
            }

            _ => bail!("Unexpected argument: {argument}\n\n{HELP}"),
        }
    }

    let mut config = EngineConfig {
        capture: CaptureConfig {
            filter,
            helper_executable,
        },
        tls: tls_enabled.then(TlsConfig::default),
        handlers: Default::default(),
        features: Default::default(),
    };

    if day_minutes.is_some() || start_time.is_some() || pin_on_sync {
        actions::set_world_time(
            &mut config,
            Some(WorldTimeConfig {
                day_minutes,
                start_time,
                pin_on_sync,
            }),
        )?;
    }

    let mut engine = actions::start(config).await?;

    let signal_result = tokio::select! {
        result = tokio::signal::ctrl_c() => {
            result.context("Failed to receive Ctrl+C")
        }
        result = engine.wait() => {
            return result;
        }
    };

    let stop_result = actions::stop(engine).await;

    signal_result?;
    stop_result
}
