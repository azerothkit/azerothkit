use std::sync::Arc;

use azerothkit_core::{app_paths::AppPaths, settings::SettingsStore};
use tauri::Manager;

mod commands;
mod engine;
mod logging;

fn command_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            commands::get_engine_snapshot,
            commands::start_engine,
            commands::stop_engine,
            commands::restart_engine,
            commands::get_settings_snapshot,
            commands::save_client_settings,
            commands::set_selected_client,
            commands::get_app_info,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

fn export_bindings(builder: &tauri_specta::Builder<tauri::Wry>) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/shared/api/rust/bindings.ts");

    std::fs::create_dir_all(path.parent().expect("Bindings path must have a parent"))
        .expect("Failed to create bindings directory");

    builder
        .export(specta_typescript::Typescript::default(), &path)
        .expect("Failed to export TypeScript bindings");

    println!("TypeScript bindings exported to {}", path.display());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = command_builder();
    let paths = AppPaths::discover().expect("Failed to determine application directories");

    let app_builder =
        tauri::Builder::default().plugin(tauri_plugin_window_state::Builder::new().build());

    #[cfg(desktop)]
    let app_builder = app_builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }));

    app_builder
        .plugin(logging::plugin(paths.logs_dir()))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let store = SettingsStore::open(&paths)?;
            let engine = engine::EngineState::new(app.handle().clone(), paths.clone());

            app.manage(paths);
            app.manage(Arc::new(store));
            app.manage(engine);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Failed to run Tauri application");
}

pub fn export_typescript_bindings() {
    export_bindings(&command_builder());
}
