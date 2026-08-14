import { readFileSync } from "node:fs";
import { join } from "node:path";

import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { parseKeymapSvg } from "../../keymap/parser";
import { DEFAULT_HUD_VISIBILITY, type HudVisibility } from "../../profile";
import { MOD_LSFT, type TelemetryState } from "../../telemetry";
import { OverlayView } from "../OverlayView";

const keymap = parseKeymapSvg(
  readFileSync(join(__dirname, "../../keymap/__tests__/corne.svg"), "utf-8"),
);

function state(patch: Partial<TelemetryState> = {}): TelemetryState {
  return {
    connection: "connected",
    snapshot: false,
    pressed: [],
    activeLayers: 1,
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

function hud(patch: Partial<HudVisibility> = {}): HudVisibility {
  return { ...DEFAULT_HUD_VISIBILITY, ...patch };
}

describe("OverlayView HUD visibility toggles", () => {
  it("renders all pills by default when telemetry supplies them", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state({
          error: "link lost",
          modifiers: MOD_LSFT,
          transport: "ble",
          bleProfile: 1,
          centralBatteryPct: 80,
          peripheralBatteryPct: 70,
          gaps: 2,
          firmwareDrops: 1,
        })}
      />,
    );
    expect(html).toContain("layer-badge");
    expect(html).toContain("status-dot");
    expect(html).toContain("conn-error");
    expect(html).toContain("modifier-badge");
    expect(html).toContain("gaps 2");
    expect(html).toContain("fw drops 1");
    expect(html).toContain("L 80%");
    expect(html).toContain("R 70%");
    expect(html).toContain("BLE 1");
  });

  it("hides the layer badge when layer toggle is off", () => {
    const html = renderToString(
      <OverlayView keymap={keymap} state={state()} hud={hud({ layer: false })} />,
    );
    expect(html).not.toContain("layer-badge");
  });

  it("connection toggle off hides both the status dot and error message", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state({ error: "link lost", connection: "disconnected" })}
        hud={hud({ connection: false })}
      />,
    );
    expect(html).not.toContain("status-dot");
    expect(html).not.toContain("conn-error");
  });

  it("hides gaps pill when gaps toggle is off", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state({ gaps: 4 })}
        hud={hud({ gaps: false })}
      />,
    );
    expect(html).not.toContain("gaps 4");
  });

  it("hides firmware drops pill when firmwareDrops toggle is off", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state({ firmwareDrops: 2 })}
        hud={hud({ firmwareDrops: false })}
      />,
    );
    expect(html).not.toContain("fw drops 2");
  });

  it("hides battery pills when battery toggle is off", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state({ centralBatteryPct: 55, peripheralBatteryPct: 60 })}
        hud={hud({ battery: false })}
      />,
    );
    expect(html).not.toContain("L 55%");
    expect(html).not.toContain("R 60%");
  });

  it("hides transport pill when transport toggle is off", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state({ transport: "usb" })}
        hud={hud({ transport: false })}
      />,
    );
    expect(html).not.toContain("USB");
  });

  it("hides modifier badges when modifiers toggle is off", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state({ modifiers: MOD_LSFT })}
        hud={hud({ modifiers: false })}
      />,
    );
    expect(html).not.toContain("modifier-badge");
  });
});
