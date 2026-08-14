//! BLE telemetry source: BlueZ GATT via the `bluer` crate.

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use bluer::{Adapter, Address, Device, DeviceEvent, DeviceProperty, Session};
use futures_util::StreamExt;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::config::AUTO_DEVICE;
use crate::telemetry::hub::SharedHub;
use crate::telemetry::protocol::{self};
use crate::telemetry::state::ConnectionStatus;

pub const SERVICE_UUID: &str = "9e7a7d70-df1b-4f76-9d45-8c3f4a6b2100";
pub const CHARACTERISTIC_UUID: &str = "9e7a7d70-df1b-4f76-9d45-8c3f4a6b2101";

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
    let Some(service) = find_service_by_uuid(&services, SERVICE_UUID).await else {
        let _ = device.disconnect().await;
        return Outcome::Fatal("unsupported device: telemetry service not found".to_string());
    };

    let characteristics = match service.characteristics().await {
        Ok(chars) => chars,
        Err(e) => return Outcome::Ended(Some(format!("characteristic discovery failed: {e}"))),
    };
    let Some(characteristic) =
        find_characteristic_by_uuid(&characteristics, CHARACTERISTIC_UUID).await
    else {
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
    let initial_frame = match protocol::decode(&snapshot) {
        Ok(frame) if frame.snapshot => frame,
        Ok(_) => {
            let _ = device.disconnect().await;
            return Outcome::Fatal(
                "incompatible telemetry: initial read is not a snapshot frame".to_string(),
            );
        }
        Err(e) => {
            let _ = device.disconnect().await;
            return Outcome::Fatal(format!("incompatible telemetry: {e}"));
        }
    };

    let mut notify = match characteristic.notify().await {
        Ok(stream) => Box::pin(stream),
        Err(e) => return Outcome::Ended(Some(format!("notify subscription failed: {e}"))),
    };

    {
        let mut hub = hub.lock().unwrap();
        hub.reset_sequence_tracking();
        hub.publish_frame(&initial_frame);
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
        Ok(frame) => hub.publish_frame(&frame),
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
    let selection = parse_device_selection(device_mac).map_err(Outcome::Fatal)?;

    let session = Session::new()
        .await
        .map_err(|e| Outcome::Ended(Some(format!("no system Bluetooth bus: {e}"))))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|e| Outcome::Ended(Some(format!("no default Bluetooth adapter: {e}"))))?;

    match selection {
        DeviceSelection::Explicit(address) => Ok((adapter, address)),
        DeviceSelection::Auto => auto_resolve(&adapter).await,
    }
}

/// Interprets the profile device value without any BlueZ I/O.
/// Explicit MAC addresses bypass candidate selection entirely.
fn parse_device_selection(device_mac: &str) -> Result<DeviceSelection, String> {
    if device_mac == AUTO_DEVICE {
        return Ok(DeviceSelection::Auto);
    }
    Address::from_str(device_mac)
        .map(DeviceSelection::Explicit)
        .map_err(|_| format!("invalid Bluetooth MAC address: {device_mac}"))
}

#[derive(Debug, PartialEq, Eq)]
enum DeviceSelection {
    Explicit(Address),
    Auto,
}

