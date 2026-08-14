import type { KeyGeometry, KeymapGeometry } from "../keymap/parser";
import { effectiveLayerIndex, resolveKey } from "../keymap/resolve";
import type { TelemetryState } from "../telemetry";
import { OVERLAY_PADDING } from "./sizing";
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
}

function Keycap({ geometry: k, tap, hold, empty, intensity }: KeycapProps) {
  const { x, y, width, height, rx } = k.rect;
  return (
    <g transform={matrixAttr(k)}>
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

export interface OverlayViewProps {
  keymap: KeymapGeometry | null;
  state: TelemetryState;
  error?: string | null;
}

export function OverlayView({ keymap, state, error }: OverlayViewProps) {
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

  return (
    <div className="overlay">
      <svg
        viewBox={`${minX - OVERLAY_PADDING} ${minY - OVERLAY_PADDING} ${
          width + OVERLAY_PADDING * 2
        } ${height + OVERLAY_PADDING * 2}`}
      >
        {[...layer.keys.values()].map((key) => {
          const resolved = resolveKey(keymap, key.position, state.activeLayers);
          return (
            <Keycap
              key={key.position}
              geometry={key}
              tap={resolved.tap}
              hold={resolved.hold}
              empty={resolved.empty}
              intensity={intensities.get(key.position) ?? 0}
            />
          );
        })}
      </svg>
      <div className="overlay-hud">
        {state.error && <span className="conn-error">{state.error}</span>}
        {state.gaps > 0 && <span className="gaps">gaps {state.gaps}</span>}
        <span className="layer-badge">{layer.name}</span>
        <span className={`status-dot ${STATUS_CLASS[state.connection]}`} />
      </div>
    </div>
  );
}
