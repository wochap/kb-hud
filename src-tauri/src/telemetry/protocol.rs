//! Decoder for the zmk-key-telemetry authoritative state frame.
//! All multibyte values are unsigned little-endian and use explicit offsets.

pub const FRAME_SIZE: usize = 48;
pub const FORMAT_IDENTIFIER: u8 = 2;
pub const POSITION_COUNT: u8 = 64;
pub const VALUE_UNKNOWN: u8 = 0xff;

pub const FLAG_SNAPSHOT: u8 = 1 << 0;
pub const KNOWN_FLAGS: u8 = FLAG_SNAPSHOT;

pub const FIELD_POSITIONS: u32 = 1 << 0;
pub const FIELD_LAYERS: u32 = 1 << 1;
pub const FIELD_MODIFIERS: u32 = 1 << 2;
pub const FIELD_HID_INDICATORS: u32 = 1 << 3;
pub const FIELD_DEFAULT_LAYER: u32 = 1 << 4;
pub const FIELD_ENDPOINT: u32 = 1 << 5;
pub const FIELD_CENTRAL_BATTERY: u32 = 1 << 6;
pub const FIELD_PERIPHERAL_BATTERY: u32 = 1 << 7;
pub const FIELD_SPLIT_STATUS: u32 = 1 << 8;

const OFFSET_VERSION: usize = 0;
const OFFSET_FLAGS: usize = 1;
const OFFSET_FRAME_SIZE: usize = 2;
const OFFSET_SEQUENCE: usize = 4;
const OFFSET_TIMESTAMP: usize = 8;
const OFFSET_POSITIONS: usize = 16;
const OFFSET_LAYERS: usize = 24;
const OFFSET_CHANGED_FIELDS: usize = 28;
const OFFSET_VALID_FIELDS: usize = 32;
const OFFSET_MODIFIERS: usize = 36;
const OFFSET_HID_INDICATORS: usize = 37;
const OFFSET_DEFAULT_LAYER: usize = 38;
const OFFSET_TRANSPORT: usize = 39;
const OFFSET_BLE_PROFILE: usize = 40;
const OFFSET_CENTRAL_BATTERY: usize = 41;
const OFFSET_PERIPHERAL_BATTERY: usize = 42;
const OFFSET_SPLIT_STATUS: usize = 43;
const OFFSET_DROPPED_FRAMES: usize = 44;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Transport {
    Unknown = 0,
    Usb = 1,
    Ble = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SplitStatus {
    Unknown = 0,
    Disconnected = 1,
    Connected = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub snapshot: bool,
    pub sequence: u32,
    pub timestamp_ms: u64,
    pub pressed_positions: u64,
    pub active_layers: u32,
    pub changed_fields: u32,
    pub valid_fields: u32,
    pub modifiers: u8,
    pub hid_indicators: u8,
    pub default_layer: u8,
    pub transport: Transport,
    pub ble_profile: u8,
    pub central_battery_pct: u8,
    pub peripheral_battery_pct: u8,
    pub split_status: SplitStatus,
    pub dropped_frames: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    InvalidLength { len: usize },
    UnsupportedFormatIdentifier { identifier: u8 },
    InvalidDeclaredSize { size: u16 },
    UnknownFlags { flags: u8 },
    InvalidTransport { value: u8 },
    InvalidSplitStatus { value: u8 },
    InvalidDefaultLayer { value: u8 },
    InvalidBattery { field: &'static str, value: u8 },
    InvalidBleProfile { value: u8 },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength { len } => write!(f, "frame length {len} != {FRAME_SIZE}"),
            Self::UnsupportedFormatIdentifier { identifier } => {
                write!(
                    f,
                    "unsupported telemetry format identifier 0x{identifier:02x}"
                )
            }
            Self::InvalidDeclaredSize { size } => {
                write!(f, "declared frame size {size} != {FRAME_SIZE}")
            }
            Self::UnknownFlags { flags } => write!(f, "unknown frame flags 0x{flags:02x}"),
            Self::InvalidTransport { value } => write!(f, "invalid transport {value}"),
            Self::InvalidSplitStatus { value } => write!(f, "invalid split status {value}"),
            Self::InvalidDefaultLayer { value } => write!(f, "invalid default layer {value}"),
            Self::InvalidBattery { field, value } => {
                write!(f, "invalid {field} battery percentage {value}")
            }
            Self::InvalidBleProfile { value } => write!(f, "invalid BLE profile {value}"),
        }
    }
}

fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn le64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

