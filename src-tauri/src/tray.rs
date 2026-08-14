//! StatusNotifier tray: open settings, connection status, quit.
//! Overlay visibility toggling stays with the compositor (Hyprland).

use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Listener, Manager,
};

use crate::telemetry::state::TELEMETRY_STATE_EVENT;

pub fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let open_settings =
        MenuItem::with_id(app, "open_settings", "Open settings", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let status = MenuItem::with_id(app, "status", "disconnected", false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit =
        MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(|e| e.to_string())?;

    let menu = MenuBuilder::new(app)
        .items(&[&open_settings, &status, &quit])
        .build()
        .map_err(|e| e.to_string())?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| "no default icon".to_string())?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("kb-hud")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_settings" => open_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_settings_window(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    // Mirror connection status into the tray menu.
    let status_item = status.clone();
    app.listen_any(TELEMETRY_STATE_EVENT, move |event| {
        let payload = event.payload();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
            return;
        };
        let connection = value
            .get("connection")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let text = match value.get("error").and_then(|v| v.as_str()) {
            Some(error) => format!("{connection}: {error}"),
            None => connection.to_string(),
        };
        let _ = status_item.set_text(text);
    });

    Ok(())
}

fn open_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
