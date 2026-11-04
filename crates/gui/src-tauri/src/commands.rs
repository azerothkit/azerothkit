use std::sync::Arc;

use azerothkit_core::{
    clients::{ClientDescriptor, ClientId, available_clients},
    settings::{AppSettings, ClientSettings, SettingsStore},
};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::engine::{EngineSnapshot, EngineState};
use azerothkit_core::app_paths::AppPaths;

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub clients: Vec<ClientDescriptor>,
    pub settings: AppSettings,
}

#[tauri::command]
#[specta::specta]
pub fn get_engine_snapshot(engine: State<'_, EngineState>) -> Result<EngineSnapshot, String> {
    engine.snapshot()
}

#[tauri::command]
#[specta::specta]
pub async fn start_engine(
    engine: State<'_, EngineState>,
) -> Result<EngineSnapshot, String> {
    engine.start().await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_engine(
    engine: State<'_, EngineState>,
) -> Result<EngineSnapshot, String> {
    engine.stop().await
}

#[tauri::command]
#[specta::specta]
pub async fn restart_engine(
    engine: State<'_, EngineState>,
) -> Result<EngineSnapshot, String> {
    engine.restart().await
}

#[tauri::command]
#[specta::specta]
pub async fn get_settings_snapshot(
    store: State<'_, Arc<SettingsStore>>,
) -> Result<SettingsSnapshot, String> {
    let store = Arc::clone(store.inner());

    let settings = tauri::async_runtime::spawn_blocking(move || store.snapshot())
        .await
        .map_err(|error| format!("Settings task failed: {error}"))?
        .map_err(|error| format!("{error:#}"))?;

    Ok(SettingsSnapshot {
        clients: available_clients().into_iter().collect(),
        settings,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn save_client_settings(
    store: State<'_, Arc<SettingsStore>>,
    client_id: ClientId,
    settings: ClientSettings,
) -> Result<AppSettings, String> {
    let store = Arc::clone(store.inner());

    tauri::async_runtime::spawn_blocking(move || store.save_client_settings(client_id, settings))
        .await
        .map_err(|error| format!("Settings task failed: {error}"))?
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
#[specta::specta]
pub async fn set_selected_client(
    store: State<'_, Arc<SettingsStore>>,
    client_id: ClientId,
) -> Result<AppSettings, String> {
    let store = Arc::clone(store.inner());

    tauri::async_runtime::spawn_blocking(move || store.set_selected_client(client_id))
        .await
        .map_err(|error| format!("Settings task failed: {error}"))?
        .map_err(|error| format!("{error:#}"))
}

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub logs_dir: String,
}

#[tauri::command]
#[specta::specta]
pub fn get_app_info(
    app: AppHandle,
    paths: State<'_, AppPaths>,
) -> AppInfo {
    let package = app.package_info();

    AppInfo {
        name: app
            .config()
            .product_name
            .clone()
            .unwrap_or_else(|| package.name.clone()),
        version: package.version.to_string(),
        logs_dir: paths.logs_dir().to_string_lossy().into_owned(),
    }
}