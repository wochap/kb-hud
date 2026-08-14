//! BLE telemetry source: BlueZ GATT via the `bluer` crate.

use std::str::FromStr;
use std::time::Duration;

use bluer::{Adapter, Address, DeviceEvent, DeviceProperty, Session};
use futures_util::StreamExt;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::config::AUTO_DEVICE;
use crate::telemetry::hub::SharedHub;
use crate::telemetry::protocol::{self};
use crate::telemetry::state::ConnectionStatus;

pub const SERVICE_UUID: &str = "9e7a7d70-df1b-4f76-9d45-8c3f4a6b2100";
pub const CHARACTERISTIC_UUID: &str = "9e7a7d70-df1b-4f76-9d45-8c3f4a6b2101";
pub const DEFAULT_ALIAS: &str = "Chocochap";

const BACKOFF_INITIAL: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub name: String,
    pub address: String,
}

pub enum BleCommand {
    Start(String),
    Stop,
}

pub struct BleController {
    tx: UnboundedSender<BleCommand>,
}

impl BleController {
    pub fn spawn(hub: SharedHub) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tauri::async_runtime::spawn(controller_loop(rx, hub));
        Self { tx }
    }

    pub fn start(&self, device_mac: &str) {
        let _ = self.tx.send(BleCommand::Start(device_mac.to_string()));
    }

    #[allow(dead_code)]
    pub fn stop(&self) {
        let _ = self.tx.send(BleCommand::Stop);
    }
}

async fn controller_loop(mut rx: UnboundedReceiver<BleCommand>, hub: SharedHub) {
    let mut target: Option<String> = None;
    let mut backoff = BACKOFF_INITIAL;

    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                BleCommand::Start(mac) => {
                    target = Some(mac);
                    backoff = BACKOFF_INITIAL;
                }
                BleCommand::Stop => {
                    target = None;
                    hub.lock()
                        .unwrap()
                        .publish_connection(ConnectionStatus::Disconnected, None);
                }
            }
        }

        let Some(device_mac) = target.clone() else {
            match rx.recv().await {
                Some(cmd) => {
                    if let BleCommand::Start(mac) = cmd {
                        target = Some(mac);
                    }
                }
                None => return,
            }
            continue;
        };

        hub.lock()
            .unwrap()
            .publish_connection(ConnectionStatus::Connecting, None);

        match connection_cycle(&hub, &device_mac).await {
            Outcome::Fatal(message) => {
                hub.lock()
                    .unwrap()
                    .publish_connection(ConnectionStatus::Disconnected, Some(message));
                target = None;
            }
            Outcome::Ended(message) => {
                hub.lock()
                    .unwrap()
                    .publish_connection(ConnectionStatus::Disconnected, message);
                // Sleep with backoff, but wake immediately on new commands.
                tokio::select! {
                    _ = tokio::time::sleep(backoff) => {}
                    cmd = rx.recv() => {
                        match cmd {
                            Some(BleCommand::Start(mac)) => {
                                target = Some(mac);
                                backoff = BACKOFF_INITIAL;
                            }
                            Some(BleCommand::Stop) => target = None,
                            None => return,
                        }
                        continue;
                    }
                }
                backoff = (backoff * 2).min(BACKOFF_MAX);
            }
            Outcome::SessionClosed => {
                hub.lock()
                    .unwrap()
                    .publish_connection(ConnectionStatus::Disconnected, None);
                backoff = BACKOFF_INITIAL;
            }
        }
    }
}

enum Outcome {
    /// Unrecoverable until the user reconnects manually (unsupported device,
    /// ambiguous auto-detect, invalid MAC).
    Fatal(String),
    /// Failed but worth retrying with backoff.
    Ended(Option<String>),
    /// Was connected; device disconnected (sleep, out of range). Retry soon.
    SessionClosed,
}

