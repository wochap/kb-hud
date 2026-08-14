import type { KeyGeometry, KeymapGeometry } from "../keymap/parser";
import { effectiveLayerIndex, resolveKey } from "../keymap/resolve";
import { DEFAULT_HUD_VISIBILITY, type HudVisibility } from "../profile";
import {
  MOD_LALT,
  MOD_LCTL,
  MOD_LGUI,
  MOD_LSFT,
  MOD_RALT,
  MOD_RCTL,
  MOD_RGUI,
  MOD_RSFT,
  type TelemetryState,
} from "../telemetry";
import { OVERLAY_PADDING } from "./sizing";
import { shiftedTapLabel } from "./shiftLabels";
import { usePressHighlight } from "./usePressHighlight";
import "./overlay.css";

function matrixAttr(key: KeyGeometry): string {
  const { a, b, c, d, e, f } = key.transform;
  return `matrix(${a} ${b} ${c} ${d} ${e} ${f})`;
}

function tapFontSize(label: string): number {
  if (label.length <= 4) return 14;
  return Math.max(7, (14 * 4) / label.length);
}

function holdFontSize(label: string): number {
  if (label.length <= 5) return 11;
  return Math.max(6, (11 * 5) / label.length);
}

interface KeycapProps {
  geometry: KeyGeometry;
  tap: string;
  hold: string;
  empty: boolean;
  intensity: number;
  modifierResolved: boolean;
}

function Keycap({
  geometry: k,
  tap,
  hold,
  empty,
  intensity,
  modifierResolved,
}: KeycapProps) {
  const { x, y, width, height, rx } = k.rect;
  return (
    <g
      transform={matrixAttr(k)}
      className={modifierResolved ? "keycap-group modifier-resolved" : "keycap-group"}
    >
      <rect
        x={x}
        y={y}
        width={width}
        height={height}
        rx={rx}
        className={empty ? "keycap keycap-empty" : "keycap"}
      />
      {intensity > 0 && (
        <rect
          x={x}
          y={y}
          width={width}
          height={height}
          rx={rx}
          className="keycap-highlight"
          style={{ opacity: intensity }}
        />
      )}
      {tap !== "" && (
        <text className="keycap-tap" fontSize={tapFontSize(tap)}>
          {tap}
        </text>
      )}
      {hold !== "" && (
        <text className="keycap-hold" y={height / 2 - 2} fontSize={holdFontSize(hold)}>
          {hold}
        </text>
      )}
    </g>
  );
}

const STATUS_CLASS: Record<TelemetryState["connection"], string> = {
  connected: "status-connected",
  connecting: "status-connecting",
  disconnected: "status-disconnected",
};

const MODIFIERS = [
  [MOD_LCTL, "LCTRL"],
  [MOD_LSFT, "LSHIFT"],
  [MOD_LALT, "LALT"],
  [MOD_LGUI, "LGUI"],
  [MOD_RCTL, "RCTRL"],
  [MOD_RSFT, "RSHIFT"],
  [MOD_RALT, "RALT"],
  [MOD_RGUI, "RGUI"],
] as const;

const HOLD_ALIASES: Readonly<Record<string, number>> = {
  LCTL: MOD_LCTL,
  LCTRL: MOD_LCTL,
  LSFT: MOD_LSFT,
  LSHIFT: MOD_LSFT,
  LALT: MOD_LALT,
  LGUI: MOD_LGUI,
  RCTL: MOD_RCTL,
  RCTRL: MOD_RCTL,
  RSFT: MOD_RSFT,
  RSHIFT: MOD_RSFT,
  RALT: MOD_RALT,
  RGUI: MOD_RGUI,
};

function holdModifierBit(label: string): number {
  return HOLD_ALIASES[label.toUpperCase().replace(/\s+/g, "")] ?? 0;
}

function indicatorNames(indicators: number): string[] {
  return ["NUM", "CAPS", "SCROLL", "COMPOSE", "KANA"].filter(
    (_, bit) => indicators & (1 << bit),
  );
}

export interface OverlayViewProps {
  keymap: KeymapGeometry | null;
  state: TelemetryState;
  error?: string | null;
  hud?: HudVisibility;
}

export function OverlayView({ keymap, state, error, hud = DEFAULT_HUD_VISIBILITY }: OverlayViewProps) {
  const intensities = usePressHighlight(state.pressed);

  if (!keymap) {
    return (
      <div className="overlay-empty">
        {error ?? "no keymap loaded"}
      </div>
    );
  }

  const effectiveIndex = effectiveLayerIndex(state.activeLayers);
  const layer = keymap.layers[effectiveIndex] ?? keymap.layers[0];
  const { minX, minY, width, height } = keymap.bounds;
  const activeModifiers = MODIFIERS.filter(([bit]) => state.modifiers & bit);
  const indicators = indicatorNames(state.hidIndicators ?? 0);

  return (
    <div className="overlay">
      <svg
        viewBox={`${minX - OVERLAY_PADDING} ${minY - OVERLAY_PADDING} ${
          width + OVERLAY_PADDING * 2
        } ${height + OVERLAY_PADDING * 2}`}
      >
        {[...layer.keys.values()].map((key) => {
          const resolved = resolveKey(keymap, key.position, state.activeLayers);
          const modifierBit = holdModifierBit(resolved.hold);
          return (
            <Keycap
              key={key.position}
              geometry={key}
              tap={shiftedTapLabel(resolved.tap, state.modifiers)}
              hold={resolved.hold}
              empty={resolved.empty}
              intensity={intensities.get(key.position) ?? 0}
              modifierResolved={
                state.pressed.includes(key.position) &&
                modifierBit !== 0 &&
                (state.modifiers & modifierBit) !== 0
              }
            />
          );
        })}
      </svg>
      <div className="overlay-hud">
        {hud.connection && state.error && (
          <span className="conn-error">{state.error}</span>
        )}
        {hud.modifiers &&
          activeModifiers.map(([, name]) => (
            <span className="modifier-badge" key={name}>{name}</span>
          ))}
        {indicators.map((name) => (
          <span className="indicator-badge" key={name}>{name}</span>
        ))}
        {hud.transport && state.transport && (
          <span className="telemetry-status output-status">
            {state.transport === "ble"
              ? `BLE ${state.bleProfile ?? "?"}`
              : "USB"}
          </span>
        )}
        {hud.battery && state.centralBatteryPct !== undefined && (
          <span className="telemetry-status">{`L ${state.centralBatteryPct}%`}</span>
        )}
        {hud.battery && state.peripheralBatteryPct !== undefined && (
          <span className="telemetry-status">{`R ${state.peripheralBatteryPct}%`}</span>
        )}
        {state.splitStatus && (
          <span className={`telemetry-status split-${state.splitStatus}`}>
            {`split ${state.splitStatus === "connected" ? "up" : "down"}`}
          </span>
        )}
        {hud.gaps && state.gaps > 0 && (
          <span className="gaps">{`gaps ${state.gaps}`}</span>
        )}
        {hud.firmwareDrops && state.firmwareDrops > 0 && (
          <span className="gaps">{`fw drops ${state.firmwareDrops}`}</span>
        )}
        {hud.layer && <span className="layer-badge">{layer.name}</span>}
        {hud.connection && (
          <span className={`status-dot ${STATUS_CLASS[state.connection]}`} />
        )}
      </div>
    </div>
  );
}
