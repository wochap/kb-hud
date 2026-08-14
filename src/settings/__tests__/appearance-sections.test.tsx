import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  DEFAULT_APPEARANCE,
  DEFAULT_HUD_VISIBILITY,
  DEFAULT_OVERLAY_APPEARANCE,
  type Appearance,
  type Profile,
} from "../../profile";
import { AppearanceSection, OverlayAppearanceSection } from "../SettingsView";

function profile(patch: Partial<Profile> = {}): Profile {
  return {
    name: "default",
    svgPath: "/tmp/corne.svg",
    deviceMac: "auto",
    scale: 1,
    hud: DEFAULT_HUD_VISIBILITY,
    overlayAppearance: DEFAULT_OVERLAY_APPEARANCE,
    ...patch,
  };
}

describe("legacy overlay appearance defaults", () => {
  it("preserves the pre-change overlay look", () => {
    expect(DEFAULT_OVERLAY_APPEARANCE.showIdleKeyBackgrounds).toBe(true);
    expect(DEFAULT_OVERLAY_APPEARANCE.labelOpacity).toBeCloseTo(0.92);
    expect(DEFAULT_OVERLAY_APPEARANCE.idleKeyBackgroundOpacity).toBeCloseTo(0.68);
    expect(DEFAULT_OVERLAY_APPEARANCE.keyBorderOpacity).toBeCloseTo(0.16);
    expect(DEFAULT_OVERLAY_APPEARANCE.activeKeyBackgroundOpacity).toBeCloseTo(0.75);
    expect(
      DEFAULT_OVERLAY_APPEARANCE.topBarPillBackgroundOpacity,
    ).toBeCloseTo(0.68);
  });

  it("defaults global appearance to Latte light and Mocha dark", () => {
    expect(DEFAULT_APPEARANCE.lightTheme).toBe("latte");
    expect(DEFAULT_APPEARANCE.darkTheme).toBe("mocha");
  });
});

describe("AppearanceSection (global scope)", () => {
  it("renders separate light and dark palette selectors", () => {
    const html = renderToString(
      <AppearanceSection appearance={DEFAULT_APPEARANCE} update={() => {}} />,
    );
    expect(html).toContain("Light mode palette");
    expect(html).toContain("Dark mode palette");
    expect(html).toContain("Appearance");
  });

  it("falls back gracefully when appearance has not loaded yet", () => {
    const html = renderToString(
      <AppearanceSection appearance={null} update={() => {}} />,
    );
    expect(html).toContain("Light mode palette");
    expect(html).toContain("Dark mode palette");
  });
});

describe("OverlayAppearanceSection (profile scope)", () => {
  it("renders the idle-background switch and five opacity sliders", () => {
    const html = renderToString(
      <OverlayAppearanceSection profile={profile()} reload={() => {}} />,
    );
    expect(html).toContain("Show idle key backgrounds");
    expect(html).toContain("Label opacity");
    expect(html).toContain("Idle key background opacity");
    expect(html).toContain("Key border opacity");
    expect(html).toContain("Active key background opacity");
    expect(html).toContain("Top-bar pill background opacity");
    const sliderCount = (html.match(/role="slider"/g) ?? []).length;
    expect(sliderCount).toBe(5);
  });

  it("renders the idle-background switch as checked by default", () => {
    const html = renderToString(
      <OverlayAppearanceSection profile={profile()} reload={() => {}} />,
    );
    expect(html).toContain('aria-checked="true"');
  });

  it("reflects a profile's own overlay appearance values", () => {
    const custom = profile({
      overlayAppearance: {
        ...DEFAULT_OVERLAY_APPEARANCE,
        showIdleKeyBackgrounds: false,
        labelOpacity: 0.25,
      },
    });
    const html = renderToString(
      <OverlayAppearanceSection profile={custom} reload={() => {}} />,
    );
    expect(html).toContain('aria-checked="false"');
    expect(html).toContain("25%");
  });

  it("applies defaults when a legacy profile lacks overlay appearance", () => {
    const legacy = profile();
    // Simulate a legacy profile object missing the field entirely.
    delete (legacy as Partial<Profile>).overlayAppearance;
    const html = renderToString(
      <OverlayAppearanceSection profile={legacy} reload={() => {}} />,
    );
    expect(html).toContain('aria-checked="true"');
    expect(html).toContain("92%");
  });
});

describe("global vs profile scope separation", () => {
  it("appearance is independent of any profile fields", () => {
    const appearance: Appearance = { lightTheme: "frappe", darkTheme: "macchiato" };
    const html = renderToString(
      <AppearanceSection appearance={appearance} update={() => {}} />,
    );
    // Appearance section never references profile-specific controls.
    expect(html).not.toContain("Overlay appearance");
    expect(html).not.toContain("Keymap");
  });
});
