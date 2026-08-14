# keyboard-overlay Specification

## Purpose

Renders the keyboard HUD overlay: parses keymap-drawer SVGs into geometry,
draws custom keycaps, selects the effective layer, resolves transparent
keys through the layer stack, highlights pressed keys with decay, and shows
layer/connection status indicators.

## Requirements

### Requirement: SVG keymap parsing
The system SHALL parse a keymap-drawer SVG to extract, per layer group
(`layer-*` class), per key position (`keypos-N` class): the key transform
(translate and any rotation), key rectangle dimensions, and tap/hold label
text. The parser SHALL validate that every layer group contains the same
set of key positions and SHALL report precise parse errors rather than
rendering partial results.

#### Scenario: Valid keymap SVG
- **WHEN** the user selects a keymap-drawer SVG with 6 layer groups each containing keypos-0 through keypos-41
- **THEN** the parser produces a complete geometry model for all layers and positions

#### Scenario: Malformed SVG
- **WHEN** the selected file lacks layer groups or a layer group is missing key positions present in other layers
- **THEN** the settings UI displays a parse error identifying the problem and the overlay keeps its last valid keymap

### Requirement: Keycap rendering
The system SHALL render its own keycaps from parsed geometry using semantic colors from the active application palette, displaying each resolved tap label and hold sub-label. Tap and hold labels SHALL use a theme-derived contrasting shadow. When a pressed key's normalized hold label matches an active effective modifier, its hold role SHALL receive distinct resolved-modifier styling.

#### Scenario: Layer rendered
- **WHEN** a keymap is loaded and a layer is active
- **THEN** all positions render as theme-colored keycaps with resolved labels and contrasting label shadows at geometry-derived positions

#### Scenario: Home-row hold resolves
- **WHEN** a pressed home-row key has hold label `LSHIFT` and telemetry reports LSHIFT active
- **THEN** that keycap visually promotes its hold role with resolved-modifier theme colors rather than appearing only as a physically held tap key

### Requirement: Configurable key appearance
The overlay SHALL apply the active profile's idle-key-background visibility and its label, idle key background, key border, and active key background opacity values. Disabling idle key backgrounds SHALL affect only idle fills; ordinary pressed-key backgrounds and resolved-modifier backgrounds SHALL remain independently rendered using the active-key opacity and their distinct theme colors. Press-decay intensity SHALL multiply the configured active-key opacity.

#### Scenario: Backgrounds disabled during ordinary press
- **WHEN** idle key backgrounds are disabled and an ordinary key is pressed
- **THEN** idle fills remain transparent while the pressed key displays its themed active background at the configured active-key opacity

#### Scenario: Backgrounds disabled during resolved modifier
- **WHEN** idle key backgrounds are disabled and a pressed hold resolves to an active modifier
- **THEN** that key displays the distinct resolved-modifier background at the configured active-key opacity

#### Scenario: Restore idle background
- **WHEN** the user disables and later re-enables idle key backgrounds
- **THEN** idle fills return at the previously configured idle-key-background opacity

#### Scenario: Highlight decay respects opacity
- **WHEN** a released key highlight fades
- **THEN** its animation intensity decays from the profile's configured active-key opacity

### Requirement: Configurable top-bar pill backgrounds
The overlay SHALL apply the active profile's top-bar pill background opacity to the fills behind layer, telemetry, modifier, indicator, gap, firmware-drop, and connection-error pills. This opacity SHALL NOT alter pill text, pill borders, or the connection-status dot.

#### Scenario: Reduce pill background opacity
- **WHEN** the user lowers top-bar pill background opacity
- **THEN** pill fills become more transparent while their text, borders, and connection-status dot retain their own theme styling

### Requirement: Effective layer selection
The system SHALL determine the effective layer as the highest set bit of
the active-layer mask, mapping layer indices to the SVG layer-group order,
SHALL render that layer's resolved labels, and SHALL display the effective
layer's name as a badge on the overlay.

