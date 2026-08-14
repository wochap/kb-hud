import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { KeymapParseError, parseKeymapSvg, parseTransform } from "../parser";

const corneSvg = readFileSync(join(__dirname, "corne.svg"), "utf-8");

describe("parseKeymapSvg with the real corne.svg", () => {
  const keymap = parseKeymapSvg(corneSvg);

  it("finds all 6 layers in document order", () => {
    expect(keymap.layers.map((l) => l.name)).toEqual([
      "colemakdh",
      "querty",
      "num",
      "nav",
      "fn",
      "adjust",
    ]);
    expect(keymap.layers.map((l) => l.index)).toEqual([0, 1, 2, 3, 4, 5]);
  });

  it("finds 42 identical positions in every layer", () => {
    expect(keymap.positions).toHaveLength(42);
    expect(keymap.positions[0]).toBe(0);
    expect(keymap.positions[41]).toBe(41);
    for (const layer of keymap.layers) {
      expect(layer.keys.size).toBe(42);
    }
  });

  it("extracts tap labels", () => {
    const colemakdh = keymap.layers[0];
    expect(colemakdh.keys.get(1)?.tap).toBe("Q");
    expect(colemakdh.keys.get(2)?.tap).toBe("W");
    expect(colemakdh.keys.get(6)?.tap).toBe("J");
  });

  it("extracts hold labels", () => {
    const colemakdh = keymap.layers[0];
    expect(colemakdh.keys.get(37)?.hold).toBe("num");
  });

  it("extracts key dimensions", () => {
    const key = keymap.layers[0].keys.get(0)!;
    expect(key.rect.width).toBe(52);
    expect(key.rect.height).toBe(52);
    expect(key.rect.x).toBe(-26);
    expect(key.rect.y).toBe(-26);
    expect(key.rect.rx).toBe(6);
  });

  it("extracts thumb-cluster rotations", () => {
    const colemakdh = keymap.layers[0];
    expect(colemakdh.keys.get(37)?.rotation).toBe(15);
    expect(colemakdh.keys.get(38)?.rotation).toBe(30);
    expect(colemakdh.keys.get(39)?.rotation).toBe(-30);
    expect(colemakdh.keys.get(40)?.rotation).toBe(-15);
    expect(colemakdh.keys.get(0)?.rotation).toBe(0);
  });

  it("normalizes every layer to the origin", () => {
    // keypos-0 absolute: layer translate(30,0) + wrapper translate(0,56) +
    // translate(28,49) = (58,105); layer bbox min corner is (32,58)
    const key = keymap.layers[0].keys.get(0)!;
    expect(key.center.x).toBeCloseTo(58 - 32, 5);
    expect(key.center.y).toBeCloseTo(105 - 58, 5);
  });

  it("aligns all layers on the same geometry", () => {
    const base = keymap.layers[0];
    for (const layer of keymap.layers.slice(1)) {
      for (const position of keymap.positions) {
        const a = base.keys.get(position)!.center;
        const b = layer.keys.get(position)!.center;
        expect(b.x).toBeCloseTo(a.x, 5);
        expect(b.y).toBeCloseTo(a.y, 5);
      }
    }
  });

  it("detects transparent keys", () => {
    const num = keymap.layers[2];
    expect(num.keys.get(36)?.transparent).toBe(true);
    expect(num.keys.get(36)?.tap).toBe("▽");
    expect(keymap.layers[0].keys.get(0)?.transparent).toBe(false);
  });

  it("computes native bounds covering the board from the origin", () => {
    const { minX, minY, width, height } = keymap.bounds;
    expect(minX).toBe(0);
    expect(minY).toBe(0);
    expect(width).toBeGreaterThan(700);
    expect(width).toBeLessThan(900);
    expect(height).toBeGreaterThan(200);
    expect(height).toBeLessThan(400);
  });
});

describe("parseKeymapSvg error reporting", () => {
  const key = (pos: number) => `
    <g transform="translate(${pos * 56}, 0)" class="key keypos-${pos}">
      <rect rx="6" ry="6" x="-26" y="-26" width="52" height="52" class="key"/>
      <text x="0" y="0" class="key tap">K</text>
    </g>`;
  const layer = (name: string, keys: string) =>
    `<g transform="translate(0, 0)" class="layer-${name}">${keys}</g>`;
  const svg = (layers: string) =>
    `<svg xmlns="http://www.w3.org/2000/svg">${layers}</svg>`;

  it("rejects SVG without layer groups", () => {
    expect(() => parseKeymapSvg(svg("<g>nothing</g>"))).toThrow(KeymapParseError);
    expect(() => parseKeymapSvg(svg("<g>nothing</g>"))).toThrow(/no layer groups/);
  });

  it("rejects a layer missing positions present elsewhere", () => {
    const doc = svg(
      layer("base", key(0) + key(1)) + layer("other", key(0)),
    );
    expect(() => parseKeymapSvg(doc)).toThrow(
      /layer 'other' does not match 'base'.*keypos-1/,
    );
  });

  it("rejects a layer with unexpected extra positions", () => {
    const doc = svg(
      layer("base", key(0)) + layer("other", key(0) + key(9)),
    );
    expect(() => parseKeymapSvg(doc)).toThrow(/unexpected keypos-9/);
  });

  it("rejects a key group without a rect", () => {
    const doc = svg(
      `<g class="layer-base">
        <g transform="translate(0, 0)" class="key keypos-0">
          <text x="0" y="0" class="key tap">K</text>
        </g>
      </g>`,
    );
    expect(() => parseKeymapSvg(doc)).toThrow(/keypos-0.*rect/);
  });

  it("rejects non-XML input", () => {
    expect(() => parseKeymapSvg("this is not svg")).toThrow(KeymapParseError);
  });
});

describe("parseTransform", () => {
  it("composes translate and rotate", () => {
    const m = parseTransform("translate(10, 20) rotate(90)");
    expect(m.e).toBeCloseTo(10);
    expect(m.f).toBeCloseTo(20);
    expect(m.b).toBeCloseTo(1);
    expect(m.c).toBeCloseTo(-1);
  });

  it("handles empty transform as identity", () => {
    const m = parseTransform("");
    expect(m).toEqual({ a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 });
  });

  it("rejects unsupported functions", () => {
    expect(() => parseTransform("skewX(5)")).toThrow(/unsupported|unrecognized/);
  });
});
