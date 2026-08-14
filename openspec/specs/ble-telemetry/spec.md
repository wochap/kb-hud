# ble-telemetry Specification

## Purpose

BlueZ GATT connection lifecycle for the keyboard's BLE telemetry: device
auto-detection, telemetry-frame validation and decoding, full-state
publication to the frontend, sequence-gap detection, and automatic
reconnection after keyboard sleep.

## Requirements

### Requirement: Device auto-detection
The system SHALL enumerate paired BlueZ devices and, when the active
profile's device selection is `auto`, select the unique device compatible
with `zmk-key-telemetry`. Compatibility SHALL require telemetry service
`9e7a7d70-df1b-4f76-9d45-8c3f4a6b2100` and characteristic
`9e7a7d70-df1b-4f76-9d45-8c3f4a6b2101` and SHALL NOT depend on the device
alias, keyboard model, or deployment name. The system SHALL use BlueZ's
cached service UUID metadata when available and SHALL connect to probe
paired candidates when metadata is insufficient to identify compatibility
reliably.

#### Scenario: Single compatible paired device in cached metadata
- **WHEN** exactly one paired device has the telemetry service UUID in BlueZ's cached UUID metadata and the profile device is `auto`
- **THEN** the system selects that device and validates the telemetry characteristic during connection

#### Scenario: Metadata unavailable
- **WHEN** BlueZ does not expose sufficient cached UUID metadata for paired candidates and the profile device is `auto`
- **THEN** the system probes paired candidates by connecting and inspecting their GATT service and characteristic UUIDs

#### Scenario: Bluetooth name is irrelevant
- **WHEN** a paired device exposes the required telemetry service and characteristic under any Bluetooth alias and the profile device is `auto`
- **THEN** the device remains eligible for automatic selection

#### Scenario: No compatible device
- **WHEN** no paired device can be identified as compatible and the profile device is `auto`
- **THEN** the system reports a "no telemetry keyboard found" error in the connection status without crashing and retains retry behavior for temporarily unavailable candidates

#### Scenario: Multiple compatible devices
- **WHEN** more than one paired device is identified as compatible and the profile device is `auto`
- **THEN** the system reports an ambiguity error that lists the candidates and requires an explicit MAC address in the profile

#### Scenario: Explicit device selection
- **WHEN** the profile contains an explicit valid Bluetooth MAC address
- **THEN** the system connects to that address without running automatic candidate selection and validates its telemetry GATT interface through the existing connection path

### Requirement: GATT subscription
The system SHALL connect to the selected device, discover telemetry service `9e7a7d70-df1b-4f76-9d45-8c3f4a6b2100`, subscribe to notifications on characteristic `9e7a7d70-df1b-4f76-9d45-8c3f4a6b2101`, and perform an initial characteristic read to obtain an authoritative snapshot frame.

#### Scenario: Successful subscription
- **WHEN** connection and discovery succeed with negotiated ATT MTU at least 51
- **THEN** the system receives an initial 48-byte snapshot and subsequent state-frame notifications

#### Scenario: Telemetry service absent
- **WHEN** the connected device does not expose the telemetry service UUID
- **THEN** the system reports "unsupported device" in the connection status and stops retrying until the user reconnects manually

### Requirement: Record validation
The system SHALL reject a frame whose received length is not 48 bytes, version is not `0x02`, declared frame size is not 48, snapshot flags contain unknown bits, or typed values violate documented ranges. It SHALL report an unsupported/invalid protocol error rather than guessing field semantics.

#### Scenario: Invalid version frame
- **WHEN** a 48-byte frame arrives with version other than `0x02`
- **THEN** it is discarded and an unsupported-protocol error is surfaced

#### Scenario: Invalid declared size
- **WHEN** the received length is 48 but the declared frame size differs
- **THEN** the frame is discarded without changing published keyboard state

#### Scenario: Short frame
- **WHEN** a frame shorter than 48 bytes arrives
- **THEN** it is discarded and no state change is emitted

### Requirement: Full-state publication
The system SHALL publish the complete decoded keyboard state on every valid frame: pressed positions, active layers, changed/valid masks, effective modifiers, indicators, default layer, endpoint/profile, both batteries, split status, firmware drops, sequence, timestamp, snapshot flag, and connection status. Each publication SHALL replace prior state wholesale.

#### Scenario: Modifier state frame
- **WHEN** a frame contains LSHIFT and position 13 with valid position/modifier fields
- **THEN** frontend state contains both facts along with all other frame fields

#### Scenario: State replacement after gap
- **WHEN** frames are missed and the next valid frame arrives
- **THEN** state from that frame fully replaces frontend state without reconstructing missed events

#### Scenario: Optional field unavailable
- **WHEN** a valid frame clears the peripheral-battery validity bit
- **THEN** frontend state represents that field as unavailable rather than as a real zero-percent value

### Requirement: Sequence gap detection
The system SHALL compare 32-bit sequence numbers modulo 2^32 for consecutive non-snapshot frames and expose the cumulative count of missing revisions. A snapshot SHALL establish a new baseline without creating a gap.

#### Scenario: Gap detected
- **WHEN** the previous sequence was 100 and the next non-snapshot frame carries 103
- **THEN** the gap counter increases by 2 and the newest complete state is still published

#### Scenario: Reconnect snapshot
- **WHEN** a snapshot arrives after reconnect or firmware reboot
- **THEN** its sequence becomes the new baseline and no cross-session gap is counted

### Requirement: Automatic reconnection
The system SHALL detect keyboard disconnection, retry with backoff, re-subscribe on success, obtain a fresh snapshot frame, and continuously publish connected/connecting/disconnected transitions.

#### Scenario: Keyboard sleeps and wakes
- **WHEN** the keyboard disconnects due to idle sleep and later reconnects
- **THEN** the system restores its subscription and publishes the new snapshot baseline without user action

#### Scenario: Status transitions visible
- **WHEN** connection state changes
- **THEN** the new state is published for overlay and tray presentation
