# mock-telemetry Specification

## Purpose

Development panel in the settings window that injects synthetic telemetry
through the same state publication path as real BLE records, so all overlay
behavior can be developed and tested without Bluetooth hardware or a BlueZ
system bus.

## Requirements

### Requirement: Mock telemetry dev panel
The settings window SHALL include a development panel that injects
synthetic telemetry through the same state publication path as BLE records,
supporting: a single key press/release at a chosen position, a burst of
random presses, a layer hold and release, a sequence-gap injection, and a
disconnect/reconnect simulation.

#### Scenario: Single mock press
- **WHEN** the user triggers a mock press of position 15 from the dev panel
- **THEN** the overlay highlights position 15 exactly as it would for a real BLE record

#### Scenario: Mock layer hold
- **WHEN** the user holds a mock layer from the dev panel
- **THEN** the overlay switches to that layer's effective rendering with resolved trans keys

#### Scenario: Mock sequence gap
- **WHEN** the user injects a sequence gap from the dev panel
- **THEN** the gap counter increases and the behavior matches a real BLE gap

#### Scenario: Mock disconnect
- **WHEN** the user triggers a disconnect simulation
- **THEN** the overlay and tray show the disconnected state until a reconnect simulation is triggered

### Requirement: Mock mode independence from BLE
The mock panel SHALL function without any Bluetooth stack, D-Bus system
bus, or hardware present, and SHALL NOT require the real telemetry
connection to be active.

#### Scenario: Sandbox development
- **WHEN** the app runs in an environment without BlueZ access
- **THEN** all overlay behavior remains fully exercisable through the mock panel
