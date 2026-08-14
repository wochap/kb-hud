import { Component, useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

import { parseKeymapSvg, type KeymapGeometry } from "../keymap/parser";
import {
  DEFAULT_HUD_VISIBILITY,
  DEFAULT_OVERLAY_APPEARANCE,
  type HudVisibility,
  type OverlayAppearance,
  type Profile,
} from "../profile";
import {
  DISCONNECTED_STATE,
  TELEMETRY_STATE_EVENT,
  type TelemetryState,
} from "../telemetry";
import { OverlayView } from "./OverlayView";
import { overlayWindowSize } from "./sizing";

/** Surfaces render exceptions on the overlay instead of a blank window. */
class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: string | null }
> {
  state = { error: null as string | null };

  static getDerivedStateFromError(error: unknown) {
    return { error: String(error) };
  }

  render() {
    if (this.state.error) {
      return <div className="overlay-empty">overlay error: {this.state.error}</div>;
    }
    return this.props.children;
  }
}

/**
 * Dev-only fallback keymap so overlay geometry can be verified in the
 * sandbox before a profile has an SVG path configured.
 */
const DEV_FALLBACK_SVG =
  "/home/gean/Sandboxes/sandbox/_keyboards/chocofi-zmk-config/draw/corne.svg";

async function resizeToKeymap(keymap: KeymapGeometry, scale: number) {
  const { width, height } = overlayWindowSize(keymap, scale);
  try {
    await getCurrentWindow().setSize(new LogicalSize(width, height));
  } catch {
    // plain browser dev session without a Tauri window
  }
}

export function OverlayApp() {
  const [keymap, setKeymap] = useState<KeymapGeometry | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reloadCount, setReloadCount] = useState(0);
  const [telemetry, setTelemetry] = useState<TelemetryState>(DISCONNECTED_STATE);
  const [hud, setHud] = useState<HudVisibility>(DEFAULT_HUD_VISIBILITY);
  const [overlayAppearance, setOverlayAppearance] = useState<OverlayAppearance>(
    DEFAULT_OVERLAY_APPEARANCE,
  );

  useEffect(() => {
    const unlisten = listen<TelemetryState>(TELEMETRY_STATE_EVENT, (event) => {
      setTelemetry(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    const unlisten = listen("profile-changed", () =>
      setReloadCount((n) => n + 1),
    );
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      let svgPath = "";
      try {
        const profile = await invoke<Profile>("get_active_profile");
        svgPath = profile.svgPath;
        if (!cancelled) {
          setHud(profile.hud ?? DEFAULT_HUD_VISIBILITY);
          setOverlayAppearance(
            profile.overlayAppearance ?? DEFAULT_OVERLAY_APPEARANCE,
          );
        }
      } catch {
        // backend unavailable (plain vite dev): fall through to dev default
      }
      if (!svgPath && import.meta.env.DEV) svgPath = DEV_FALLBACK_SVG;
      if (!svgPath) {
        setError("no keymap SVG configured — set one in settings");
        return;
      }
      try {
        const svgText = await invoke<string>("read_keymap_svg", { path: svgPath });
        const parsed = parseKeymapSvg(svgText);
        if (cancelled) return;
        setKeymap(parsed);
        setError(null);
        const profile = await invoke<Profile>("get_active_profile").catch(
          () => null,
        );
        await resizeToKeymap(parsed, profile?.scale ?? 1.0);
      } catch (e) {
        if (!cancelled) setError(`keymap load failed: ${e}`);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [reloadCount]);

  return (
    <ErrorBoundary>
      <OverlayView
        keymap={keymap}
        state={telemetry}
        error={error}
        hud={hud}
        overlayAppearance={overlayAppearance}
      />
    </ErrorBoundary>
  );
}
