use serde::Serialize;

use super::protocol::{
    pressed_set, Frame, SplitStatus, Transport, FIELD_CENTRAL_BATTERY, FIELD_DEFAULT_LAYER,
    FIELD_ENDPOINT, FIELD_HID_INDICATORS, FIELD_PERIPHERAL_BATTERY, FIELD_SPLIT_STATUS,
};

pub const TELEMETRY_STATE_EVENT: &str = "telemetry-state";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EndpointTransport {
    Usb,
    Ble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PeripheralStatus {
    Disconnected,
    Connected,
}

/// Full authoritative keyboard state published on every valid telemetry frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryState {
    pub connection: ConnectionStatus,
    pub snapshot: bool,
    pub pressed: Vec<u8>,
    pub active_layers: u32,
    pub changed_fields: u32,
    pub valid_fields: u32,
    pub modifiers: u8,
    pub sequence: u32,
    pub timestamp_ms: u64,
    pub firmware_drops: u32,
    pub gaps: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hid_indicators: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_layer: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<EndpointTransport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ble_profile: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub central_battery_pct: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peripheral_battery_pct: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_status: Option<PeripheralStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl TelemetryState {
    pub fn disconnected(gaps: u64) -> Self {
        Self {
            connection: ConnectionStatus::Disconnected,
            snapshot: false,
            pressed: Vec::new(),
            active_layers: 1,
            changed_fields: 0,
            valid_fields: 0,
            modifiers: 0,
            sequence: 0,
            timestamp_ms: 0,
            firmware_drops: 0,
            gaps,
            hid_indicators: None,
            default_layer: None,
            transport: None,
            ble_profile: None,
            central_battery_pct: None,
            peripheral_battery_pct: None,
            split_status: None,
            error: None,
        }
    }

    pub fn from_frame(frame: &Frame, connection: ConnectionStatus, gaps: u64) -> Self {
        let endpoint_valid = frame.valid_fields & FIELD_ENDPOINT != 0;
        Self {
            connection,
            snapshot: frame.snapshot,
            pressed: pressed_set(frame.pressed_positions),
            active_layers: frame.active_layers,
            changed_fields: frame.changed_fields,
            valid_fields: frame.valid_fields,
            modifiers: frame.modifiers,
            sequence: frame.sequence,
            timestamp_ms: frame.timestamp_ms,
            firmware_drops: frame.dropped_frames,
            gaps,
            hid_indicators: (frame.valid_fields & FIELD_HID_INDICATORS != 0)
                .then_some(frame.hid_indicators),
            default_layer: (frame.valid_fields & FIELD_DEFAULT_LAYER != 0)
                .then_some(frame.default_layer),
            transport: endpoint_valid.then(|| match frame.transport {
                Transport::Usb => EndpointTransport::Usb,
                Transport::Ble => EndpointTransport::Ble,
                Transport::Unknown => unreachable!("validated endpoint transport"),
            }),
            ble_profile: (endpoint_valid && frame.transport == Transport::Ble)
                .then_some(frame.ble_profile),
            central_battery_pct: (frame.valid_fields & FIELD_CENTRAL_BATTERY != 0)
                .then_some(frame.central_battery_pct),
            peripheral_battery_pct: (frame.valid_fields & FIELD_PERIPHERAL_BATTERY != 0)
                .then_some(frame.peripheral_battery_pct),
            split_status: (frame.valid_fields & FIELD_SPLIT_STATUS != 0).then(|| {
                match frame.split_status {
                    SplitStatus::Disconnected => PeripheralStatus::Disconnected,
                    SplitStatus::Connected => PeripheralStatus::Connected,
                    SplitStatus::Unknown => unreachable!("validated split status"),
                }
            }),
            error: None,
        }
    }
}

/// Tracks non-snapshot sequence gaps modulo 2^32.
#[derive(Debug, Default)]
pub struct SequenceTracker {
    last_sequence: Option<u32>,
    gaps: u64,
}

impl SequenceTracker {
    pub fn observe(&mut self, sequence: u32, snapshot: bool) {
        if snapshot {
            self.last_sequence = Some(sequence);
            return;
        }
        if let Some(previous) = self.last_sequence {
            let difference = sequence.wrapping_sub(previous);
            if difference > 1 {
                self.gaps += (difference - 1) as u64;
            }
        }
        self.last_sequence = Some(sequence);
    }

    pub fn gaps(&self) -> u64 {
        self.gaps
    }

    pub fn reset(&mut self) {
        self.last_sequence = None;
        self.gaps = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::protocol::{
        SplitStatus, Transport, FIELD_CENTRAL_BATTERY, FIELD_DEFAULT_LAYER, FIELD_ENDPOINT,
        FIELD_HID_INDICATORS, FIELD_LAYERS, FIELD_MODIFIERS, FIELD_PERIPHERAL_BATTERY,
        FIELD_POSITIONS, FIELD_SPLIT_STATUS, VALUE_UNKNOWN,
    };

    const KNOWN_FIELDS: u32 = FIELD_POSITIONS
        | FIELD_LAYERS
        | FIELD_MODIFIERS
        | FIELD_HID_INDICATORS
        | FIELD_DEFAULT_LAYER
        | FIELD_ENDPOINT
        | FIELD_CENTRAL_BATTERY
        | FIELD_PERIPHERAL_BATTERY
        | FIELD_SPLIT_STATUS;

    fn frame(snapshot: bool, sequence: u32) -> Frame {
        Frame {
            snapshot,
            sequence,
            timestamp_ms: 123_456,
            pressed_positions: (1 << 3) | (1 << 15),
            active_layers: 9,
            changed_fields: FIELD_POSITIONS | FIELD_MODIFIERS,
            valid_fields: KNOWN_FIELDS,
            modifiers: 2,
            hid_indicators: 2,
            default_layer: 0,
            transport: Transport::Ble,
            ble_profile: 3,
            central_battery_pct: 90,
            peripheral_battery_pct: 80,
            split_status: SplitStatus::Connected,
            dropped_frames: 4,
        }
    }

    #[test]
    fn snapshot_establishes_baseline_without_gap() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(4_000_000_000, false);
        tracker.observe(7, true);
        tracker.observe(8, false);
        assert_eq!(tracker.gaps(), 0);
    }

    #[test]
    fn counts_u32_modular_gaps() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(u32::MAX - 1, false);
        tracker.observe(1, false);
        assert_eq!(tracker.gaps(), 2);
    }

    #[test]
    fn publishes_all_valid_fields() {
        let state = TelemetryState::from_frame(&frame(false, 8), ConnectionStatus::Connected, 3);
        assert_eq!(state.pressed, vec![3, 15]);
        assert_eq!(state.modifiers, 2);
        assert_eq!(state.transport, Some(EndpointTransport::Ble));
        assert_eq!(state.ble_profile, Some(3));
        assert_eq!(state.central_battery_pct, Some(90));
        assert_eq!(state.peripheral_battery_pct, Some(80));
        assert_eq!(state.split_status, Some(PeripheralStatus::Connected));
        assert_eq!(state.firmware_drops, 4);
        assert_eq!(state.gaps, 3);
    }

    #[test]
    fn invalid_optional_fields_become_none() {
        let mut input = frame(false, 1);
        input.valid_fields = FIELD_POSITIONS | FIELD_LAYERS | FIELD_MODIFIERS;
        input.default_layer = VALUE_UNKNOWN;
        input.transport = Transport::Unknown;
        input.ble_profile = VALUE_UNKNOWN;
        input.central_battery_pct = VALUE_UNKNOWN;
        input.peripheral_battery_pct = VALUE_UNKNOWN;
        input.split_status = SplitStatus::Unknown;
        let state = TelemetryState::from_frame(&input, ConnectionStatus::Connected, 0);
        assert_eq!(state.hid_indicators, None);
        assert_eq!(state.transport, None);
        assert_eq!(state.central_battery_pct, None);
        assert_eq!(state.split_status, None);
    }
}