async fn connection_cycle(hub: &SharedHub, device_mac: &str) -> Outcome {
    let (adapter, address) = match resolve_device(device_mac).await {
        Ok(pair) => pair,
        Err(message) => return message,
    };

    let device = match adapter.device(address) {
        Ok(device) => device,
        Err(e) => return Outcome::Ended(Some(e.to_string())),
    };

    if let Err(e) = device.connect().await {
        return Outcome::Ended(Some(format!("connect failed: {e}")));
    }

    let services = match device.services().await {
        Ok(services) => services,
        Err(e) => return Outcome::Ended(Some(format!("service discovery failed: {e}"))),
    };
    let mut telemetry_service = None;
    for service in &services {
        if let Ok(uuid) = service.uuid().await {
            if uuid.to_string() == SERVICE_UUID {
                telemetry_service = Some(service);
                break;
            }
        }
    }
    let Some(service) = telemetry_service else {
        let _ = device.disconnect().await;
        return Outcome::Fatal(
            "unsupported device: telemetry service not found".to_string(),
        );
    };

    let characteristics = match service.characteristics().await {
        Ok(chars) => chars,
        Err(e) => return Outcome::Ended(Some(format!("characteristic discovery failed: {e}"))),
    };
    let mut telemetry_characteristic = None;
    for characteristic in &characteristics {
        if let Ok(uuid) = characteristic.uuid().await {
            if uuid.to_string() == CHARACTERISTIC_UUID {
                telemetry_characteristic = Some(characteristic);
                break;
            }
        }
    }
    let Some(characteristic) = telemetry_characteristic else {
        let _ = device.disconnect().await;
        return Outcome::Fatal(
            "unsupported device: telemetry characteristic not found".to_string(),
        );
    };

    // Initial read gives the current snapshot before notifications begin.
    let snapshot = match characteristic.read().await {
        Ok(value) => value,
        Err(e) => return Outcome::Ended(Some(format!("initial read failed: {e}"))),
    };

    let mut notify = match characteristic.notify().await {
        Ok(stream) => Box::pin(stream),
        Err(e) => return Outcome::Ended(Some(format!("notify subscription failed: {e}"))),
    };

    {
        let mut hub = hub.lock().unwrap();
        hub.reset_sequence_tracking();
        feed(&mut hub, &snapshot);
        hub.publish_connection(ConnectionStatus::Connected, None);
    }

    let mut events = match device.events().await {
        Ok(events) => Box::pin(events),
        Err(e) => return Outcome::Ended(Some(format!("device events unavailable: {e}"))),
    };

    loop {
        tokio::select! {
            value = notify.next() => {
                match value {
                    Some(bytes) => feed(&mut hub.lock().unwrap(), &bytes),
                    None => break,
                }
            }
            event = events.next() => {
                match event {
                    Some(DeviceEvent::PropertyChanged(DeviceProperty::Connected(false)))
                    | None => break,
                    Some(_) => {}
                }
            }
        }
    }

    let _ = device.disconnect().await;
    Outcome::SessionClosed
}

/// Feeds one raw record through validation + decode + publication.
fn feed(hub: &mut crate::telemetry::hub::TelemetryHub, bytes: &[u8]) {
    match protocol::decode(bytes) {
        Ok(record) => hub.publish_record(&record),
        Err(e) => hub.publish_connection(ConnectionStatus::Connected, Some(e.to_string())),
    }
}

/// Enumerates paired devices for the settings device picker.
pub async fn list_paired_devices() -> Result<Vec<PairedDevice>, String> {
    let session = Session::new()
        .await
        .map_err(|e| format!("no system Bluetooth bus: {e}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|e| format!("no default Bluetooth adapter: {e}"))?;
    let mut devices = Vec::new();
    for address in adapter
        .device_addresses()
        .await
        .map_err(|e| e.to_string())?
    {
        let device = adapter.device(address).map_err(|e| e.to_string())?;
        if device.is_paired().await.unwrap_or(false) {
            let name = device.alias().await.unwrap_or_default();
            devices.push(PairedDevice {
                name,
                address: address.to_string(),
            });
        }
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(devices)
}

/// Resolves the profile device selection to a concrete adapter + address.
/// Returns an Outcome so fatal auto-detect errors stop the retry loop.
async fn resolve_device(device_mac: &str) -> Result<(Adapter, Address), Outcome> {
    let session = Session::new().await.map_err(|e| {
        Outcome::Ended(Some(format!("no system Bluetooth bus: {e}")))
    })?;
    let adapter = session.default_adapter().await.map_err(|e| {
        Outcome::Ended(Some(format!("no default Bluetooth adapter: {e}")))
    })?;

    if device_mac != AUTO_DEVICE {
        let address = Address::from_str(device_mac).map_err(|_| {
            Outcome::Fatal(format!("invalid Bluetooth MAC address: {device_mac}"))
        })?;
        return Ok((adapter, address));
    }

    let mut matches = Vec::new();
    let addresses = adapter
        .device_addresses()
        .await
        .map_err(|e| Outcome::Ended(Some(e.to_string())))?;
    for address in addresses {
        let Ok(device) = adapter.device(address) else {
            continue;
        };
        if !device.is_paired().await.unwrap_or(false) {
            continue;
        }
        if let Ok(alias) = device.alias().await {
            if alias == DEFAULT_ALIAS {
                matches.push(address);
            }
        }
    }

    match matches.len() {
        0 => Err(Outcome::Ended(Some(
            "no telemetry keyboard found: no paired device named 'Chocochap'".to_string(),
        ))),
        1 => Ok((adapter, matches[0])),
        _ => Err(Outcome::Fatal(format!(
            "multiple paired devices named '{DEFAULT_ALIAS}' ({}); set an explicit MAC in the profile",
            matches
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}
