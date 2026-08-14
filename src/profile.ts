export interface HudVisibility {
  layer: boolean;
  connection: boolean;
  gaps: boolean;
  firmwareDrops: boolean;
  battery: boolean;
  transport: boolean;
  modifiers: boolean;
}

export const DEFAULT_HUD_VISIBILITY: HudVisibility = {
  layer: true,
  connection: true,
  gaps: true,
  firmwareDrops: true,
  battery: true,
  transport: true,
  modifiers: true,
};

export type ThemeId = "latte" | "frappe" | "macchiato" | "mocha";

export interface Appearance {
  lightTheme: ThemeId;
  darkTheme: ThemeId;
}

export const DEFAULT_APPEARANCE: Appearance = {
  lightTheme: "latte",
  darkTheme: "mocha",
};

export interface OverlayAppearance {
  showIdleKeyBackgrounds: boolean;
  labelOpacity: number;
  idleKeyBackgroundOpacity: number;
  keyBorderOpacity: number;
  activeKeyBackgroundOpacity: number;
  topBarPillBackgroundOpacity: number;
}

export const DEFAULT_OVERLAY_APPEARANCE: OverlayAppearance = {
  showIdleKeyBackgrounds: true,
  labelOpacity: 0.92,
  idleKeyBackgroundOpacity: 0.68,
  keyBorderOpacity: 0.16,
  activeKeyBackgroundOpacity: 0.75,
  topBarPillBackgroundOpacity: 0.68,
};

export interface Profile {
  name: string;
  svgPath: string;
  deviceMac: string;
  scale: number;
  hud: HudVisibility;
  overlayAppearance: OverlayAppearance;
}
