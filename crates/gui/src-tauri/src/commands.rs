use std::sync::Arc;

use azerothkit_core::{
    clients::{ClientDescriptor, ClientId, available_clients},
    settings::{AppSettings, ClientSettings, SettingsStore},
};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub clients: Vec<ClientDescriptor>,
    pub settings: AppSettings,
}

#[tauri::command]
#[specta::specta]
pub async fn get_settings_snapshot(
    store: State<'_, Arc<SettingsStore>>,
) -> Result<SettingsSnapshot, String> {
    let store = Arc::clone(store.inner());

    let settings = tauri::async_runtime::spawn_blocking(
        move || store.snapshot(),
    )
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

    tauri::async_runtime::spawn_blocking(move || {
        store.save_client_settings(client_id, settings)
    })
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

    tauri::async_runtime::spawn_blocking(move || {
        store.set_selected_client(client_id)
    })
        .await
        .map_err(|error| format!("Settings task failed: {error}"))?
        .map_err(|error| format!("{error:#}"))
}
