import { readFileSync } from "node:fs";
import { join } from "node:path";

import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { parseKeymapSvg } from "../../keymap/parser";
import { MOD_LSFT, type TelemetryState } from "../../telemetry";
import { OverlayView } from "../OverlayView";

const keymap = parseKeymapSvg(
  readFileSync(join(__dirname, "../../keymap/__tests__/corne.svg"), "utf-8"),
);

function state(
  activeLayers: number,
  pressed: number[] = [],
  patch: Partial<TelemetryState> = {},
): TelemetryState {
  return {
    connection: "connected",
    snapshot: false,
    pressed,
    activeLayers,
    changedFields: 0,
    validFields: 0,
    modifiers: 0,
    sequence: 1,
    timestampMs: 0,
    firmwareDrops: 0,
    gaps: 0,
    ...patch,
  };
}

describe("OverlayView rendering", () => {
  it("renders all keycaps on the default layer", () => {
    const html = renderToString(<OverlayView keymap={keymap} state={state(1)} />);
    expect(html.match(/class="keycap"/g)?.length).toBe(42);
  });

  it("renders all keycaps with a held layer (mask 0b1001)", () => {
    const html = renderToString(
      <OverlayView keymap={keymap} state={state(0b1001)} />,
    );
    expect(html.match(/class="keycap"/g)?.length).toBe(42);
  });

  it("shows the effective layer name in the badge", () => {
    const html = renderToString(
      <OverlayView keymap={keymap} state={state(0b1001)} />,
    );
    expect(html).toContain("nav");
  });

  it("renders labeled caps when base layer is also active", () => {
    const html = renderToString(
      <OverlayView keymap={keymap} state={state(0b1001)} />,
    );
    // nav resolves trans keys down to colemakdh: labels must be present
    expect(html).toContain("F1");
    expect(html.match(/class="keycap keycap-empty"/g)?.length ?? 0).toBeLessThan(42);
  });

  it("renders nav labels and two empty caps when only nav is active", () => {
    const html = renderToString(
      <OverlayView keymap={keymap} state={state(0b1000)} />,
    );
    expect(html).toContain("F1");
    expect(html.match(/class="keycap keycap-empty"/g)?.length).toBe(2);
  });

  it("shows shifted slash and resolved home-row Shift hold", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state(1, [16], { modifiers: MOD_LSFT })}
      />,
    );
    expect(html).toContain(">?<");
    expect(html).toContain("modifier-resolved");
    expect(html).toContain("LSHIFT");
  });

  it("returns to ordinary labels after Shift release", () => {
    const html = renderToString(
      <OverlayView keymap={keymap} state={state(1, [])} />,
    );
    expect(html).toContain(">/<");
    expect(html).not.toContain("modifier-resolved");
  });

  it("shows only valid optional status supplied by the backend", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state(1, [], {
          transport: "ble",
          bleProfile: 2,
          centralBatteryPct: 91,
          peripheralBatteryPct: 84,
          splitStatus: "connected",
          hidIndicators: 2,
          firmwareDrops: 3,
        })}
      />,
    );
    expect(html).toContain("BLE 2");
    expect(html).toContain("L 91%");
    expect(html).toContain("R 84%");
    expect(html).toContain("split up");
    expect(html).toContain("CAPS");
    expect(html).toContain("fw drops 3");
  });
});
