use serde::Serialize;

use super::protocol::{pressed_set, Record};

pub const TELEMETRY_STATE_EVENT: &str = "telemetry-state";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

/// Full keyboard state published to the frontend on every valid record.
/// The frontend replaces its state wholesale with each event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryState {
    pub connection: ConnectionStatus,
    pub pressed: Vec<u8>,
    pub active_layers: u32,
    pub sequence: u16,
    pub timestamp_ms: u32,
    pub gaps: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Tracks sequence gaps across records (mod 65536).
#[derive(Debug, Default)]
pub struct SequenceTracker {
    last_sequence: Option<u16>,
    gaps: u64,
}

impl SequenceTracker {
    pub fn observe(&mut self, sequence: u16) {
        if let Some(prev) = self.last_sequence {
            let diff = sequence.wrapping_sub(prev);
            if diff > 1 {
                self.gaps += (diff - 1) as u64;
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

impl TelemetryState {
    pub fn from_record(record: &Record, connection: ConnectionStatus, gaps: u64) -> Self {
        Self {
            connection,
            pressed: pressed_set(record.pressed_positions),
            active_layers: record.active_layers,
            sequence: record.sequence,
            timestamp_ms: record.timestamp_ms,
            gaps,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gap_on_first_record() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(100);
        assert_eq!(tracker.gaps(), 0);
    }

    #[test]
    fn no_gap_on_consecutive() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(100);
        tracker.observe(101);
        assert_eq!(tracker.gaps(), 0);
    }

    #[test]
    fn counts_gap_by_modular_difference() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(100);
        tracker.observe(103);
        assert_eq!(tracker.gaps(), 2);
    }

    #[test]
    fn wraps_modulo_65536() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(65535);
        tracker.observe(0);
        assert_eq!(tracker.gaps(), 0);
    }

    #[test]
    fn counts_gap_across_wrap() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(65534);
        tracker.observe(1);
        assert_eq!(tracker.gaps(), 2);
    }

    #[test]
    fn accumulates_across_records() {
        let mut tracker = SequenceTracker::default();
        tracker.observe(0);
        tracker.observe(3);
        tracker.observe(10);
        assert_eq!(tracker.gaps(), 2 + 6);
    }

    #[test]
    fn state_collects_full_record_fields() {
        let record = super::super::protocol::decode(&[
            0x01, 0x01, 0x00, 0xFF, 0x07, 0x00, 0x40, 0xE2, 0x01, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x08, 0x80, 0x00, 0x00, 0x00, 0x00,
        ])
        .unwrap();
        let state = TelemetryState::from_record(&record, ConnectionStatus::Connected, 3);
        assert_eq!(state.connection, ConnectionStatus::Connected);
        assert_eq!(state.pressed, vec![3, 15]);
        assert_eq!(state.active_layers, 0x9);
        assert_eq!(state.sequence, 7);
        assert_eq!(state.timestamp_ms, 123_456);
        assert_eq!(state.gaps, 3);
    }
}
