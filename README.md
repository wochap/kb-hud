# kb-hud

A transparent HUD overlay that visualizes the Chocofi split keyboard in real
time: key presses light up as you type, with layer badges, transparent-key
resolution, and connection status. Telemetry arrives over BLE from the
keyboard's ZMK firmware (`zmk-key-telemetry` module, protocol v1).

Built with Tauri 2 + React 19. Designed for NixOS + Hyprland.

## Features

- Transparent, unfocusable overlay window rendering custom dark-glass
  keycaps from a keymap-drawer SVG (`corne.svg`)
- Effective-layer selection and `▽` (transparent key) resolution down the
  active-layer stack
- Pressed-key highlight with minimum visibility and fade-out
- BlueZ GATT connection with auto-detection (paired device alias
  `Chocochap`), reconnect-with-backoff after keyboard sleep, and
  sequence-gap detection
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
bun run test      # frontend (vitest: SVG parser, layer resolution)
cargo test        # backend (protocol decoder, gaps, config)
```

Real BLE requires a BlueZ system D-Bus — it only works on the host, not in
a container sandbox. In the sandbox the app reports the missing bus in the
connection status and the mock panel covers everything downstream.

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
