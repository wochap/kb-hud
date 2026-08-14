//! Mock telemetry source. Every synthetic state is encoded as a protocol-v2
//! frame, decoded by the production parser, and published through the BLE hub.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::telemetry::hub::SharedHub;
use crate::telemetry::protocol::{
    self, Frame, SplitStatus, Transport, FIELD_CENTRAL_BATTERY, FIELD_DEFAULT_LAYER,
    FIELD_ENDPOINT, FIELD_HID_INDICATORS, FIELD_LAYERS, FIELD_MODIFIERS, FIELD_PERIPHERAL_BATTERY,
    FIELD_POSITIONS, FIELD_SPLIT_STATUS, POSITION_COUNT,
};
use crate::telemetry::state::ConnectionStatus;

const BASE_VALID_FIELDS: u32 =
    FIELD_POSITIONS | FIELD_LAYERS | FIELD_MODIFIERS | FIELD_DEFAULT_LAYER | FIELD_ENDPOINT;

#[derive(Debug)]
pub struct MockKeyboard {
    pressed: u64,
    active_layers: u32,
    modifiers: u8,
    sequence: u32,
    pending_skip: u32,
    rng_state: u64,
    valid_fields: u32,
    hid_indicators: u8,
    transport: Transport,
    ble_profile: u8,
    central_battery_pct: u8,
    peripheral_battery_pct: u8,
    split_status: SplitStatus,
    dropped_frames: u32,
}

impl Default for MockKeyboard {
    fn default() -> Self {
        Self {
            pressed: 0,
            active_layers: 1,
            modifiers: 0,
            sequence: 0,
            pending_skip: 0,
            rng_state: 0,
            valid_fields: BASE_VALID_FIELDS,
            hid_indicators: 0,
            transport: Transport::Ble,
            ble_profile: 0,
            central_battery_pct: 0xff,
            peripheral_battery_pct: 0xff,
            split_status: SplitStatus::Unknown,
            dropped_frames: 0,
        }
    }
}

impl MockKeyboard {
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }

    fn next_sequence(&mut self) -> u32 {
        self.sequence = self.sequence.wrapping_add(1 + self.pending_skip);
        self.pending_skip = 0;
        self.sequence
    }

    fn random_position(&mut self) -> u8 {
        if self.rng_state == 0 {
            self.rng_state = Self::now_ms() | 0x9E37_79B9_7F4A_7C15;
        }
        let mut value = self.rng_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.rng_state = value;
        (value % POSITION_COUNT as u64) as u8
    }

    fn frame(&mut self, snapshot: bool, changed_fields: u32) -> Frame {
        let sequence = if snapshot {
            self.sequence
        } else {
            self.next_sequence()
        };
        Frame {
            snapshot,
            sequence,
            timestamp_ms: Self::now_ms(),
            pressed_positions: self.pressed,
            active_layers: self.active_layers,
            changed_fields: if snapshot { 0 } else { changed_fields },
            valid_fields: self.valid_fields,
            modifiers: self.modifiers,
            hid_indicators: self.hid_indicators,
            default_layer: 0,
            transport: self.transport,
            ble_profile: self.ble_profile,
            central_battery_pct: self.central_battery_pct,
            peripheral_battery_pct: self.peripheral_battery_pct,
            split_status: self.split_status,
            dropped_frames: self.dropped_frames,
        }
    }
}

#[derive(Clone)]
pub struct MockSource {
    hub: SharedHub,
    keyboard: Arc<Mutex<MockKeyboard>>,
}

impl MockSource {
    pub fn new(hub: SharedHub) -> Self {
        Self {
            hub,
            keyboard: Arc::new(Mutex::new(MockKeyboard::default())),
        }
    }

    fn publish(&self, frame: Frame) {
        let bytes = protocol::encode(&frame);
        if let Ok(decoded) = protocol::decode(&bytes) {
            self.hub.lock().unwrap().publish_frame(&decoded);
        }
    }

    pub fn press(&self, position: u8) -> Result<(), String> {
        check_position(position)?;
        let frame = {
            let mut keyboard = self.keyboard.lock().unwrap();
            keyboard.pressed |= 1u64 << position;
            keyboard.frame(false, FIELD_POSITIONS)
        };
        self.publish(frame);
        Ok(())
    }

    pub fn release(&self, position: u8) -> Result<(), String> {
        check_position(position)?;
        let frame = {
            let mut keyboard = self.keyboard.lock().unwrap();
            keyboard.pressed &= !(1u64 << position);
            keyboard.frame(false, FIELD_POSITIONS)
        };
        self.publish(frame);
        Ok(())
    }