pub fn decode(bytes: &[u8]) -> Result<Frame, DecodeError> {
    if bytes.len() != FRAME_SIZE {
        return Err(DecodeError::InvalidLength { len: bytes.len() });
    }
    if bytes[OFFSET_VERSION] != FORMAT_IDENTIFIER {
        return Err(DecodeError::UnsupportedFormatIdentifier {
            identifier: bytes[OFFSET_VERSION],
        });
    }
    let declared_size = le16(&bytes[OFFSET_FRAME_SIZE..]);
    if declared_size as usize != FRAME_SIZE {
        return Err(DecodeError::InvalidDeclaredSize {
            size: declared_size,
        });
    }
    let flags = bytes[OFFSET_FLAGS];
    if flags & !KNOWN_FLAGS != 0 {
        return Err(DecodeError::UnknownFlags { flags });
    }

    let valid_fields = le32(&bytes[OFFSET_VALID_FIELDS..]);
    let transport = match bytes[OFFSET_TRANSPORT] {
        0 => Transport::Unknown,
        1 => Transport::Usb,
        2 => Transport::Ble,
        value => return Err(DecodeError::InvalidTransport { value }),
    };
    let split_status = match bytes[OFFSET_SPLIT_STATUS] {
        0 => SplitStatus::Unknown,
        1 => SplitStatus::Disconnected,
        2 => SplitStatus::Connected,
        value => return Err(DecodeError::InvalidSplitStatus { value }),
    };
    if valid_fields & FIELD_SPLIT_STATUS != 0 && split_status == SplitStatus::Unknown {
        return Err(DecodeError::InvalidSplitStatus { value: 0 });
    }
    let default_layer = bytes[OFFSET_DEFAULT_LAYER];
    if valid_fields & FIELD_DEFAULT_LAYER != 0 && default_layer >= 32 {
        return Err(DecodeError::InvalidDefaultLayer {
            value: default_layer,
        });
    }
    for (bit, field, offset) in [
        (FIELD_CENTRAL_BATTERY, "central", OFFSET_CENTRAL_BATTERY),
        (
            FIELD_PERIPHERAL_BATTERY,
            "peripheral",
            OFFSET_PERIPHERAL_BATTERY,
        ),
    ] {
        let value = bytes[offset];
        if valid_fields & bit != 0 && value > 100 {
            return Err(DecodeError::InvalidBattery { field, value });
        }
    }
    let ble_profile = bytes[OFFSET_BLE_PROFILE];
    if valid_fields & FIELD_ENDPOINT != 0 {
        match transport {
            Transport::Ble if ble_profile == VALUE_UNKNOWN => {
                return Err(DecodeError::InvalidBleProfile { value: ble_profile })
            }
            Transport::Usb if ble_profile != VALUE_UNKNOWN => {
                return Err(DecodeError::InvalidBleProfile { value: ble_profile })
            }
            Transport::Unknown => {
                return Err(DecodeError::InvalidTransport { value: 0 });
            }
            _ => {}
        }
    }

    Ok(Frame {
        snapshot: flags & FLAG_SNAPSHOT != 0,
        sequence: le32(&bytes[OFFSET_SEQUENCE..]),
        timestamp_ms: le64(&bytes[OFFSET_TIMESTAMP..]),
        pressed_positions: le64(&bytes[OFFSET_POSITIONS..]),
        active_layers: le32(&bytes[OFFSET_LAYERS..]),
        changed_fields: le32(&bytes[OFFSET_CHANGED_FIELDS..]),
        valid_fields,
        modifiers: bytes[OFFSET_MODIFIERS],
        hid_indicators: bytes[OFFSET_HID_INDICATORS],
        default_layer,
        transport,
        ble_profile,
        central_battery_pct: bytes[OFFSET_CENTRAL_BATTERY],
        peripheral_battery_pct: bytes[OFFSET_PERIPHERAL_BATTERY],
        split_status,
        dropped_frames: le32(&bytes[OFFSET_DROPPED_FRAMES..]),
    })
}

pub fn pressed_set(bitmap: u64) -> Vec<u8> {
    (0..POSITION_COUNT)
        .filter(|pos| bitmap & (1u64 << pos) != 0)
        .collect()
}

