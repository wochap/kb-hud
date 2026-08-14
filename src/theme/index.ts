import type { Appearance, ThemeId } from "../profile";
import { DEFAULT_APPEARANCE } from "../profile";
import { PALETTES, isDarkFlavor, type CatppuccinPalette } from "./palettes";
import { buildThemeTokens, type ThemeTokens } from "./tokens";

export { PALETTES, isDarkFlavor } from "./palettes";
export type { CatppuccinPalette } from "./palettes";
export { buildThemeTokens, APP_TOKEN_VARS, OVERLAY_TOKEN_VARS } from "./tokens";
export type { AppTokens, OverlayTokens, ThemeTokens } from "./tokens";

export type SystemAppearance = "light" | "dark";

const THEME_IDS: ThemeId[] = ["latte", "frappe", "macchiato", "mocha"];

export const THEME_LABELS: Record<ThemeId, string> = {
  latte: "Latte",
  frappe: "Frappé",
  macchiato: "Macchiato",
  mocha: "Mocha",
};

export function isThemeId(value: string): value is ThemeId {
  return (THEME_IDS as string[]).includes(value);
}

export const DEFAULT_LIGHT_THEME: ThemeId = "latte";
export const DEFAULT_DARK_THEME: ThemeId = "mocha";

/**
 * Resolves the palette for a saved theme id, falling back to the default for
 * the given system appearance when the id is missing or unknown.
 */
export function resolveThemeId(
  themeId: string | undefined | null,
  systemAppearance: SystemAppearance,
): ThemeId {
  if (themeId && isThemeId(themeId)) return themeId;
  return systemAppearance === "light" ? DEFAULT_LIGHT_THEME : DEFAULT_DARK_THEME;
}

/**
 * Picks the active theme id for the current system appearance from the global
 * appearance configuration, applying per-appearance fallbacks.
 */
export function activeThemeId(
  appearance: Appearance | undefined | null,
  systemAppearance: SystemAppearance,
): ThemeId {
  const configured = appearance ?? DEFAULT_APPEARANCE;
  const saved =
    systemAppearance === "light" ? configured.lightTheme : configured.darkTheme;
  return resolveThemeId(saved, systemAppearance);
}

export interface ResolvedTheme {
  themeId: ThemeId;
  palette: CatppuccinPalette;
  tokens: ThemeTokens;
  isDark: boolean;
}

/**
 * Resolves the full theme (palette + semantic tokens) for a system appearance
 * and global appearance configuration.
 */
export function resolveTheme(
  appearance: Appearance | undefined | null,
  systemAppearance: SystemAppearance,
): ResolvedTheme {
  const themeId = activeThemeId(appearance, systemAppearance);
  const palette = PALETTES[themeId];
  const isDark = isDarkFlavor(themeId);
  return {
    themeId,
    palette,
    tokens: buildThemeTokens(palette, isDark),
    isDark,
  };
}