    pub fn burst(&self, count: u32) {
        for _ in 0..count {
            let position = {
                let mut keyboard = self.keyboard.lock().unwrap();
                let position = keyboard.random_position();
                keyboard.pressed |= 1u64 << position;
                let frame = keyboard.frame(false, FIELD_POSITIONS);
                drop(keyboard);
                self.publish(frame);
                position
            };
            let frame = {
                let mut keyboard = self.keyboard.lock().unwrap();
                keyboard.pressed &= !(1u64 << position);
                keyboard.frame(false, FIELD_POSITIONS)
            };
            self.publish(frame);
        }
    }

    pub fn hold_layer(&self, layer: u8) -> Result<(), String> {
        check_layer(layer)?;
        let frame = {
            let mut keyboard = self.keyboard.lock().unwrap();
            keyboard.active_layers |= 1u32 << layer;
            keyboard.frame(false, FIELD_LAYERS)
        };
        self.publish(frame);
        Ok(())
    }

    pub fn release_layer(&self, layer: u8) -> Result<(), String> {
        check_layer(layer)?;
        let frame = {
            let mut keyboard = self.keyboard.lock().unwrap();
            keyboard.active_layers &= !(1u32 << layer);
            if keyboard.active_layers == 0 {
                keyboard.active_layers = 1;
            }
            keyboard.frame(false, FIELD_LAYERS)
        };
        self.publish(frame);
        Ok(())
    }

    pub fn set_modifier(&self, bit: u8, active: bool) -> Result<(), String> {
        if bit >= 8 {
            return Err(format!("modifier bit {bit} out of range (0..8)"));
        }
        let frame = {
            let mut keyboard = self.keyboard.lock().unwrap();
            if active {
                keyboard.modifiers |= 1 << bit;
            } else {
                keyboard.modifiers &= !(1 << bit);
            }
            keyboard.frame(false, FIELD_MODIFIERS)
        };
        self.publish(frame);
        Ok(())
    }

    pub fn set_demo_status(&self, enabled: bool) {
        let frame = {
            let mut keyboard = self.keyboard.lock().unwrap();
            let fields = FIELD_HID_INDICATORS
                | FIELD_ENDPOINT
                | FIELD_CENTRAL_BATTERY
                | FIELD_PERIPHERAL_BATTERY
                | FIELD_SPLIT_STATUS;
            if enabled {
                keyboard.valid_fields |= fields;
                keyboard.hid_indicators = 0x02;
                keyboard.transport = Transport::Ble;
                keyboard.ble_profile = 2;
                keyboard.central_battery_pct = 91;
                keyboard.peripheral_battery_pct = 84;
                keyboard.split_status = SplitStatus::Connected;
            } else {
                keyboard.valid_fields &= !(FIELD_HID_INDICATORS
                    | FIELD_CENTRAL_BATTERY
                    | FIELD_PERIPHERAL_BATTERY
                    | FIELD_SPLIT_STATUS);
                keyboard.hid_indicators = 0;
                keyboard.central_battery_pct = 0xff;
                keyboard.peripheral_battery_pct = 0xff;
                keyboard.split_status = SplitStatus::Unknown;
            }
            keyboard.frame(false, fields)
        };
        self.publish(frame);
    }

    pub fn inject_gap(&self) {
        self.keyboard.lock().unwrap().pending_skip += 2;
    }

    pub fn inject_firmware_drop(&self) {
        let frame = {
            let mut keyboard = self.keyboard.lock().unwrap();
            keyboard.dropped_frames = keyboard.dropped_frames.wrapping_add(1);
            keyboard.frame(false, 0)
        };
        self.publish(frame);
    }

    pub fn disconnect(&self) {
        self.hub
            .lock()
            .unwrap()
            .publish_connection(ConnectionStatus::Disconnected, None);
    }

    pub fn reconnect(&self) {
        let snapshot = self.keyboard.lock().unwrap().frame(true, 0);
        {
            let mut hub = self.hub.lock().unwrap();
            hub.reset_sequence_tracking();
            hub.publish_connection(ConnectionStatus::Connected, None);
        }
        self.publish(snapshot);
    }
}

fn check_position(position: u8) -> Result<(), String> {
    if position >= POSITION_COUNT {
        return Err(format!(
            "position {position} out of range (0..{POSITION_COUNT})"
        ));
    }
    Ok(())
}

fn check_layer(layer: u8) -> Result<(), String> {
    if layer >= 32 {
        return Err(format!("layer {layer} out of range (0..32)"));
    }
    Ok(())
}
