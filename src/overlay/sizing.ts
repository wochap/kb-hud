import type { KeymapGeometry } from "../keymap/parser";

export const OVERLAY_PADDING = 14;

/** Logical window size for a keymap at the given scale (D6/D7: native bounds × scale). */
export function overlayWindowSize(
  keymap: KeymapGeometry,
  scale: number,
): { width: number; height: number } {
  return {
    width: Math.ceil((keymap.bounds.width + OVERLAY_PADDING * 2) * scale),
    height: Math.ceil((keymap.bounds.height + OVERLAY_PADDING * 2) * scale),
  };
}
