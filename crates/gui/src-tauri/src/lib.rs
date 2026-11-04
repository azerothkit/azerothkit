use std::sync::Arc;

use azerothkit_core::{
    app_paths::AppPaths,
    settings::SettingsStore,
};
use tauri::Manager;

mod commands;

fn command_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            commands::get_settings_snapshot,
            commands::save_client_settings,
            commands::set_selected_client,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

#[cfg(any(debug_assertions, test))]
fn export_bindings(
    builder: &tauri_specta::Builder<tauri::Wry>,
) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src/shared/api/rust/bindings.ts");

    std::fs::create_dir_all(
        path.parent().expect("Bindings path must have a parent"),
    )
        .expect("Failed to create bindings directory");

    builder
        .export(specta_typescript::Typescript::default(), &path)
        .expect("Failed to export TypeScript bindings");

    println!("TypeScript bindings exported to {}", path.display());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = command_builder();

    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let paths = AppPaths::discover()?;
            let store = SettingsStore::open(&paths)?;

            app.manage(Arc::new(store));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Failed to run Tauri application");
}

pub fn export_typescript_bindings() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src/shared/api/rust/bindings.ts");

    command_builder()
        .export(specta_typescript::Typescript::default(), &path)
        .expect("Failed to export TypeScript bindings");

    println!("TypeScript bindings exported to {}", path.display());
}
