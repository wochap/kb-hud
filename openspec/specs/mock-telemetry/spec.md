# mock-telemetry Specification

## Purpose

Development panel in the settings window that injects synthetic telemetry
through the same state publication path as real BLE records, so all overlay
behavior can be developed and tested without Bluetooth hardware or a BlueZ
system bus.

## Requirements

### Requirement: Mock telemetry dev panel
The settings window SHALL inject synthetic 48-byte telemetry frames through the same encoder, decoder, hub, and publication path as BLE. It SHALL support key press/release, random bursts, layer hold/release, modifier toggles, representative optional state, sequence gaps, firmware drops, and disconnect/reconnect simulation.

#### Scenario: Single mock press
- **WHEN** the user triggers a mock press of position 15
- **THEN** the overlay highlights position 15 exactly as it would for a real telemetry frame

#### Scenario: Mock modifier hold
- **WHEN** the user presses a home-row modifier position and enables its matching modifier bit
- **THEN** the overlay shows the active modifier, resolved-hold styling, and shifted labels through the production path

#### Scenario: Mock optional state
- **WHEN** the user configures valid endpoint, battery, indicator, or split values
- **THEN** the overlay presents them exactly as equivalent BLE state

#### Scenario: Mock sequence gap
- **WHEN** the user injects a sequence gap
- **THEN** the gap counter increases and behavior matches a real BLE gap

#### Scenario: Mock disconnect
- **WHEN** the user triggers a disconnect simulation
- **THEN** overlay and tray remain disconnected until mock reconnect

### Requirement: Mock mode independence from BLE
The mock panel SHALL function without Bluetooth, D-Bus, or hardware and SHALL NOT require the real telemetry connection to be active. Every mock state change SHALL still traverse telemetry-frame encoding and validation.

#### Scenario: Sandbox development
- **WHEN** the app runs without BlueZ access
- **THEN** all telemetry overlay behavior remains exercisable through the mock panel