/// Enumerates paired devices, classifies them by cached service UUID
/// metadata, probes the unknown ones, and selects the unique compatible
/// telemetry keyboard.
async fn auto_resolve(adapter: &Adapter) -> Result<(Adapter, Address), Outcome> {
    let mut candidates = Vec::new();
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
        let alias = device.alias().await.unwrap_or_default();
        let cached_service_uuids = match device.uuids().await {
            Ok(Some(uuids)) => Some(uuids.iter().map(|uuid| uuid.to_string()).collect()),
            _ => None,
        };
        candidates.push(Candidate {
            address: address.to_string(),
            alias,
            cached_service_uuids,
        });
    }

    let mut probes = HashMap::new();
    for candidate in &candidates {
        if cached_service_match(&candidate.cached_service_uuids).is_some() {
            continue;
        }
        let Ok(address) = Address::from_str(&candidate.address) else {
            continue;
        };
        let Ok(device) = adapter.device(address) else {
            continue;
        };
        let outcome = probe_candidate(&device).await;
        probes.insert(candidate.address.clone(), outcome);
    }

    match select_auto_candidate(&candidates, &probes) {
        AutoSelection::Selected(address) => {
            let address = Address::from_str(&address)
                .map_err(|e| Outcome::Ended(Some(format!("invalid candidate address: {e}"))))?;
            Ok((adapter.clone(), address))
        }
        AutoSelection::Ambiguous(addresses) => {
            let listed = addresses
                .iter()
                .map(|address| {
                    let alias = candidates
                        .iter()
                        .find(|candidate| &candidate.address == address)
                        .map(|candidate| candidate.alias.as_str())
                        .unwrap_or_default();
                    if alias.is_empty() {
                        address.clone()
                    } else {
                        format!("{alias} ({address})")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            Err(Outcome::Fatal(format!(
                "multiple compatible telemetry keyboards found ({listed}); set an explicit MAC in the profile"
            )))
        }
        AutoSelection::NoneFound => Err(Outcome::Ended(Some(
            "no telemetry keyboard found: no paired device exposes the zmk-key-telemetry service"
                .to_string(),
        ))),
        AutoSelection::Unresolved => Err(Outcome::Ended(Some(
            "no telemetry keyboard confirmed yet: some paired devices could not be probed"
                .to_string(),
        ))),
    }
}

/// One paired device considered during auto-discovery.
#[derive(Debug, Clone)]
struct Candidate {
    address: String,
    alias: String,
    /// Cached `Device1.UUIDs` service metadata when BlueZ has resolved it.
    cached_service_uuids: Option<Vec<String>>,
}

/// Result of connecting to a candidate and inspecting its GATT table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeOutcome {
    /// Connected and confirmed both telemetry UUIDs.
    Compatible,
    /// Connected and confirmed the telemetry GATT interface is absent.
    Incompatible,
    /// Temporary connection or service-resolution failure.
    Failed,
}

/// Resolution result for `deviceMac: "auto"`.
#[derive(Debug, PartialEq, Eq)]
enum AutoSelection {
    Selected(String),
    Ambiguous(Vec<String>),
    NoneFound,
    /// No compatible device confirmed; at least one candidate unresolved.
    Unresolved,
}

/// Reports cached service UUID metadata as confirming compatibility,
/// denying it, or insufficient (`None` means probing is required).
fn cached_service_match(cached_service_uuids: &Option<Vec<String>>) -> Option<bool> {
    cached_service_uuids.as_ref().map(|uuids| {
        uuids
            .iter()
            .any(|uuid| uuid.eq_ignore_ascii_case(SERVICE_UUID))
    })
}

/// Selects the auto-discovery target from paired candidates and probe results.
/// Aliases never influence selection; only the telemetry service UUID does.
fn select_auto_candidate(
    candidates: &[Candidate],
    probes: &HashMap<String, ProbeOutcome>,
) -> AutoSelection {
    let mut compatible = Vec::new();
    let mut unresolved = false;

    for candidate in candidates {
        match cached_service_match(&candidate.cached_service_uuids) {
            Some(true) => compatible.push(candidate.address.clone()),
            Some(false) => {}
            None => match probes.get(&candidate.address) {
                Some(ProbeOutcome::Compatible) => compatible.push(candidate.address.clone()),
                Some(ProbeOutcome::Incompatible) => {}
                Some(ProbeOutcome::Failed) | None => unresolved = true,
            },
        }
    }

    match compatible.len() {
        0 if unresolved => AutoSelection::Unresolved,
        0 => AutoSelection::NoneFound,
        1 => AutoSelection::Selected(compatible.remove(0)),
        _ => AutoSelection::Ambiguous(compatible),
    }
}

/// Connects to a paired candidate and inspects its GATT table for the
/// telemetry interface. Always disconnects afterwards so the normal
/// connection cycle owns the connection lifecycle.
async fn probe_candidate(device: &Device) -> ProbeOutcome {
    let outcome = probe_gatt(device).await;
    let _ = device.disconnect().await;
    outcome
}

async fn probe_gatt(device: &Device) -> ProbeOutcome {
    if device.connect().await.is_err() {
        return ProbeOutcome::Failed;
    }
    let Ok(services) = device.services().await else {
        return ProbeOutcome::Failed;
    };
    let Some(service) = find_service_by_uuid(&services, SERVICE_UUID).await else {
        return ProbeOutcome::Incompatible;
    };
    let Ok(characteristics) = service.characteristics().await else {
        return ProbeOutcome::Failed;
    };
    match find_characteristic_by_uuid(&characteristics, CHARACTERISTIC_UUID).await {
        Some(_) => ProbeOutcome::Compatible,
        None => ProbeOutcome::Incompatible,
    }
}

/// Finds the first service exposing the given UUID. Shared by connection
/// validation and discovery probes so compatibility cannot drift.
async fn find_service_by_uuid<'a>(
    services: &'a [bluer::gatt::remote::Service],
    uuid: &str,
) -> Option<&'a bluer::gatt::remote::Service> {
    for service in services {
        if let Ok(service_uuid) = service.uuid().await {
            if service_uuid.to_string().eq_ignore_ascii_case(uuid) {
                return Some(service);
            }
        }
    }
    None
}

