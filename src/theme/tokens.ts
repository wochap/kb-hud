import type { CatppuccinPalette } from "./palettes";

/**
 * shadcn/ui-compatible application tokens. These map onto the CSS custom
 * properties consumed by `src/index.css` (`--background`, `--foreground`, …).
 */
export interface AppTokens {
  background: string;
  foreground: string;
  card: string;
  cardForeground: string;
  popover: string;
  popoverForeground: string;
  primary: string;
  primaryForeground: string;
  secondary: string;
  secondaryForeground: string;
  muted: string;
  mutedForeground: string;
  accent: string;
  accentForeground: string;
  destructive: string;
  destructiveForeground: string;
  border: string;
  input: string;
  ring: string;
}

/**
 * Overlay-specific semantic tokens used by the keyboard HUD. These are applied
 * as CSS custom properties on the overlay root and referenced from the overlay
 * stylesheet instead of hard-coded colors.
 */
export interface OverlayTokens {
  idleFill: string;
  idleBorder: string;
  tapLabel: string;
  holdLabel: string;
  /** Contrasting shadow behind tap labels (dark label → light shadow, etc.). */
  tapLabelShadow: string;
  /** Contrasting shadow behind hold labels. */
  holdLabelShadow: string;
  activeFill: string;
  activeBorder: string;
  resolvedModifierFill: string;
  resolvedModifierBorder: string;
  pillFill: string;
  pillBorder: string;
  pillText: string;
  statusConnected: string;
  statusConnecting: string;
  statusDisconnected: string;
  error: string;
}

export interface ThemeTokens {
  app: AppTokens;
  overlay: OverlayTokens;
}

/**
 * Maps a canonical palette into the shared semantic token sets. `isDark`
 * distinguishes elevation for popover surfaces: dark flavors lift popovers to
 * a lighter surface, while the light flavor keeps them on the base surface.
 *
 * Blue is the primary / ordinary pressed-key accent in every flavor; Peach is
 * the distinct resolved-modifier accent. Label shadows deliberately contrast
 * with label colors (crust is dark in dark flavors and light in Latte).
 */
export function buildThemeTokens(
  palette: CatppuccinPalette,
  isDark: boolean,
): ThemeTokens {
  const app: AppTokens = {
    background: palette.base,
    foreground: palette.text,
    card: palette.base,
    cardForeground: palette.text,
    popover: isDark ? palette.surface0 : palette.mantle,
    popoverForeground: palette.text,
    primary: palette.blue,
    primaryForeground: palette.base,
    secondary: palette.surface0,
    secondaryForeground: palette.text,
    muted: palette.surface0,
    mutedForeground: palette.subtext0,
    accent: palette.surface1,
    accentForeground: palette.text,
    destructive: palette.red,
    destructiveForeground: palette.base,
    border: palette.surface0,
    input: palette.surface1,
    ring: palette.blue,
  };

  const overlay: OverlayTokens = {
    idleFill: palette.base,
    idleBorder: palette.overlay0,
    tapLabel: palette.text,
    holdLabel: palette.subtext0,
    tapLabelShadow: palette.crust,
    holdLabelShadow: palette.crust,
    activeFill: palette.blue,
    activeBorder: palette.blue,
    resolvedModifierFill: palette.peach,
    resolvedModifierBorder: palette.peach,
    pillFill: palette.base,
    pillBorder: palette.overlay0,
    pillText: palette.text,
    statusConnected: palette.green,
    statusConnecting: palette.yellow,
    statusDisconnected: palette.red,
    error: palette.red,
  };

  return { app, overlay };
}

/** CSS custom property names for the shadcn application tokens. */
export const APP_TOKEN_VARS: Record<keyof AppTokens, string> = {
  background: "--background",
  foreground: "--foreground",
  card: "--card",
  cardForeground: "--card-foreground",
  popover: "--popover",
  popoverForeground: "--popover-foreground",
  primary: "--primary",
  primaryForeground: "--primary-foreground",
  secondary: "--secondary",
  secondaryForeground: "--secondary-foreground",
  muted: "--muted",
  mutedForeground: "--muted-foreground",
  accent: "--accent",
  accentForeground: "--accent-foreground",
  destructive: "--destructive",
  destructiveForeground: "--destructive-foreground",
  border: "--border",
  input: "--input",
  ring: "--ring",
};

/** CSS custom property names for the overlay semantic tokens. */
export const OVERLAY_TOKEN_VARS: Record<keyof OverlayTokens, string> = {
  idleFill: "--overlay-idle-fill",
  idleBorder: "--overlay-idle-border",
  tapLabel: "--overlay-tap-label",
  holdLabel: "--overlay-hold-label",
  tapLabelShadow: "--overlay-tap-label-shadow",
  holdLabelShadow: "--overlay-hold-label-shadow",
  activeFill: "--overlay-active-fill",
  activeBorder: "--overlay-active-border",
  resolvedModifierFill: "--overlay-resolved-modifier-fill",
  resolvedModifierBorder: "--overlay-resolved-modifier-border",
  pillFill: "--overlay-pill-fill",
  pillBorder: "--overlay-pill-border",
  pillText: "--overlay-pill-text",
  statusConnected: "--overlay-status-connected",
  statusConnecting: "--overlay-status-connecting",
  statusDisconnected: "--overlay-status-disconnected",
  error: "--overlay-error",
};
