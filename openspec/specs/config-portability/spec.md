# config-portability Specification

## Purpose

TBD - Defines portable, versioned configuration export and validated,
atomic replace-all import.

## Requirements

### Requirement: Portable configuration export
The settings window SHALL export a single JSON document with a format discriminator and schema version containing the global palette assignments, active profile name, and every profile's name, scale, HUD visibility, overlay appearance, and embedded keymap SVG. A profile without a configured keymap SHALL be represented with a null keymap. The export SHALL NOT contain Bluetooth MAC addresses or original keymap filesystem paths.

#### Scenario: Export configured profiles
- **WHEN** the user exports configuration containing profiles with readable keymap SVGs
- **THEN** the resulting versioned JSON embeds each SVG and contains all portable global and profile settings without device addresses or original paths

#### Scenario: Export unconfigured keymap
- **WHEN** a profile has no configured keymap path
- **THEN** that profile is exported with a null keymap

#### Scenario: Configured keymap cannot be read
- **WHEN** a profile has a non-empty keymap path that cannot be read
- **THEN** export fails with an error identifying the profile and no incomplete export is produced

### Requirement: Import validation and confirmation
The settings window SHALL validate an entire portable export before replacement, SHALL display a summary of its profiles, active profile, palette assignments, and keymap results, and SHALL require explicit user confirmation that all current configuration will be replaced. Unsupported versions, invalid values, unknown version-1 theme IDs, duplicate or missing profile names, invalid active-profile references, and invalid embedded SVGs SHALL prevent confirmation and leave current configuration unchanged.

#### Scenario: Valid import preview
- **WHEN** the user selects a valid supported export
- **THEN** the settings window shows a successful replacement summary and enables explicit confirmation

#### Scenario: Invalid import
- **WHEN** any part of a selected export fails validation
- **THEN** the settings window reports the validation error, does not offer a successful replacement action, and preserves current configuration

#### Scenario: User cancels replacement
- **WHEN** a valid export is previewed but the user cancels the confirmation
- **THEN** current configuration remains unchanged

### Requirement: Atomic replace-all import
After explicit confirmation, the system SHALL re-read and re-validate the export, store embedded SVGs under app-managed safe paths, replace the active configuration as one committed operation, set every imported profile's device selection to `auto`, and notify both windows to reload. A failure before configuration commit SHALL leave the previous active configuration unchanged.

#### Scenario: Successful replacement
- **WHEN** the user confirms a valid import
- **THEN** all previous profiles and global appearance settings are replaced, the imported active profile is restored, imported keymaps use app-managed paths, and every profile uses automatic device discovery

#### Scenario: Commit fails
- **WHEN** writing or validating the import fails before the replacement configuration is committed
- **THEN** the application continues using the complete previous configuration rather than a partial import

#### Scenario: Imported configuration becomes active
- **WHEN** a replace-all import commits successfully
- **THEN** settings and overlay reload immediately and BLE reconnects using automatic discovery
