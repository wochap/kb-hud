# window-behavior Specification

## Purpose

Window lifecycle and platform integration: the transparent, unfocusable
overlay window, the settings window, profile-driven sizing, tray presence,
launch behavior, and the Hyprland integration contract that delegates
visibility toggling to the compositor.

## Requirements

### Requirement: Transparent overlay window
The system SHALL create the overlay window without decorations, excluded
from the taskbar, not requesting focus, with a fully transparent background
so only rendered keycaps are visible. If the runtime platform cannot render
app-blended transparency (determined by the compositing spike), the system
SHALL fall back to an opaque dark background with a documented
compositor-level opacity rule instead.

#### Scenario: Window created without stealing focus
- **WHEN** the overlay window is shown while another application has focus
- **THEN** the existing application keeps focus

#### Scenario: Transparent background
- **WHEN** the overlay renders with no keys pressed
- **THEN** the desktop behind the window is visible everywhere except the status indicator and layer badge

### Requirement: Overlay sizing from profile scale
The system SHALL size the overlay window as the parsed keymap's native
bounds multiplied by the active profile's scale factor, and SHALL resize
the window when the scale changes.

#### Scenario: Scale change
- **WHEN** the user changes the active profile scale from 1.0 to 1.5
- **THEN** the overlay window resizes to 1.5 times the native keymap dimensions

### Requirement: Settings window
The system SHALL provide a separate, normally decorated settings window,
independent of overlay visibility, for profile management, global light/dark
palette selection, device selection, scale and HUD adjustment, overlay
appearance controls, portable configuration export/import, and the mock
telemetry dev panel. The settings interface SHALL use Tailwind CSS and
shadcn/ui components and SHALL render with the same active semantic palette
as the overlay.

#### Scenario: Open settings from tray
- **WHEN** the user activates the tray's settings entry
- **THEN** the themed settings window opens or is brought forward without hiding the overlay

#### Scenario: Settings controls use shared theme
- **WHEN** the active system appearance or assigned palette changes
- **THEN** settings surfaces and controls update through the shared semantic theme without affecting window independence

### Requirement: Tray presence
The system SHALL provide a StatusNotifier tray icon offering: open
settings, current connection status, and quit. The app SHALL NOT attempt to
control overlay visibility itself; visibility toggling belongs to the
compositor.

#### Scenario: Tray shows connection status
- **WHEN** the telemetry connection state changes
- **THEN** the tray menu/tooltip reflects the new state

### Requirement: Hyprland integration contract
The overlay window SHALL expose a stable window class `kb-hud` so Hyprland
windowrules can match it. The project documentation SHALL provide the
required Hyprland configuration: `nofocus` rule, floating rule, position
rule (bottom center), special-workspace assignment, and a keybinding to
toggle that workspace. The app SHALL NOT attempt to position or toggle
itself via compositor IPC.

#### Scenario: Stable window class
- **WHEN** the overlay window is created on any launch
- **THEN** Hyprland sees window class `kb-hud`

### Requirement: Launch behavior
The system SHALL auto-connect to the active profile's device on launch and
SHALL show the overlay window on launch.

#### Scenario: Launch with saved profile
- **WHEN** the app launches with a saved active profile pointing at a reachable keyboard
- **THEN** the overlay is visible and the connection reaches the connected state without user interaction
