//! Protocol v1 decoder for the zmk-key-telemetry GATT characteristic.
//!
//! Record layout (20 bytes, little-endian):
//! offset 0: version (1)
//! offset 1: type (0x01 snapshot, 0x02 key, 0x03 layers)
//! offset 2: flags (bit0 = pressed)
//! offset 3: position (0xFF = none)
//! offset 4: sequence (u16)
//! offset 6: timestamp_ms (u32)
//! offset 10: active_layers (u32)
//! offset 14: pressed_positions bitmap (48 bits, 6 bytes)

pub const RECORD_SIZE: usize = 20;
pub const PROTOCOL_VERSION: u8 = 1;
pub const POSITION_COUNT: u8 = 48;
pub const POSITION_NONE: u8 = 0xFF;

const OFFSET_VERSION: usize = 0;
const OFFSET_TYPE: usize = 1;
const OFFSET_FLAGS: usize = 2;
const OFFSET_POSITION: usize = 3;
const OFFSET_SEQUENCE: usize = 4;
const OFFSET_TIMESTAMP: usize = 6;
const OFFSET_LAYERS: usize = 10;
const OFFSET_POSITIONS: usize = 14;

const FLAG_PRESSED: u8 = 1 << 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    Snapshot,
    Key,
    Layers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    InvalidLength { len: usize },
    UnsupportedVersion { version: u8 },
    UnknownType { type_byte: u8 },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidLength { len } => {
                write!(f, "record length {len} != {RECORD_SIZE}")
            }
            DecodeError::UnsupportedVersion { version } => {
                write!(f, "unsupported protocol version 0x{version:02x}")
            }
            DecodeError::UnknownType { type_byte } => {
                write!(f, "unknown record type 0x{type_byte:02x}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub record_type: RecordType,
    pub pressed: bool,
    pub position: Option<u8>,
    pub sequence: u16,
    pub timestamp_ms: u32,
    pub active_layers: u32,
    pub pressed_positions: u64,
}

fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub fn decode(bytes: &[u8]) -> Result<Record, DecodeError> {
    if bytes.len() != RECORD_SIZE {
        return Err(DecodeError::InvalidLength { len: bytes.len() });
    }

    let version = bytes[OFFSET_VERSION];
    if version != PROTOCOL_VERSION {
        return Err(DecodeError::UnsupportedVersion { version });
    }

    let record_type = match bytes[OFFSET_TYPE] {
        0x01 => RecordType::Snapshot,
        0x02 => RecordType::Key,
        0x03 => RecordType::Layers,
        other => return Err(DecodeError::UnknownType { type_byte: other }),
    };

    let flags = bytes[OFFSET_FLAGS];
    let position_raw = bytes[OFFSET_POSITION];

    let mut pressed_positions: u64 = 0;
    for i in 0..6 {
        pressed_positions |= (bytes[OFFSET_POSITIONS + i] as u64) << (i * 8);
    }

    Ok(Record {
        record_type,
        pressed: flags & FLAG_PRESSED != 0,
        position: if position_raw == POSITION_NONE {
            None
        } else {
            Some(position_raw)
        },
        sequence: le16(&bytes[OFFSET_SEQUENCE..]),
        timestamp_ms: le32(&bytes[OFFSET_TIMESTAMP..]),
        active_layers: le32(&bytes[OFFSET_LAYERS..]),
        pressed_positions,
    })
}

pub fn pressed_set(bitmap: u64) -> Vec<u8> {
    (0..POSITION_COUNT)
        .filter(|pos| bitmap & (1 << pos) != 0)
        .collect()
}

/// Encodes a record into the 20-byte wire format. Mirrors the ZMK firmware
/// encoder; used by the mock telemetry source so mock records traverse the
/// exact same decode path as BLE records.
pub fn encode(record: &Record) -> [u8; RECORD_SIZE] {
    let mut out = [0u8; RECORD_SIZE];
    out[OFFSET_VERSION] = PROTOCOL_VERSION;
    out[OFFSET_TYPE] = match record.record_type {
        RecordType::Snapshot => 0x01,
        RecordType::Key => 0x02,
        RecordType::Layers => 0x03,
    };
    out[OFFSET_FLAGS] = if record.pressed { FLAG_PRESSED } else { 0 };
    out[OFFSET_POSITION] = record.position.unwrap_or(POSITION_NONE);
    out[OFFSET_SEQUENCE..OFFSET_SEQUENCE + 2].copy_from_slice(&record.sequence.to_le_bytes());
    out[OFFSET_TIMESTAMP..OFFSET_TIMESTAMP + 4]
        .copy_from_slice(&record.timestamp_ms.to_le_bytes());
    out[OFFSET_LAYERS..OFFSET_LAYERS + 4].copy_from_slice(&record.active_layers.to_le_bytes());
    for i in 0..6 {
        out[OFFSET_POSITIONS + i] = (record.pressed_positions >> (i * 8)) as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        record_type: u8,
        flags: u8,
        position: u8,
        sequence: u16,
        timestamp_ms: u32,
        active_layers: u32,
        pressed_positions: u64,
    ) -> [u8; RECORD_SIZE] {
        let mut out = [0u8; RECORD_SIZE];
        out[OFFSET_VERSION] = PROTOCOL_VERSION;
        out[OFFSET_TYPE] = record_type;
        out[OFFSET_FLAGS] = flags;
        out[OFFSET_POSITION] = position;
        out[OFFSET_SEQUENCE..OFFSET_SEQUENCE + 2].copy_from_slice(&sequence.to_le_bytes());
        out[OFFSET_TIMESTAMP..OFFSET_TIMESTAMP + 4].copy_from_slice(&timestamp_ms.to_le_bytes());
        out[OFFSET_LAYERS..OFFSET_LAYERS + 4].copy_from_slice(&active_layers.to_le_bytes());
        for i in 0..6 {
            out[OFFSET_POSITIONS + i] = (pressed_positions >> (i * 8)) as u8;
        }
        out
    }

    #[test]
    fn decodes_snapshot() {
        let bytes = record(0x01, 0, POSITION_NONE, 7, 123456, 0x0000_0001, 0b1011);
        let rec = decode(&bytes).unwrap();
        assert_eq!(rec.record_type, RecordType::Snapshot);
        assert!(!rec.pressed);
        assert_eq!(rec.position, None);
        assert_eq!(rec.sequence, 7);
        assert_eq!(rec.timestamp_ms, 123456);
        assert_eq!(rec.active_layers, 1);
        assert_eq!(pressed_set(rec.pressed_positions), vec![0, 1, 3]);
    }

    #[test]
    fn decodes_key_down() {
        let bytes = record(0x02, FLAG_PRESSED, 15, 101, 900, 0x9, 1 << 15);
        let rec = decode(&bytes).unwrap();
        assert_eq!(rec.record_type, RecordType::Key);
        assert!(rec.pressed);
        assert_eq!(rec.position, Some(15));
        assert_eq!(rec.sequence, 101);
        assert_eq!(rec.active_layers, 0x9);
        assert_eq!(pressed_set(rec.pressed_positions), vec![15]);
    }

    #[test]
    fn decodes_key_up() {
        let bytes = record(0x02, 0, 15, 102, 970, 0x9, 0);
        let rec = decode(&bytes).unwrap();
        assert_eq!(rec.record_type, RecordType::Key);
        assert!(!rec.pressed);
        assert_eq!(rec.position, Some(15));
        assert_eq!(pressed_set(rec.pressed_positions), Vec::<u8>::new());
    }

    #[test]
    fn decodes_layers_record() {
        let bytes = record(0x03, 0, POSITION_NONE, 103, 1000, 0x0000_0019, 0);
        let rec = decode(&bytes).unwrap();
        assert_eq!(rec.record_type, RecordType::Layers);
        assert_eq!(rec.active_layers, 0x19);
    }

    #[test]
    fn decodes_high_positions() {
        let bitmap: u64 = (1 << 41) | (1 << 47);
        let bytes = record(0x01, 0, POSITION_NONE, 1, 0, 1, bitmap);
        let rec = decode(&bytes).unwrap();
        assert_eq!(pressed_set(rec.pressed_positions), vec![41, 47]);
    }

    #[test]
    fn rejects_bad_version() {
        let mut bytes = record(0x01, 0, POSITION_NONE, 0, 0, 0, 0);
        bytes[OFFSET_VERSION] = 0x02;
        assert_eq!(
            decode(&bytes),
            Err(DecodeError::UnsupportedVersion { version: 2 })
        );
    }

    #[test]
    fn rejects_short_record() {
        let bytes = [0u8; RECORD_SIZE - 1];
        assert_eq!(
            decode(&bytes),
            Err(DecodeError::InvalidLength {
                len: RECORD_SIZE - 1
            })
        );
    }

    #[test]
    fn rejects_long_record() {
        let bytes = [0u8; RECORD_SIZE + 4];
        assert_eq!(
            decode(&bytes),
            Err(DecodeError::InvalidLength {
                len: RECORD_SIZE + 4
            })
        );
    }

    #[test]
    fn rejects_unknown_type() {
        let bytes = record(0x7f, 0, POSITION_NONE, 0, 0, 0, 0);
        assert_eq!(decode(&bytes), Err(DecodeError::UnknownType { type_byte: 0x7f }));
    }

    #[test]
    fn encode_decode_round_trip() {
        let original = Record {
            record_type: RecordType::Key,
            pressed: true,
            position: Some(41),
            sequence: 65530,
            timestamp_ms: 4_000_000_000,
            active_layers: 0xDEAD_BEEF,
            pressed_positions: (1 << 41) | (1 << 3),
        };
        let decoded = decode(&encode(&original)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn encode_maps_none_position_to_0xff() {
        let original = Record {
            record_type: RecordType::Snapshot,
            pressed: false,
            position: None,
            sequence: 0,
            timestamp_ms: 0,
            active_layers: 1,
            pressed_positions: 0,
        };
        let bytes = encode(&original);
        assert_eq!(bytes[OFFSET_POSITION], POSITION_NONE);
    }
}
