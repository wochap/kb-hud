# application-theming Specification

## Purpose

TBD - Defines system-appearance following, bundled palettes, global palette
assignments, and shared semantic theme tokens across application windows.

## Requirements

### Requirement: System appearance following
The system SHALL always follow the GTK/system light or dark appearance, SHALL apply the palette assigned to the current appearance, and SHALL update both the settings and overlay windows without an application restart when the system appearance changes. The system SHALL use dark appearance when the platform does not report a known appearance.

#### Scenario: System changes to dark
- **WHEN** GTK/system appearance changes from light to dark
- **THEN** each open application window applies the configured dark-mode palette without restarting

#### Scenario: System appearance unavailable
- **WHEN** the platform does not report light or dark appearance
- **THEN** the application uses its configured dark-mode palette

### Requirement: Bundled Catppuccin palettes
The system SHALL bundle Catppuccin Latte, Frappé, Macchiato, and Mocha using Blue as the primary accent in every flavor. First launch and legacy configuration without appearance settings SHALL assign Latte to light mode and Mocha to dark mode.

#### Scenario: First-launch defaults
- **WHEN** the application creates configuration without saved appearance settings
- **THEN** light mode uses Latte, dark mode uses Mocha, and both use Blue as the primary accent

#### Scenario: Any bundled flavor can be assigned
- **WHEN** the user opens either the light-mode or dark-mode theme selector
- **THEN** Latte, Frappé, Macchiato, and Mocha are all available choices

### Requirement: Global palette assignments
The settings window SHALL provide separate select controls for the light-mode and dark-mode palettes, SHALL persist both assignments globally rather than in a profile, and SHALL apply selection changes immediately to every open window using the affected system appearance.

#### Scenario: Change active appearance palette
- **WHEN** the system is in dark mode and the user changes the dark-mode selection from Mocha to Macchiato
- **THEN** the settings and overlay windows immediately use Macchiato and the selection survives restart

#### Scenario: Switch keyboard profile
- **WHEN** the user selects a different keyboard profile
- **THEN** the global light-mode and dark-mode palette assignments remain unchanged

### Requirement: Shared semantic theme tokens
The settings and overlay windows SHALL derive their colors from the same active Catppuccin palette through semantic theme tokens. The tokens SHALL cover settings surfaces and controls as well as idle keys, borders, labels, pressed keys, resolved modifiers, top-bar pills, status colors, and key-label shadow.

#### Scenario: Palette applies across windows
- **WHEN** a palette becomes active
- **THEN** settings surfaces and every themed overlay element use semantic colors derived from that palette rather than independent hard-coded color sets

#### Scenario: Contrasting label shadow
- **WHEN** tap and hold labels are rendered for the active palette
- **THEN** each label uses a theme-derived shadow color that contrasts with its text color
