import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import type { Appearance } from "../profile";
import { CONFIG_CHANGED_EVENT } from "../events";
import {
  activeThemeId,
  resolveTheme,
  type ResolvedTheme,
  type SystemAppearance,
} from "./index";
import { APP_TOKEN_VARS, OVERLAY_TOKEN_VARS } from "./tokens";

/**
 * Reads the current GTK/system appearance from Tauri. Falls back to dark when
 * the platform does not report a known appearance (or outside a Tauri window).
 */
export async function readSystemAppearance(): Promise<SystemAppearance> {
  try {
    const theme = await getCurrentWindow().theme();
    if (theme === "light") return "light";
    if (theme === "dark") return "dark";
    return "dark";
  } catch {
    return "dark";
  }
}

/**
 * Applies the resolved theme's semantic tokens as CSS custom properties on the
 * document root and tags the root with appearance/theme data attributes so CSS
 * can branch on them.
 */
export function applyThemeToDocument(theme: ResolvedTheme): void {
  const root = document.documentElement;
  for (const [key, varName] of Object.entries(APP_TOKEN_VARS)) {
    root.style.setProperty(varName, theme.tokens.app[key as keyof typeof theme.tokens.app]);
  }
  for (const [key, varName] of Object.entries(OVERLAY_TOKEN_VARS)) {
    root.style.setProperty(
      varName,
      theme.tokens.overlay[key as keyof typeof theme.tokens.overlay],
    );
  }
  root.setAttribute("data-theme", theme.isDark ? "dark" : "light");
  root.setAttribute("data-theme-id", theme.themeId);
}

async function loadAppearance(): Promise<Appearance | null> {
  try {
    return await invoke<Appearance>("get_global_appearance");
  } catch {
    return null;
  }
}

export interface UseThemeResult {
  theme: ResolvedTheme | null;
  systemAppearance: SystemAppearance;
}

/**
 * Shared theming hook. Reads Tauri's current GTK/system appearance, subscribes
 * to live appearance changes and to global configuration changes, resolves the
 * configured palette for the active appearance, and applies the resulting
 * semantic tokens to the document root.
 *
 * Mounted independently by both the settings and overlay roots. The system
 * appearance is authoritative; there is no manual override.
 */
export function useTheme(): UseThemeResult {
  const [appearance, setAppearance] = useState<Appearance | null>(null);
  const [systemAppearance, setSystemAppearance] =
    useState<SystemAppearance>("dark");

  const refresh = useCallback(async () => {
    const [saved, system] = await Promise.all([
      loadAppearance(),
      readSystemAppearance(),
    ]);
    setAppearance(saved);
    setSystemAppearance(system);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    // Live GTK/system appearance changes.
    unlisteners.push(
      listen<{ value: string | null }>("tauri://theme-changed", async () => {
        setSystemAppearance(await readSystemAppearance());
      }),
    );

    // Global palette assignment edits (light/dark theme selections).
    unlisteners.push(
      listen(CONFIG_CHANGED_EVENT, async () => {
        setAppearance(await loadAppearance());
      }),
    );

    return () => {
      for (const pending of unlisteners) {
        pending.then((fn) => fn()).catch(() => {});
      }
    };
  }, []);

  const theme = resolveTheme(appearance, systemAppearance);

  useEffect(() => {
    applyThemeToDocument(theme);
  }, [theme.themeId, systemAppearance]); // eslint-disable-line react-hooks/exhaustive-deps

  return { theme, systemAppearance };
}

/** Convenience accessor for the active theme id without the full token set. */
export function useActiveThemeId(): string {
  const { theme } = useTheme();
  return theme?.themeId ?? activeThemeId(null, "dark");
}
