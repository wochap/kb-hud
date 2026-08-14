# profiles-config Specification

## Purpose

Profile-based configuration: the profile data model (keymap SVG path,
device selection, scale), JSON persistence in the user config directory,
and the settings UI for managing profiles, SVG paths, and device selection.

## Requirements

### Requirement: Profile data model
The system SHALL support profiles, each containing: a name, a keymap SVG
path, a device selection (either a Bluetooth MAC address or `auto`), and a
render scale factor. Exactly one profile SHALL be active at a time.

#### Scenario: Profile fields
- **WHEN** a profile is created
- **THEN** it stores name, svgPath, deviceMac or `auto`, and scale

### Requirement: Configuration persistence
The system SHALL persist profiles and the active profile selection as JSON
in the user configuration directory, load them at startup, and save changes
when they occur. First launch with no configuration file SHALL create a
default profile with `auto` device selection and scale 1.0.

#### Scenario: First launch
- **WHEN** the app starts and no configuration file exists
- **THEN** a default profile is created and persisted

#### Scenario: Edit survives restart
- **WHEN** the user edits the active profile's SVG path and restarts the app
- **THEN** the edited value is loaded at startup

### Requirement: Profile management UI
The settings window SHALL allow creating, renaming, deleting, and selecting
the active profile. Selecting a profile SHALL apply it immediately to the
overlay (keymap, device, scale).

#### Scenario: Switch profile
- **WHEN** the user selects a different profile
- **THEN** the overlay re-parses that profile's SVG, resizes to its scale, and connects to its device

### Requirement: SVG path selection with validation
The settings UI SHALL let the user set a profile's SVG path and SHALL
display parse-validation feedback (success summary: layer count and key
position count, or the specific parse error) immediately after selection.

#### Scenario: Valid SVG selected
- **WHEN** the user picks a valid keymap-drawer SVG
- **THEN** the settings UI reports the parsed layer and position counts

#### Scenario: Invalid SVG selected
- **WHEN** the user picks a file that fails keymap parsing
- **THEN** the settings UI displays the parse error and the profile keeps its previous SVG path

### Requirement: Device selection UI
The settings UI SHALL offer device selection as `auto` or an explicit MAC
address, and SHALL list paired BlueZ devices (name + MAC) discovered by the
backend to assist selection.

#### Scenario: Device list shown
- **WHEN** the user opens device selection
- **THEN** paired BlueZ devices are listed with names and MAC addresses
