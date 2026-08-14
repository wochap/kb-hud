import { readFileSync } from "node:fs";
import { join } from "node:path";

import { act } from "react";
import { createRoot } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";

import { parseKeymapSvg } from "../../keymap/parser";
import {
  DEFAULT_HUD_VISIBILITY,
  DEFAULT_OVERLAY_APPEARANCE,
  type OverlayAppearance,
} from "../../profile";
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

function appearance(patch: Partial<OverlayAppearance> = {}): OverlayAppearance {
  return { ...DEFAULT_OVERLAY_APPEARANCE, ...patch };
}

const containers: HTMLDivElement[] = [];

function renderToDom(props: {
  overlayAppearance?: OverlayAppearance;
  st?: TelemetryState;
}): HTMLElement {
  const container = document.createElement("div");
  document.body.appendChild(container);
  containers.push(container);
  const root = createRoot(container);
  act(() => {
    root.render(
      <OverlayView
        keymap={keymap}
        state={props.st ?? state(1)}
        hud={DEFAULT_HUD_VISIBILITY}
        overlayAppearance={props.overlayAppearance}
      />,
    );
  });
  return container;
}

function overlayVar(container: HTMLElement, name: string): string {
  const overlay = container.querySelector(".overlay") as HTMLElement;
  return overlay.style.getPropertyValue(name).trim();
}

afterEach(() => {
  for (const container of containers.splice(0)) {
    container.remove();
  }
});

describe("idle key background visibility", () => {
  it("exposes the configured idle-fill opacity when backgrounds are enabled", () => {
    const container = renderToDom({
      overlayAppearance: appearance({ idleKeyBackgroundOpacity: 0.68 }),
    });
    expect(overlayVar(container, "--idle-fill-opacity")).toBe("0.68");
  });

  it("zeroes the idle-fill opacity when backgrounds are disabled", () => {
    const container = renderToDom({
      overlayAppearance: appearance({
        showIdleKeyBackgrounds: false,
        idleKeyBackgroundOpacity: 0.68,
      }),
    });
    expect(overlayVar(container, "--idle-fill-opacity")).toBe("0");
  });

  it("restores the saved idle opacity when backgrounds are re-enabled", () => {
    const container = renderToDom({
      overlayAppearance: appearance({
        showIdleKeyBackgrounds: true,
        idleKeyBackgroundOpacity: 0.42,
      }),
    });
    expect(overlayVar(container, "--idle-fill-opacity")).toBe("0.42");
  });

  it("keeps idle fill opacity independent of other opacity values", () => {
    const container = renderToDom({
      overlayAppearance: appearance({
        showIdleKeyBackgrounds: false,
        idleKeyBackgroundOpacity: 0.5,
        activeKeyBackgroundOpacity: 0.9,
      }),
    });
    expect(overlayVar(container, "--idle-fill-opacity")).toBe("0");
    expect(overlayVar(container, "--active-fill-opacity")).toBe("0.9");
  });
});

describe("ordinary and resolved modifier feedback are preserved", () => {
  it("still renders pressed-key highlights when idle backgrounds are off", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state(1, [16])}
        overlayAppearance={appearance({ showIdleKeyBackgrounds: false })}
      />,
    );
    // The idle fill is hidden but the highlight/feedback markup remains.
    expect(html).toContain('class="keycap');
    expect(html).not.toContain("--idle-fill-opacity:0.68");
  });

  it("still applies resolved-modifier styling for an active Shift hold", () => {
    const html = renderToString(
      <OverlayView
        keymap={keymap}
        state={state(1, [16], { modifiers: MOD_LSFT })}
        overlayAppearance={appearance({ showIdleKeyBackgrounds: false })}
      />,
    );
    expect(html).toContain("modifier-resolved");
  });

  it("wires the active-key opacity variable for pressed fills", () => {
    const container = renderToDom({
      overlayAppearance: appearance({ activeKeyBackgroundOpacity: 0.75 }),
    });
    expect(overlayVar(container, "--active-fill-opacity")).toBe("0.75");
  });
});

describe("label, border, and pill opacity composition", () => {
  it("wires label and key-border opacity variables", () => {
    const container = renderToDom({
      overlayAppearance: appearance({ labelOpacity: 0.5, keyBorderOpacity: 0.3 }),
    });
    expect(overlayVar(container, "--label-opacity")).toBe("0.5");
    expect(overlayVar(container, "--key-border-opacity")).toBe("0.3");
  });

  it("isolates top-bar pill opacity from the idle-fill opacity", () => {
    const container = renderToDom({
      overlayAppearance: appearance({
        topBarPillBackgroundOpacity: 0.2,
        idleKeyBackgroundOpacity: 0.68,
      }),
    });
    expect(overlayVar(container, "--pill-fill-opacity")).toBe("0.2");
    expect(overlayVar(container, "--idle-fill-opacity")).toBe("0.68");
  });
});

describe("themed label contrast", () => {
  it("renders tap and hold labels that receive the contrasting outline via CSS", () => {
    const html = renderToString(
      <OverlayView keymap={keymap} state={state(1)} />,
    );
    expect(html).toContain('class="keycap-tap"');
    expect(html).toContain('class="keycap-hold"');
  });
});

describe("press-decay multiplies active fill opacity", () => {
  it("applies intensity as inline opacity on the highlight", () => {
    const frames: FrameRequestCallback[] = [];
    const originalRaf = globalThis.requestAnimationFrame;
    const originalCancel = globalThis.cancelAnimationFrame;
    globalThis.requestAnimationFrame = ((cb: FrameRequestCallback) => {
      frames.push(cb);
      return frames.length;
    }) as typeof requestAnimationFrame;
    globalThis.cancelAnimationFrame = (() => {}) as typeof cancelAnimationFrame;

    try {
      const container = document.createElement("div");
      document.body.appendChild(container);
      containers.push(container);
      const root = createRoot(container);
      act(() => {
        root.render(
          <OverlayView
            keymap={keymap}
            state={state(1, [16])}
            overlayAppearance={appearance()}
          />,
        );
      });
      // Flush the scheduled highlight tick while the key is still held.
      act(() => {
        for (const cb of frames.splice(0)) cb(performance.now());
      });
      const highlight = container.querySelector(".keycap-highlight");
      expect(highlight).not.toBeNull();
      const style = highlight!.getAttribute("style") ?? "";
      expect(style).toContain("opacity");
    } finally {
      globalThis.requestAnimationFrame = originalRaf;
      globalThis.cancelAnimationFrame = originalCancel;
    }
  });
});