#### Scenario: Multiple active layers
- **WHEN** the active-layer mask is `0x00000009` (bits 0 and 3) and layer order is colemakdh, querty, num, nav, fn, adjust
- **THEN** the overlay renders layer index 3 (nav) and the badge displays its name

#### Scenario: Default layer only
- **WHEN** the active-layer mask is `0x00000001`
- **THEN** the overlay renders layer index 0 and the badge displays its name

### Requirement: Transparent key resolution
The system SHALL resolve keys whose label on the effective layer is
transparent (trans/`▽`) downward through the active-layer stack to the
first non-transparent label among lower active layers, displaying an empty
keycap if no active layer provides one.

#### Scenario: Trans key resolves through active layer
- **WHEN** position 36 is `▽` on the effective layer `num` and layer `colemakdh` (active, lower) has label `LSHIFT` at position 36
- **THEN** the keycap displays `LSHIFT`

#### Scenario: Trans key with no active lower label
- **WHEN** a trans key has no non-transparent label on any lower active layer
- **THEN** the keycap renders empty without error

### Requirement: Pressed key highlight with decay
The system SHALL highlight pressed positions on the rendered layer at
key-down, SHALL keep a released key highlighted until at least ~120 ms of
total press visibility has elapsed, and SHALL fade the highlight out over
~150 ms afterward.

#### Scenario: Fast tap
- **WHEN** a key is pressed and released after 70 ms
- **THEN** the key remains fully highlighted until 120 ms from press, then fades out over ~150 ms

#### Scenario: Held key
- **WHEN** a key stays pressed for 2 seconds
- **THEN** the highlight persists for the entire hold and fades out ~150 ms after release

### Requirement: Connection status indicator
The system SHALL display connection status and compact valid keyboard status including active modifiers, endpoint/profile, batteries, split connection, active lock indicators, sequence gaps, and firmware drop count. Optional items SHALL be hidden when their validity bits are clear. Each pill category (layer badge, connection dot and error message, gaps, firmware drops, battery levels, transport, active modifiers) SHALL additionally be hidden when its HUD visibility toggle is off. All displayed pills SHALL be vertically centered on a common midline within the top bar.

#### Scenario: Keyboard disconnects
- **WHEN** backend state is disconnected
- **THEN** the overlay shows disconnected until reconnection

#### Scenario: Optional battery unavailable
- **WHEN** peripheral-battery validity is clear
- **THEN** no right-battery percentage is displayed

#### Scenario: Active modifiers
- **WHEN** telemetry reports multiple modifier bits
- **THEN** the overlay displays each active left/right modifier distinctly

#### Scenario: Pill toggle off
- **WHEN** the gaps HUD toggle is off and telemetry reports gaps > 0
- **THEN** the overlay renders no gaps pill

#### Scenario: Connection toggle off
- **WHEN** the connection HUD toggle is off
- **THEN** neither the status dot nor any connection error message is rendered

#### Scenario: Vertical centering
- **WHEN** the top bar renders any combination of pills
- **THEN** every pill's visual center lies on the same horizontal midline

### Requirement: US Shift label preview
The overlay SHALL apply an isolated US-keyboard Shift mapping to printable tap labels whenever LSHIFT or RSHIFT is active. It SHALL transform ASCII letters, digits, and standard punctuation while leaving non-printable, behavior, media, and already-shifted-symbol labels unchanged.

#### Scenario: Shift slash
- **WHEN** either Shift modifier is active and a resolved key tap label is `/`
- **THEN** that keycap displays `?`

#### Scenario: Shift letter
- **WHEN** Shift is active and a resolved tap label is `a`
- **THEN** that keycap displays `A`

#### Scenario: Non-printable label
- **WHEN** Shift is active and a key label is `ENTER`, `F1`, or `VOL UP`
- **THEN** its label remains unchanged

#### Scenario: Shift released
- **WHEN** neither Shift modifier is active
- **THEN** each keycap displays its ordinary resolved tap label