/// Finds the first characteristic exposing the given UUID. Shared by
/// connection validation and discovery probes so compatibility cannot drift.
async fn find_characteristic_by_uuid<'a>(
    characteristics: &'a [bluer::gatt::remote::Characteristic],
    uuid: &str,
) -> Option<&'a bluer::gatt::remote::Characteristic> {
    for characteristic in characteristics {
        if let Ok(characteristic_uuid) = characteristic.uuid().await {
            if characteristic_uuid.to_string().eq_ignore_ascii_case(uuid) {
                return Some(characteristic);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(address: &str, alias: &str, cached_service_uuids: Option<Vec<&str>>) -> Candidate {
        Candidate {
            address: address.to_string(),
            alias: alias.to_string(),
            cached_service_uuids: cached_service_uuids
                .map(|uuids| uuids.into_iter().map(String::from).collect()),
        }
    }

    fn probes(entries: &[(&str, ProbeOutcome)]) -> HashMap<String, ProbeOutcome> {
        entries
            .iter()
            .map(|(address, outcome)| (address.to_string(), *outcome))
            .collect()
    }

    #[test]
    fn unique_cached_service_match_is_selected() {
        let candidates = vec![
            candidate("AA:BB:CC:DD:EE:01", "keyboard", Some(vec![SERVICE_UUID])),
            candidate(
                "AA:BB:CC:DD:EE:02",
                "headphones",
                Some(vec!["0000110b-0000-1000-8000-00805f9b34fb"]),
            ),
        ];
        let selection = select_auto_candidate(&candidates, &probes(&[]));
        assert_eq!(
            selection,
            AutoSelection::Selected("AA:BB:CC:DD:EE:01".to_string())
        );
    }

    #[test]
    fn missing_cached_metadata_uses_probe_result() {
        let candidates = vec![candidate("AA:BB:CC:DD:EE:01", "keyboard", None)];
        let selection = select_auto_candidate(
            &candidates,
            &probes(&[("AA:BB:CC:DD:EE:01", ProbeOutcome::Compatible)]),
        );
        assert_eq!(
            selection,
            AutoSelection::Selected("AA:BB:CC:DD:EE:01".to_string())
        );
    }

    #[test]
    fn aliases_are_not_a_selection_signal() {
        let candidates = vec![
            candidate("AA:BB:CC:DD:EE:01", "Chocochap", None),
            candidate("AA:BB:CC:DD:EE:02", "Totally Different Name", None),
        ];
        let selection = select_auto_candidate(
            &candidates,
            &probes(&[
                ("AA:BB:CC:DD:EE:01", ProbeOutcome::Incompatible),
                ("AA:BB:CC:DD:EE:02", ProbeOutcome::Compatible),
            ]),
        );
        assert_eq!(
            selection,
            AutoSelection::Selected("AA:BB:CC:DD:EE:02".to_string())
        );
    }

    #[test]
    fn no_compatible_devices_is_definitive() {
        let candidates = vec![
            candidate("AA:BB:CC:DD:EE:01", "a", None),
            candidate(
                "AA:BB:CC:DD:EE:02",
                "b",
                Some(vec!["0000110b-0000-1000-8000-00805f9b34fb"]),
            ),
        ];
        let selection = select_auto_candidate(
            &candidates,
            &probes(&[("AA:BB:CC:DD:EE:01", ProbeOutcome::Incompatible)]),
        );
        assert_eq!(selection, AutoSelection::NoneFound);
    }

    #[test]
    fn cached_metadata_without_service_is_excluded_without_probing() {
        let candidates = vec![candidate(
            "AA:BB:CC:DD:EE:01",
            "speaker",
            Some(vec!["0000110b-0000-1000-8000-00805f9b34fb"]),
        )];
        let selection = select_auto_candidate(&candidates, &probes(&[]));
        assert_eq!(selection, AutoSelection::NoneFound);
    }

    #[test]
    fn multiple_compatible_devices_are_ambiguous() {
        let candidates = vec![
            candidate("AA:BB:CC:DD:EE:01", "left", Some(vec![SERVICE_UUID])),
            candidate("AA:BB:CC:DD:EE:02", "right", None),
            candidate("AA:BB:CC:DD:EE:03", "mouse", None),
        ];
        let selection = select_auto_candidate(
            &candidates,
            &probes(&[
                ("AA:BB:CC:DD:EE:02", ProbeOutcome::Compatible),
                ("AA:BB:CC:DD:EE:03", ProbeOutcome::Incompatible),
            ]),
        );
        assert_eq!(
            selection,
            AutoSelection::Ambiguous(vec![
                "AA:BB:CC:DD:EE:01".to_string(),
                "AA:BB:CC:DD:EE:02".to_string(),
            ])
        );
    }

    #[test]
    fn temporary_probe_failures_are_retryable() {
        let candidates = vec![
            candidate("AA:BB:CC:DD:EE:01", "a", None),
            candidate("AA:BB:CC:DD:EE:02", "b", None),
        ];
        let selection = select_auto_candidate(
            &candidates,
            &probes(&[
                ("AA:BB:CC:DD:EE:01", ProbeOutcome::Incompatible),
                ("AA:BB:CC:DD:EE:02", ProbeOutcome::Failed),
            ]),
        );
        assert_eq!(selection, AutoSelection::Unresolved);
    }

    #[test]
    fn explicit_mac_bypasses_candidate_selection() {
        assert_eq!(
            parse_device_selection("AA:BB:CC:DD:EE:FF"),
            Ok(DeviceSelection::Explicit(
                Address::from_str("AA:BB:CC:DD:EE:FF").unwrap()
            ))
        );
        assert!(matches!(
            parse_device_selection("auto"),
            Ok(DeviceSelection::Auto)
        ));
        assert!(parse_device_selection("not-a-mac").is_err());
    }
}
