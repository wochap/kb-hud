# kb-hud

A transparent HUD overlay that visualizes the Chocofi split keyboard in real
time: key presses light up as you type, with modifier-aware labels, layer
badges, transparent-key resolution, and keyboard status. Telemetry arrives over
BLE from the keyboard's ZMK firmware (`zmk-key-telemetry` module, protocol v2).

Built with Tauri 2 + React 19. Designed for NixOS + Hyprland.

## Features

- Transparent, unfocusable overlay window rendering custom dark-glass
  keycaps from a keymap-drawer SVG (`corne.svg`)
- Effective-layer selection and `▽` (transparent key) resolution down the
  active-layer stack
- Authoritative LCTL/LSFT/LALT/LGUI/RCTL/RSFT/RALT/RGUI badges and visual
  confirmation when a physically held home-row key has resolved as a modifier
- US Shift label preview, including `/` becoming `?`, without changing the
  firmware's physical-position protocol
- Pressed-key highlight with minimum visibility and fade-out
- BlueZ GATT connection with auto-detection (paired device alias
  `Chocochap`), reconnect-with-backoff after keyboard sleep, and
  32-bit sequence-gap detection
- Validity-aware display of endpoint/profile, HID indicators, central and
  peripheral battery, split status, client gaps, and firmware-dropped frames
- Profiles (SVG path, device MAC or `auto`, scale) persisted as JSON
- System tray (StatusNotifier): open settings, connection status, quit
- Mock telemetry dev panel in settings — every overlay behavior is
  exercisable without Bluetooth hardware

## Development

Requires Nix with flakes. The devShell provides the Tauri Linux build
dependencies (webkitgtk_4_1, gtk3, librsvg, libayatana-appindicator, ...):

```sh
nix develop
bun install
bun run tauri dev
```

Tests:

```sh
bun run test      # frontend (vitest: SVG parser, layers, modifiers/labels)
bun run build     # TypeScript and production frontend build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Real BLE requires a BlueZ system D-Bus — it only works on the host, not in
a container sandbox. In the sandbox the app reports the missing bus in the
connection status and the mock panel covers everything downstream.

## Telemetry compatibility

kb-hud strictly accepts the authoritative 48-byte protocol-v2 state frame. It
does not decode the former 20-byte protocol-v1 records, so update kb-hud and
both keyboard firmware images as one coordinated deployment. A notification
needs ATT MTU 51 (48-byte value plus the 3-byte ATT notification header). The
firmware checks the active connection at runtime; the tested BlueZ/ZMK setup
negotiates ATT MTU 65 and therefore supports a 62-byte notification value.

Frames contain complete state rather than UI labels. kb-hud uses
`valid_fields` before displaying optional keyboard state, treats snapshot
frames as sequence baselines, and reports both sequence gaps and the firmware's
cumulative notification-drop counter. Protocol decode failures surface as BLE
connection errors instead of being guessed or partially applied.

## Hyprland integration

kb-hud does not talk to the compositor. Positioning, focus prevention, and
show/hide are Hyprland windowrules on the overlay's stable window class
`kb-hud`. Add to your Hyprland config:

```ini
# never steal focus, float, place bottom-center
windowrulev2 = nofocus, class:^(kb-hud)$
windowrulev2 = float, class:^(kb-hud)$
windowrulev2 = move 50%-w/2 100%-h-48, class:^(kb-hud)$

# live on a special workspace; toggle with a keybinding
windowrulev2 = workspace special:kb-hud silent, class:^(kb-hud)$
bind = SUPER, K, togglespecialworkspace, kb-hud
```

Adjust the binding and the bottom margin (`-48`) to taste. With the special
workspace rule active, `SUPER+K` (or your binding) shows and hides the
overlay; the app never controls its own visibility.

The tray icon needs a StatusNotifier host (e.g. QuickShell's SNI module or
any panel hosting SNI). Without one the app remains fully usable through
its windows.

## Configuration

Profiles live in `~/.config/com.wochap.kb-hud/profiles.json` (Tauri config
dir). Each profile: `name`, `svgPath`, `deviceMac` (`"auto"` or an explicit
MAC), `scale`. The active profile is applied to the overlay immediately on
change.

## License

MIT
