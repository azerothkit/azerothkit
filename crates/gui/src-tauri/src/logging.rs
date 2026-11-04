use tauri::{Runtime, plugin::TauriPlugin};
use tauri_plugin_log::{FileOpenStrategy, RotationStrategy, Target, TargetKind, TimezoneStrategy};
use std::path::PathBuf;

fn human_target(kind: TargetKind) -> Target {
    Target::new(kind).format(|out, message, record| {
        out.finish(format_args!(
            "[{}][{}][{}] {}",
            TimezoneStrategy::UseUtc.get_now(),
            record.level(),
            record.target(),
            message,
        ));
    })
}

pub fn plugin<R: Runtime>(logs_dir: PathBuf) -> TauriPlugin<R> {
    tauri_plugin_log::Builder::new()
        .clear_format()
        .level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .max_file_size(5 * 1024 * 1024)
        // Keep five archived files in addition to the active file.
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .file_open_strategy(FileOpenStrategy::Append)
        .targets([
            human_target(TargetKind::Stdout),
            human_target(TargetKind::Folder {
                path: logs_dir,
                file_name: Some("app".to_owned()),
            }),
            Target::new(TargetKind::Webview).format(|out, message, record| {
                let timestamp = (TimezoneStrategy::UseUtc
                    .get_now()
                    .unix_timestamp_nanos() / 1_000_000) as i64;

                let payload = serde_json::json!({
                    "timestamp": timestamp,
                    "target": record.target(),
                    "message": message.to_string(),
                });

                out.finish(format_args!("{payload}"));
            }),
        ])
        .build()
}
