# profiles-config Specification

## Purpose

Profile-based configuration: the profile data model (keymap SVG path,
device selection, scale), JSON persistence in the user config directory,
and the settings UI for managing profiles, SVG paths, and device selection.

## Requirements

### Requirement: Profile data model
The system SHALL support profiles, each containing: a name, a keymap SVG
path, a device selection (either a Bluetooth MAC address or `auto`), a
render scale factor, an HUD visibility block with one boolean per top bar
pill category (layer, connection, gaps, firmware drops, battery, transport,
modifiers), and an overlay appearance block containing idle-key-background
visibility plus label, idle-background, key-border, active-key-background,
and top-bar-pill-background opacity values. Exactly one profile SHALL be
active at a time. Profiles persisted without an HUD or overlay appearance
block SHALL receive defaults that preserve the pre-change visible HUD and
overlay appearance.

#### Scenario: Profile fields
- **WHEN** a profile is created
- **THEN** it stores name, svgPath, deviceMac or `auto`, scale, HUD visibility toggles, and overlay appearance controls

#### Scenario: Legacy profile loads
- **WHEN** a persisted profile has no HUD visibility or overlay appearance block
- **THEN** it loads with all pill toggles enabled, idle key backgrounds enabled, and opacity defaults matching the prior overlay appearance

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

### Requirement: Overlay appearance settings UI
The settings window SHALL provide per-profile controls for showing idle key backgrounds and for label, idle key background, key border, active key background, and top-bar pill background opacity. Changes SHALL be persisted through profile configuration and reflected by the running overlay without restart.

#### Scenario: Disable idle backgrounds
- **WHEN** the user disables idle key backgrounds for the active profile
- **THEN** the setting is persisted and idle key fills disappear immediately without changing the saved idle-background opacity

#### Scenario: Adjust an opacity
- **WHEN** the user changes an overlay opacity slider
- **THEN** the normalized value is persisted for the active profile and the corresponding overlay elements update without restart

#### Scenario: Switch profiles
- **WHEN** the user activates a profile with different overlay appearance settings
- **THEN** the overlay immediately applies that profile's background visibility and opacity values

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
address, SHALL describe `auto` as discovery of a compatible
`zmk-key-telemetry` keyboard without naming a particular deployment, and
SHALL list paired BlueZ devices (name + MAC) discovered by the backend to
assist explicit selection.

#### Scenario: Generic auto option shown
- **WHEN** the user opens device selection
- **THEN** the `auto` option describes compatible telemetry keyboard discovery and contains no deployment-specific Bluetooth alias

#### Scenario: Device list shown
- **WHEN** the user requests the paired-device list
- **THEN** paired BlueZ devices are listed with names and MAC addresses

### Requirement: HUD visibility settings UI
The settings window SHALL provide a section listing each top bar pill
category with a toggle, persisting changes to the active profile via the
existing profile-update mechanism. Changes SHALL apply to the running
overlay without a restart.

#### Scenario: Toggle a pill off
- **WHEN** the user disables the battery toggle in settings
- **THEN** the profile's HUD block is updated and the overlay stops showing battery pills immediately

#### Scenario: Toggle a pill on
- **WHEN** the user re-enables a previously disabled pill toggle
- **THEN** the corresponding pills reappear in the overlay once their data is valid
