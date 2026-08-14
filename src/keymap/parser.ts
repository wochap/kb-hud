/**
 * Keymap-drawer SVG parser.
 *
 * Extracts geometry and labels from the layered SVG format: per-layer groups
 * `<g class="layer-<name>">` containing per-position groups
 * `<g class="key keypos-N">` with a translate/rotate transform, a centered
 * `<rect class="key">`, and tap/hold `<text>` labels.
 */

export interface Matrix {
  a: number;
  b: number;
  c: number;
  d: number;
  e: number;
  f: number;
}

export const IDENTITY: Matrix = { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };

export function multiply(outer: Matrix, inner: Matrix): Matrix {
  return {
    a: outer.a * inner.a + outer.c * inner.b,
    b: outer.b * inner.a + outer.d * inner.b,
    c: outer.a * inner.c + outer.c * inner.d,
    d: outer.b * inner.c + outer.d * inner.d,
    e: outer.a * inner.e + outer.c * inner.f + outer.e,
    f: outer.b * inner.e + outer.d * inner.f + outer.f,
  };
}

export function apply(m: Matrix, x: number, y: number): { x: number; y: number } {
  return { x: m.a * x + m.c * y + m.e, y: m.b * x + m.d * y + m.f };
}

function translate(x: number, y: number): Matrix {
  return { a: 1, b: 0, c: 0, d: 1, e: x, f: y };
}

function rotate(deg: number): Matrix {
  const rad = (deg * Math.PI) / 180;
  const cos = Math.cos(rad);
  const sin = Math.sin(rad);
  return { a: cos, b: sin, c: -sin, d: cos, e: 0, f: 0 };
}

/** Parses `translate(x, y)` / `translate(x, y) rotate(deg)` transform lists. */
export function parseTransform(value: string): Matrix {
  let result = IDENTITY;
  const fn = /\s*(translate|rotate|scale|matrix)\s*\(([^)]*)\)/g;
  let match: RegExpExecArray | null;
  let matched = false;
  while ((match = fn.exec(value)) !== null) {
    matched = true;
    const args = match[2]
      .split(/[\s,]+/)
      .filter((s) => s.length > 0)
      .map(Number);
    if (args.some((n) => Number.isNaN(n))) {
      throw new KeymapParseError(`invalid transform arguments in "${value}"`);
    }
    switch (match[1]) {
      case "translate":
        result = multiply(result, translate(args[0] ?? 0, args[1] ?? 0));
        break;
      case "rotate":
        result = multiply(result, rotate(args[0] ?? 0));
        break;
      case "scale":
        result = multiply(result, {
          a: args[0] ?? 1,
          b: 0,
          c: 0,
          d: args[1] ?? args[0] ?? 1,
          e: 0,
          f: 0,
        });
        break;
      default:
        throw new KeymapParseError(`unsupported transform function "${match[1]}"`);
    }
  }
  if (!matched && value.trim().length > 0) {
    throw new KeymapParseError(`unrecognized transform "${value}"`);
  }
  return result;
}

export class KeymapParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "KeymapParseError";
  }
}

export interface KeyGeometry {
  position: number;
  /** Absolute transform placing the key center (translate + rotation). */
  transform: Matrix;
  rotation: number;
  center: { x: number; y: number };
  rect: { x: number; y: number; width: number; height: number; rx: number };
  tap: string;
  hold: string;
  transparent: boolean;
}

export interface LayerGeometry {
  name: string;
  index: number;
  keys: Map<number, KeyGeometry>;
}

export interface KeymapGeometry {
  layers: LayerGeometry[];
  /** Sorted position set, identical across all layers. */
  positions: number[];
  /** Native bounds of one layer's keys in SVG units. */
  bounds: { minX: number; minY: number; width: number; height: number };
}

const LAYER_CLASS = /^layer-(.+)$/;
const KEYPOS_CLASS = /keypos-(\d+)/;

function classList(element: Element): string[] {
  return (element.getAttribute("class") ?? "").split(/\s+/).filter((c) => c.length > 0);
}

function keyCornerPoints(key: KeyGeometry): { x: number; y: number }[] {
  const { x, y, width, height } = key.rect;
  return [
    { x, y },
    { x: x + width, y },
    { x, y: y + height },
    { x: x + width, y: y + height },
  ].map((p) => apply(key.transform, p.x, p.y));
}

/**
 * keymap-drawer stacks all layers vertically in one SVG; each layer group
 * carries its own stacking offset. Shift every layer so its bounding box
 * starts at the origin — layers are interchangeable renders of the same
 * board, and the overlay displays one layer at a time.
 */
function normalizeLayerOrigin(keys: Map<number, KeyGeometry>) {
  let minX = Infinity;
  let minY = Infinity;
  for (const key of keys.values()) {
    for (const corner of keyCornerPoints(key)) {
      minX = Math.min(minX, corner.x);
      minY = Math.min(minY, corner.y);
    }
  }
  for (const key of keys.values()) {
    key.transform = {
      ...key.transform,
      e: key.transform.e - minX,
      f: key.transform.f - minY,
    };
    key.center = apply(key.transform, 0, 0);
  }
}

function textOf(element: Element | null): string {
  return element?.textContent?.trim() ?? "";
}

