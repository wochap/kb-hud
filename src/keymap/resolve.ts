import type { KeymapGeometry } from "./parser";

/** Indices of all set bits in the layer mask, ascending. */
export function activeLayerIndices(activeLayers: number): number[] {
  const indices: number[] = [];
  for (let i = 0; i < 32; i++) {
    if ((activeLayers >>> i) & 1) indices.push(i);
  }
  return indices;
}

/** Effective layer = highest set bit of the mask; 0 when the mask is empty. */
export function effectiveLayerIndex(activeLayers: number): number {
  const indices = activeLayerIndices(activeLayers);
  return indices.length > 0 ? indices[indices.length - 1] : 0;
}

export interface ResolvedKey {
  tap: string;
  hold: string;
  /** True when no active layer provides a non-transparent label. */
  empty: boolean;
}

/**
 * Resolves the label shown for a position: starts at the effective layer and
 * walks downward through the active-layer stack to the first non-transparent
 * label. Transparent keys with no lower active label render empty.
 */
export function resolveKey(
  keymap: KeymapGeometry,
  position: number,
  activeLayers: number,
): ResolvedKey {
  const indices = activeLayerIndices(activeLayers);
  for (let i = indices.length - 1; i >= 0; i--) {
    const layer = keymap.layers[indices[i]];
    if (!layer) continue;
    const key = layer.keys.get(position);
    if (!key) continue;
    if (!key.transparent) {
      return { tap: key.tap, hold: key.hold, empty: false };
    }
  }
  return { tap: "", hold: "", empty: true };
}
