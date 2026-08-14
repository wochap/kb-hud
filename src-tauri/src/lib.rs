mod ble;
mod config;
mod mock;
mod portable;
mod telemetry;
mod tray;

use std::sync::Mutex;

use config::{Appearance, AppearancePatch, ConfigStore, Profile, ProfilePatch, AUTO_DEVICE};
use mock::MockSource;
use tauri::{AppHandle, Emitter, Manager, State};
use telemetry::hub::{SharedHub, TelemetryHub};

use crate::ble::{BleController, PairedDevice};

type ConfigState = Mutex<ConfigStore>;

pub const PROFILE_CHANGED_EVENT: &str = "profile-changed";
pub const CONFIG_CHANGED_EVENT: &str = "config-changed";

fn emit_profile_changed(app: &AppHandle, profile_name: &str) {
    let _ = app.emit(PROFILE_CHANGED_EVENT, profile_name);
}

fn emit_config_changed(app: &AppHandle) {
    let _ = app.emit(CONFIG_CHANGED_EVENT, ());
}

#[tauri::command]
fn list_profiles(state: State<'_, ConfigState>) -> Result<Vec<Profile>, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    Ok(store.config().profiles.clone())
}

#[tauri::command]
fn get_active_profile(state: State<'_, ConfigState>) -> Result<Profile, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    store
        .active_profile()
        .cloned()
        .ok_or_else(|| "no active profile".to_string())
}

#[tauri::command]
fn create_profile(state: State<'_, ConfigState>, name: String) -> Result<Profile, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    store.create_profile(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_profile(
    state: State<'_, ConfigState>,
    name: String,
    new_name: String,
) -> Result<(), String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    store
        .rename_profile(&name, &new_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_profile(state: State<'_, ConfigState>, name: String) -> Result<(), String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    store.delete_profile(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_active_profile(
    app: AppHandle,
    state: State<'_, ConfigState>,
    name: String,
) -> Result<(), String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    store.set_active(&name).map_err(|e| e.to_string())?;
    emit_profile_changed(&app, &name);
    Ok(())
}

#[tauri::command]
fn update_profile(
    app: AppHandle,
    state: State<'_, ConfigState>,
    name: String,
    patch: ProfilePatch,
) -> Result<Profile, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let updated = store
        .update_profile(&name, patch)
        .map_err(|e| e.to_string())?;
    emit_profile_changed(&app, &updated.name);
    Ok(updated)
}

#[tauri::command]
fn get_global_appearance(state: State<'_, ConfigState>) -> Result<Appearance, String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    Ok(store.appearance().clone())
}

#[tauri::command]
fn update_global_appearance(
    app: AppHandle,
    state: State<'_, ConfigState>,
    patch: AppearancePatch,
) -> Result<Appearance, String> {
    let mut store = state.lock().map_err(|e| e.to_string())?;
    let updated = store.update_appearance(patch).map_err(|e| e.to_string())?;
    emit_config_changed(&app);
    Ok(updated)
}

/// Reads a keymap SVG from disk so the webview can parse it with DOMParser.
#[tauri::command]
fn read_keymap_svg(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("failed to read {path}: {e}"))
}

/// Builds a portable export and writes it atomically to the chosen path.
/// The path is selected by the frontend via the save dialog.
#[tauri::command]
fn export_configuration(state: State<'_, ConfigState>, path: String) -> Result<(), String> {
    let store = state.lock().map_err(|e| e.to_string())?;
    let export = portable::build_export(store.config()).map_err(|e| e.to_string())?;
    portable::write_export(std::path::Path::new(&path), &export).map_err(|e| e.to_string())
}

/// Parses and validates a portable export, returning a replacement summary
/// without mutating current state. Used to preview an import before confirming.
#[tauri::command]
fn inspect_import(path: String) -> Result<portable::ImportSummary, String> {
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("failed to read {path}: {e}"))?;
    portable::inspect_import(&raw).map_err(|e| e.to_string())
}

/// Confirmed replace-all import. Re-reads and re-validates the document, stages
/// managed keymaps, atomically replaces the configuration last, assigns `auto`
/// to every device, notifies both windows, and reconnects BLE via discovery.
#[tauri::command]
fn commit_import(
    app: AppHandle,
    state: State<'_, ConfigState>,
    controller: State<'_, BleController>,
    path: String,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("failed to read {path}: {e}"))?;
    let export = portable::parse_import(&raw).map_err(|e| e.to_string())?;
    portable::validate_export(&export).map_err(|e| e.to_string())?;

    let mut store = state.lock().map_err(|e| e.to_string())?;
    let config_dir = store
        .config_dir()
        .ok_or_else(|| "config dir unavailable".to_string())?
        .to_path_buf();
    let keymap_dir = config_dir.join("keymaps");

    // Stage managed keymaps before touching the configuration so a failure
    // here leaves the active configuration unchanged.
    let keymap_paths =
        portable::stage_imported_keymaps(&export, &keymap_dir).map_err(|e| e.to_string())?;
    let new_config = portable::build_imported_config(&export, &keymap_paths);

    // Commit the replacement atomically as the final step.
    store.replace_all(new_config).map_err(|e| e.to_string())?;
    let active_name = export.active_profile.clone();
    drop(store);

    emit_config_changed(&app);
    emit_profile_changed(&app, &active_name);
    controller.start(AUTO_DEVICE);
    Ok(())
}

