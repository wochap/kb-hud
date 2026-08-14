use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use super::protocol::Record;
use super::state::{ConnectionStatus, SequenceTracker, TelemetryState, TELEMETRY_STATE_EVENT};

/// Single publication path shared by BLE and mock sources. Both feed decoded
/// records through here; the frontend cannot tell them apart.
pub struct TelemetryHub {
    app: AppHandle,
    tracker: SequenceTracker,
    last: Option<TelemetryState>,
}

pub type SharedHub = Arc<Mutex<TelemetryHub>>;

impl TelemetryHub {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            tracker: SequenceTracker::default(),
            last: None,
        }
    }

    pub fn shared(app: AppHandle) -> SharedHub {
        Arc::new(Mutex::new(Self::new(app)))
    }

    fn emit(&self, state: &TelemetryState) {
        let _ = self.app.emit(TELEMETRY_STATE_EVENT, state);
    }

    /// Publishes a decoded record as the full keyboard state.
    pub fn publish_record(&mut self, record: &Record) {
        self.tracker.observe(record.sequence);
        let state = TelemetryState::from_record(
            record,
            ConnectionStatus::Connected,
            self.tracker.gaps(),
        );
        self.last = Some(state.clone());
        self.emit(&state);
    }

    /// Publishes a connection-state transition. Keeps the last known layer
    /// mask so the overlay stays meaningful while disconnected; pressed keys
    /// are cleared since no key can stay pressed without a connection.
    pub fn publish_connection(&mut self, connection: ConnectionStatus, error: Option<String>) {
        let mut state = self.last.clone().unwrap_or(TelemetryState {
            connection,
            pressed: Vec::new(),
            active_layers: 1,
            sequence: 0,
            timestamp_ms: 0,
            gaps: self.tracker.gaps(),
            error: None,
        });
        state.connection = connection;
        state.error = error;
        if connection != ConnectionStatus::Connected {
            state.pressed.clear();
        }
        self.last = Some(state.clone());
        self.emit(&state);
    }

    /// New record sessions start with a fresh snapshot; sequence continuity
    /// does not hold across reconnects.
    pub fn reset_sequence_tracking(&mut self) {
        self.tracker.reset();
    }
}
