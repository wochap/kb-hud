# ble-telemetry Specification

## Purpose

BlueZ GATT connection lifecycle for the keyboard's BLE telemetry: device
auto-detection, protocol-v1 record validation and decoding, full-state
publication to the frontend, sequence-gap detection, and automatic
reconnection after keyboard sleep.

## Requirements

### Requirement: Device auto-detection
The system SHALL enumerate paired BlueZ devices and, when the active
profile's device selection is `auto`, select the unique device whose alias
matches the configured telemetry keyboard name (default `Chocochap`).

#### Scenario: Single matching paired device
- **WHEN** one paired BlueZ device has alias `Chocochap` and the profile device is `auto`
- **THEN** the system connects to that device

#### Scenario: No matching device
- **WHEN** no paired device matches and the profile device is `auto`
- **THEN** the system reports a "no telemetry keyboard found" error in the connection status without crashing

#### Scenario: Multiple matching devices
- **WHEN** more than one paired device matches and the profile device is `auto`
- **THEN** the system reports an ambiguity error and requires an explicit MAC address in the profile

### Requirement: GATT subscription
The system SHALL connect to the selected device, discover telemetry service
`9e7a7d70-df1b-4f76-9d45-8c3f4a6b2100`, subscribe to notifications on
characteristic `9e7a7d70-df1b-4f76-9d45-8c3f4a6b2101`, and perform an
initial characteristic read to obtain a snapshot.

#### Scenario: Successful subscription
- **WHEN** connection and discovery succeed
- **THEN** the system receives an initial 20-byte type-1 snapshot record and subsequent notifications

#### Scenario: Telemetry service absent
- **WHEN** the connected device does not expose the telemetry service UUID
- **THEN** the system reports "unsupported device" in the connection status and stops retrying until the user reconnects manually

### Requirement: Record validation
The system SHALL reject any record whose length is not 20 bytes or whose
version byte is not `0x01`, and SHALL report an unsupported protocol error
rather than guessing field semantics.

#### Scenario: Invalid version record
- **WHEN** a 20-byte record arrives with version byte `0x02`
- **THEN** the record is discarded and an unsupported-protocol error is surfaced in the connection status

#### Scenario: Short record
- **WHEN** a record shorter than 20 bytes arrives
- **THEN** the record is discarded and no state change is emitted

### Requirement: Full-state publication
The system SHALL publish the complete decoded keyboard state (pressed
position set, active layer mask, sequence number, and connection status) to
the frontend on every valid record, replacing prior state wholesale.

#### Scenario: Key event record
- **WHEN** a type-2 record with position 15 pressed arrives
- **THEN** the emitted state contains position 15 in the pressed set along with the record's layer mask and sequence

#### Scenario: State replacement after gap
- **WHEN** records are missed and the next valid record arrives
- **THEN** the emitted state from that record fully replaces frontend state without requiring the missed records

### Requirement: Sequence gap detection
The system SHALL compare sequence numbers modulo 65536 and SHALL expose the
cumulative count of detected gaps (modular difference greater than one) to
the frontend.

#### Scenario: Gap detected
- **WHEN** the previous sequence was 100 and the next valid record carries sequence 103
- **THEN** the gap counter increases by 2 and the record's state is still published

### Requirement: Automatic reconnection
The system SHALL detect keyboard disconnection (including sleep-initiated
disconnects), retry connection with backoff, re-subscribe on success, and
publish connection state transitions (connecting, connected, disconnected)
continuously.

#### Scenario: Keyboard sleeps and wakes
- **WHEN** the keyboard disconnects due to idle sleep and later reconnects to the host
- **THEN** the system re-establishes the GATT subscription without user action and publishes a fresh snapshot state

#### Scenario: Status transitions visible
- **WHEN** connection state changes
- **THEN** the new state (connecting, connected, disconnected) is published for display by the overlay and tray