#[tauri::command]
async fn list_bluetooth_devices() -> Result<Vec<PairedDevice>, String> {
    ble::list_paired_devices().await
}

#[tauri::command]
fn ble_reconnect(
    config_state: State<'_, ConfigState>,
    controller: State<'_, BleController>,
) -> Result<(), String> {
    let store = config_state.lock().map_err(|e| e.to_string())?;
    let profile = store
        .active_profile()
        .ok_or_else(|| "no active profile".to_string())?;
    controller.start(&profile.device_mac);
    Ok(())
}

#[tauri::command]
fn mock_press(mock: State<'_, MockSource>, position: u8) -> Result<(), String> {
    mock.press(position)
}

#[tauri::command]
fn mock_release(mock: State<'_, MockSource>, position: u8) -> Result<(), String> {
    mock.release(position)
}

#[tauri::command]
fn mock_burst(mock: State<'_, MockSource>, count: u32) -> Result<(), String> {
    mock.burst(count.min(64));
    Ok(())
}

#[tauri::command]
fn mock_hold_layer(mock: State<'_, MockSource>, layer: u8) -> Result<(), String> {
    mock.hold_layer(layer)
}

#[tauri::command]
fn mock_release_layer(mock: State<'_, MockSource>, layer: u8) -> Result<(), String> {
    mock.release_layer(layer)
}

#[tauri::command]
fn mock_set_modifier(mock: State<'_, MockSource>, bit: u8, active: bool) -> Result<(), String> {
    mock.set_modifier(bit, active)
}

#[tauri::command]
fn mock_set_demo_status(mock: State<'_, MockSource>, enabled: bool) -> Result<(), String> {
    mock.set_demo_status(enabled);
    Ok(())
}

#[tauri::command]
fn mock_inject_gap(mock: State<'_, MockSource>) -> Result<(), String> {
    mock.inject_gap();
    Ok(())
}

#[tauri::command]
fn mock_inject_firmware_drop(mock: State<'_, MockSource>) -> Result<(), String> {
    mock.inject_firmware_drop();
    Ok(())
}

#[tauri::command]
fn mock_disconnect(mock: State<'_, MockSource>) -> Result<(), String> {
    mock.disconnect();
    Ok(())
}

#[tauri::command]
fn mock_reconnect(mock: State<'_, MockSource>) -> Result<(), String> {
    mock.reconnect();
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|e| format!("config dir unavailable: {e}"))?;
            let store = ConfigStore::load_or_create(&config_dir)
                .map_err(|e| format!("failed to load profiles: {e}"))?;
            let device_mac = store
                .active_profile()
                .map(|p| p.device_mac.clone())
                .unwrap_or_else(|| config::AUTO_DEVICE.to_string());
            app.manage(ConfigState::new(store));

            let hub: SharedHub = TelemetryHub::shared(app.handle().clone());
            let controller = BleController::spawn(hub.clone());
            // Launch behavior: auto-connect to the active profile's device.
            controller.start(&device_mac);
            app.manage(controller);
            app.manage(MockSource::new(hub));

            // Tray is best-effort: without a StatusNotifier host/bus (or the
            // appindicator library itself) the app stays fully usable
            // through its windows.
            let tray_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tray::setup_tray(app.handle())
            }));
            match tray_result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => eprintln!("tray unavailable: {e}"),
                Err(_) => eprintln!("tray unavailable: appindicator library missing"),
            }

            // Dev aid / tray-less environments: open settings on launch.
            if std::env::var_os("KB_HUD_OPEN_SETTINGS").is_some() {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            get_active_profile,
            create_profile,
            rename_profile,
            delete_profile,
            set_active_profile,
            update_profile,
            get_global_appearance,
            update_global_appearance,
            read_keymap_svg,
            export_configuration,
            inspect_import,
            commit_import,
            list_bluetooth_devices,
            ble_reconnect,
            mock_press,
            mock_release,
            mock_burst,
            mock_hold_layer,
            mock_release_layer,
            mock_set_modifier,
            mock_set_demo_status,
            mock_inject_gap,
            mock_inject_firmware_drop,
            mock_disconnect,
            mock_reconnect
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
