//! Mock telemetry source. Synthesizes protocol-v1 records and publishes
//! them through the exact same encode → decode → hub path as BLE, so every
//! downstream behavior is exercisable without a Bluetooth stack.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::telemetry::hub::SharedHub;
use crate::telemetry::protocol::{self, Record, RecordType, POSITION_COUNT};
use crate::telemetry::state::ConnectionStatus;

#[derive(Debug, Default)]
pub struct MockKeyboard {
    pressed: u64,
    active_layers: u32,
    sequence: u16,
    pending_skip: u16,
    rng_state: u64,
}

impl MockKeyboard {
    fn now_ms() -> u32 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0)
    }

    fn next_sequence(&mut self) -> u16 {
        self.sequence = self.sequence.wrapping_add(1 + self.pending_skip);
        self.pending_skip = 0;
        self.sequence
    }

    fn random_position(&mut self) -> u8 {
        if self.rng_state == 0 {
            self.rng_state = Self::now_ms() as u64 | 0x9E37_79B9_7F4A_7C15;
        }
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x % POSITION_COUNT as u64) as u8
    }

    fn snapshot_record(&mut self) -> Record {
        Record {
            record_type: RecordType::Snapshot,
            pressed: false,
            position: None,
            sequence: self.next_sequence(),
            timestamp_ms: Self::now_ms(),
            active_layers: self.active_layers,
            pressed_positions: self.pressed,
        }
    }

    fn key_record(&mut self, position: u8, pressed: bool) -> Record {
        Record {
            record_type: RecordType::Key,
            pressed,
            position: Some(position),
            sequence: self.next_sequence(),
            timestamp_ms: Self::now_ms(),
            active_layers: self.active_layers,
            pressed_positions: self.pressed,
        }
    }

    fn layers_record(&mut self) -> Record {
        Record {
            record_type: RecordType::Layers,
            pressed: false,
            position: None,
            sequence: self.next_sequence(),
            timestamp_ms: Self::now_ms(),
            active_layers: self.active_layers,
            pressed_positions: self.pressed,
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
        // default layer active, like a freshly booted keyboard
        let keyboard = MockKeyboard {
            active_layers: 1,
            ..Default::default()
        };
        Self {
            hub,
            keyboard: Arc::new(Mutex::new(keyboard)),
        }
    }

    /// Encodes, then decodes, then publishes — the identical path BLE
    /// records traverse.
    fn publish(&self, record: Record) {
        let bytes = protocol::encode(&record);
        if let Ok(decoded) = protocol::decode(&bytes) {
            self.hub.lock().unwrap().publish_record(&decoded);
        }
    }

    pub fn press(&self, position: u8) -> Result<(), String> {
        check_position(position)?;
        let mut kb = self.keyboard.lock().unwrap();
        kb.pressed |= 1u64 << position;
        self.publish(kb.key_record(position, true));
        Ok(())
    }

    pub fn release(&self, position: u8) -> Result<(), String> {
        check_position(position)?;
        let mut kb = self.keyboard.lock().unwrap();
        kb.pressed &= !(1u64 << position);
        self.publish(kb.key_record(position, false));
        Ok(())
    }

    pub fn burst(&self, count: u32) {
        for _ in 0..count {
            let position = {
                let mut kb = self.keyboard.lock().unwrap();
                let position = kb.random_position();
                kb.pressed |= 1u64 << position;
                self.publish(kb.key_record(position, true));
                position
            };
            let mut kb = self.keyboard.lock().unwrap();
            kb.pressed &= !(1u64 << position);
            self.publish(kb.key_record(position, false));
        }
    }

    pub fn hold_layer(&self, layer: u8) -> Result<(), String> {
        check_layer(layer)?;
        let mut kb = self.keyboard.lock().unwrap();
        kb.active_layers |= 1u32 << layer;
        self.publish(kb.layers_record());
        Ok(())
    }

    pub fn release_layer(&self, layer: u8) -> Result<(), String> {
        check_layer(layer)?;
        let mut kb = self.keyboard.lock().unwrap();
        // never release the base layer bit entirely — keep at least bit 0
        kb.active_layers &= !(1u32 << layer);
        if kb.active_layers == 0 {
            kb.active_layers = 1;
        }
        self.publish(kb.layers_record());
        Ok(())
    }

    /// The next record skips sequence numbers, exercising gap detection.
    pub fn inject_gap(&self) {
        self.keyboard.lock().unwrap().pending_skip += 2;
    }

    pub fn disconnect(&self) {
        self.hub
            .lock()
            .unwrap()
            .publish_connection(ConnectionStatus::Disconnected, None);
    }

    pub fn reconnect(&self) {
        let mut hub = self.hub.lock().unwrap();
        hub.reset_sequence_tracking();
        hub.publish_connection(ConnectionStatus::Connected, None);
        let snapshot = self.keyboard.lock().unwrap().snapshot_record();
        drop(hub);
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
