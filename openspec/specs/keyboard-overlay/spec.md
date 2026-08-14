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
The system SHALL render its own keycaps from parsed geometry (not the SVG's
embedded styling) in a dark translucent visual style, displaying each key's
tap label and hold sub-label when present.

#### Scenario: Layer rendered
- **WHEN** a keymap is loaded and a layer is active
- **THEN** all positions of that layer render as keycaps with their labels at the geometry-derived positions

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
The system SHALL display a connection status indicator on the overlay
reflecting connected, reconnecting/connecting, and disconnected states.

#### Scenario: Keyboard disconnects
- **WHEN** the backend publishes a disconnected state
- **THEN** the overlay indicator shows the disconnected state until reconnection
