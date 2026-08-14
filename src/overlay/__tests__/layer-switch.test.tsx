import { readFileSync } from "node:fs";
import { join } from "node:path";

import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";

import { parseKeymapSvg } from "../../keymap/parser";
import type { TelemetryState } from "../../telemetry";
import { OverlayView } from "../OverlayView";

const keymap = parseKeymapSvg(
  readFileSync(join(__dirname, "../../keymap/__tests__/corne.svg"), "utf-8"),
);

function state(activeLayers: number, pressed: number[] = []): TelemetryState {
  return {
    connection: "connected",
    pressed,
    activeLayers,
    sequence: 1,
    timestampMs: 0,
    gaps: 0,
  };
}

describe("OverlayView layer switching (React update path)", () => {
  it("keeps all keycaps when switching to a held layer and back", () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);

    act(() => {
      root.render(<OverlayView keymap={keymap} state={state(1)} />);
    });
    expect(container.querySelectorAll("rect.keycap").length).toBe(42);

    act(() => {
      root.render(<OverlayView keymap={keymap} state={state(0b1001)} />);
    });
    expect(container.querySelectorAll("rect.keycap").length).toBe(42);
    expect(container.textContent).toContain("nav");

    act(() => {
      root.render(<OverlayView keymap={keymap} state={state(1)} />);
    });
    expect(container.querySelectorAll("rect.keycap").length).toBe(42);
    expect(container.textContent).toContain("colemakdh");

    root.unmount();
    container.remove();
  });
});