function parseKey(
  group: Element,
  position: number,
  ancestorTransform: Matrix,
): KeyGeometry {
  const ownTransform = parseTransform(group.getAttribute("transform") ?? "");
  const transform = multiply(ancestorTransform, ownTransform);

  const rect = group.querySelector(":scope > rect.key");
  if (!rect) {
    throw new KeymapParseError(`keypos-${position}: missing <rect class="key">`);
  }
  const rectModel = {
    x: Number(rect.getAttribute("x") ?? 0),
    y: Number(rect.getAttribute("y") ?? 0),
    width: Number(rect.getAttribute("width")),
    height: Number(rect.getAttribute("height")),
    rx: Number(rect.getAttribute("rx") ?? 0),
  };
  if (Number.isNaN(rectModel.width) || Number.isNaN(rectModel.height)) {
    throw new KeymapParseError(`keypos-${position}: rect has no width/height`);
  }

  const tap = textOf(group.querySelector(":scope text.tap, :scope > a text.tap"));
  const hold = textOf(group.querySelector(":scope text.hold"));
  const classes = classList(group);
  const transparent = classes.includes("trans") || tap === "▽";

  // Rotation angle is the transform's rotation; keymap-drawer applies
  // rotate() only on the key group itself, around the key center.
  let rotation = 0;
  const rotateMatch = /rotate\((-?[\d.]+)\)/.exec(
    group.getAttribute("transform") ?? "",
  );
  if (rotateMatch) rotation = Number(rotateMatch[1]);

  return {
    position,
    transform,
    rotation,
    center: apply(transform, 0, 0),
    rect: rectModel,
    tap,
    hold,
    transparent,
  };
}

export function parseKeymapSvg(svgText: string): KeymapGeometry {
  const doc = new DOMParser().parseFromString(svgText, "image/svg+xml");
  if (doc.querySelector("parsererror")) {
    throw new KeymapParseError("file is not valid SVG/XML");
  }

  const layerGroups = [...doc.querySelectorAll("g")].filter((g) =>
    classList(g).some((c) => LAYER_CLASS.test(c)),
  );
  if (layerGroups.length === 0) {
    throw new KeymapParseError(
      'no layer groups found (expected <g class="layer-*"> elements)',
    );
  }

  const layers: LayerGeometry[] = [];
  for (const [index, group] of layerGroups.entries()) {
    const layerClass = classList(group).find((c) => LAYER_CLASS.test(c))!;
    const name = LAYER_CLASS.exec(layerClass)![1];

    // Compose transforms from the layer group down to each key group.
    const layerTransform = parseTransform(group.getAttribute("transform") ?? "");
    const keys = new Map<number, KeyGeometry>();
    for (const keyGroup of group.querySelectorAll("g")) {
      const classes = classList(keyGroup);
      const keyposClass = classes.find((c) => KEYPOS_CLASS.test(c));
      if (!keyposClass || !classes.includes("key")) continue;
      const position = Number(KEYPOS_CLASS.exec(keyposClass)![1]);

      let ancestorTransform = layerTransform;
      for (
        let parent: Element | null = keyGroup.parentElement as Element | null;
        parent && parent !== group;
        parent = parent.parentElement as Element | null
      ) {
        if (parent.tagName.toLowerCase() !== "g") continue;
        const parentTransform = parseTransform(parent.getAttribute("transform") ?? "");
        ancestorTransform = multiply(ancestorTransform, parentTransform);
      }

      if (keys.has(position)) {
        throw new KeymapParseError(
          `layer '${name}': duplicate keypos-${position}`,
        );
      }
      keys.set(position, parseKey(keyGroup, position, ancestorTransform));
    }

    if (keys.size === 0) {
      throw new KeymapParseError(`layer '${name}': contains no keypos-* keys`);
    }
    normalizeLayerOrigin(keys);
    layers.push({ name, index, keys });
  }

  const referencePositions = [...layers[0].keys.keys()].sort((a, b) => a - b);
  for (const layer of layers.slice(1)) {
    const positions = new Set(layer.keys.keys());
    const missing = referencePositions.filter((p) => !positions.has(p));
    const extra = [...positions].filter((p) => !referencePositions.includes(p));
    if (missing.length > 0 || extra.length > 0) {
      const parts: string[] = [];
      if (missing.length > 0) {
        parts.push(`missing ${missing.map((p) => `keypos-${p}`).join(", ")}`);
      }
      if (extra.length > 0) {
        parts.push(
          `unexpected ${extra.sort((a, b) => a - b).map((p) => `keypos-${p}`).join(", ")}`,
        );
      }
      throw new KeymapParseError(
        `layer '${layer.name}' does not match '${layers[0].name}': ${parts.join("; ")}`,
      );
    }
  }

  // Native bounds from the first layer (all layers share the position set).
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const key of layers[0].keys.values()) {
    for (const corner of keyCornerPoints(key)) {
      minX = Math.min(minX, corner.x);
      minY = Math.min(minY, corner.y);
      maxX = Math.max(maxX, corner.x);
      maxY = Math.max(maxY, corner.y);
    }
  }

  return {
    layers,
    positions: referencePositions,
    bounds: {
      minX,
      minY,
      width: maxX - minX,
      height: maxY - minY,
    },
  };
}
