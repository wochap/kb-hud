import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  DEFAULT_HUD_VISIBILITY,
  DEFAULT_OVERLAY_APPEARANCE,
  type Profile,
} from "../../profile";
import { DeviceSection } from "../SettingsView";

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

describe("DeviceSection auto-discovery copy", () => {
  it("describes auto as compatible telemetry keyboard discovery", () => {
    const html = renderToString(
      <DeviceSection profile={profile()} reload={() => {}} />,
    );
    expect(html).toContain("zmk-key-telemetry");
    expect(html).toContain("explicit MAC");
    expect(html.toLowerCase()).not.toContain("chocochap");
  });

  it("checks the auto option when the profile uses auto", () => {
    const html = renderToString(
      <DeviceSection profile={profile()} reload={() => {}} />,
    );
    expect(html).toContain("checked");
  });

  it("shows the explicit MAC when the profile pins one", () => {
    const html = renderToString(
      <DeviceSection
        profile={profile({ deviceMac: "AA:BB:CC:DD:EE:FF" })}
        reload={() => {}}
      />,
    );
    expect(html).toContain("AA:BB:CC:DD:EE:FF");
  });
});