/// Mirrors the firmware encoder so mock frames traverse the production decoder.
pub fn encode(frame: &Frame) -> [u8; FRAME_SIZE] {
    let mut out = [0u8; FRAME_SIZE];
    out[OFFSET_VERSION] = FORMAT_IDENTIFIER;
    out[OFFSET_FLAGS] = if frame.snapshot { FLAG_SNAPSHOT } else { 0 };
    out[OFFSET_FRAME_SIZE..OFFSET_FRAME_SIZE + 2]
        .copy_from_slice(&(FRAME_SIZE as u16).to_le_bytes());
    out[OFFSET_SEQUENCE..OFFSET_SEQUENCE + 4].copy_from_slice(&frame.sequence.to_le_bytes());
    out[OFFSET_TIMESTAMP..OFFSET_TIMESTAMP + 8].copy_from_slice(&frame.timestamp_ms.to_le_bytes());
    out[OFFSET_POSITIONS..OFFSET_POSITIONS + 8]
        .copy_from_slice(&frame.pressed_positions.to_le_bytes());
    out[OFFSET_LAYERS..OFFSET_LAYERS + 4].copy_from_slice(&frame.active_layers.to_le_bytes());
    out[OFFSET_CHANGED_FIELDS..OFFSET_CHANGED_FIELDS + 4]
        .copy_from_slice(&frame.changed_fields.to_le_bytes());
    out[OFFSET_VALID_FIELDS..OFFSET_VALID_FIELDS + 4]
        .copy_from_slice(&frame.valid_fields.to_le_bytes());
    out[OFFSET_MODIFIERS] = frame.modifiers;
    out[OFFSET_HID_INDICATORS] = frame.hid_indicators;
    out[OFFSET_DEFAULT_LAYER] = frame.default_layer;
    out[OFFSET_TRANSPORT] = frame.transport as u8;
    out[OFFSET_BLE_PROFILE] = frame.ble_profile;
    out[OFFSET_CENTRAL_BATTERY] = frame.central_battery_pct;
    out[OFFSET_PERIPHERAL_BATTERY] = frame.peripheral_battery_pct;
    out[OFFSET_SPLIT_STATUS] = frame.split_status as u8;
    out[OFFSET_DROPPED_FRAMES..OFFSET_DROPPED_FRAMES + 4]
        .copy_from_slice(&frame.dropped_frames.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN_FIELDS: u32 = 0x0000_01ff;

    fn full_frame() -> Frame {
        Frame {
            snapshot: true,
            sequence: 0x1234_5678,
            timestamp_ms: 0x0123_4567_89ab_cdef,
            pressed_positions: 0x8000_0008_0100_0081,
            active_layers: 0x8000_0025,
            changed_fields: KNOWN_FIELDS,
            valid_fields: KNOWN_FIELDS,
            modifiers: 0xa5,
            hid_indicators: 3,
            default_layer: 2,
            transport: Transport::Ble,
            ble_profile: 4,
            central_battery_pct: 99,
            peripheral_battery_pct: 87,
            split_status: SplitStatus::Connected,
            dropped_frames: 0x89ab_cdef,
        }
    }

    #[test]
    fn exact_layout_and_round_trip() {
        let frame = full_frame();
        let bytes = encode(&frame);
        assert_eq!(&bytes[0..8], &[2, 1, 48, 0, 0x78, 0x56, 0x34, 0x12]);
        assert_eq!(
            &bytes[8..16],
            &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]
        );
        assert_eq!(&bytes[36..44], &[0xa5, 3, 2, 2, 4, 99, 87, 2]);
        assert_eq!(&bytes[44..48], &[0xef, 0xcd, 0xab, 0x89]);
        assert_eq!(decode(&bytes).unwrap(), frame);
        assert_eq!(pressed_set(frame.pressed_positions), vec![0, 7, 24, 35, 63]);
    }

    #[test]
    fn retains_unknown_field_bits() {
        let mut frame = full_frame();
        frame.changed_fields |= 1 << 31;
        frame.valid_fields |= 1 << 30;
        let decoded = decode(&encode(&frame)).unwrap();
        assert_eq!(decoded.changed_fields, frame.changed_fields);
        assert_eq!(decoded.valid_fields, frame.valid_fields);
    }

    #[test]
    fn validates_envelope_and_ranges() {
        let bytes = encode(&full_frame());
        assert_eq!(
            decode(&bytes[..47]),
            Err(DecodeError::InvalidLength { len: 47 })
        );

        let mut bad = bytes;
        bad[OFFSET_VERSION] = 1;
        assert_eq!(
            decode(&bad),
            Err(DecodeError::UnsupportedFormatIdentifier { identifier: 1 })
        );
        bad = bytes;
        bad[OFFSET_FRAME_SIZE] = 49;
        assert_eq!(
            decode(&bad),
            Err(DecodeError::InvalidDeclaredSize { size: 49 })
        );
        bad = bytes;
        bad[OFFSET_FLAGS] = 0x80;
        assert_eq!(decode(&bad), Err(DecodeError::UnknownFlags { flags: 0x80 }));
        bad = bytes;
        bad[OFFSET_CENTRAL_BATTERY] = 101;
        assert_eq!(
            decode(&bad),
            Err(DecodeError::InvalidBattery {
                field: "central",
                value: 101
            })
        );
    }

    #[test]
    fn accepts_invalid_optional_sentinels_when_not_valid() {
        let mut frame = full_frame();
        frame.valid_fields = FIELD_POSITIONS | FIELD_LAYERS | FIELD_MODIFIERS;
        frame.default_layer = VALUE_UNKNOWN;
        frame.transport = Transport::Unknown;
        frame.ble_profile = VALUE_UNKNOWN;
        frame.central_battery_pct = VALUE_UNKNOWN;
        frame.peripheral_battery_pct = VALUE_UNKNOWN;
        frame.split_status = SplitStatus::Unknown;
        assert_eq!(decode(&encode(&frame)).unwrap(), frame);
    }
}
