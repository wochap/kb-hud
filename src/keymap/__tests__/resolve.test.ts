import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { parseKeymapSvg } from "../parser";
import { activeLayerIndices, effectiveLayerIndex, resolveKey } from "../resolve";

const keymap = parseKeymapSvg(readFileSync(join(__dirname, "corne.svg"), "utf-8"));

describe("effectiveLayerIndex", () => {
  it("picks the highest set bit", () => {
    expect(effectiveLayerIndex(0b1001)).toBe(3);
  });

  it("returns 0 for the default layer only", () => {
    expect(effectiveLayerIndex(0b0001)).toBe(0);
  });

  it("returns 0 for an empty mask", () => {
    expect(effectiveLayerIndex(0)).toBe(0);
  });
});

describe("activeLayerIndices", () => {
  it("lists set bits ascending", () => {
    expect(activeLayerIndices(0b1011)).toEqual([0, 1, 3]);
  });
});

describe("resolveKey", () => {
  it("renders the effective layer label directly when opaque", () => {
    // layer colemakdh (0) position 1 is Q
    expect(resolveKey(keymap, 1, 0b000001)).toEqual({
      tap: "Q",
      hold: "",
      empty: false,
    });
  });

  it("resolves trans keys down to a lower active layer", () => {
    // position 36 is trans on num (2); colemakdh (0) has LSHIFT there
    const base = keymap.layers[0].keys.get(36)?.tap;
    expect(base).toBe("LSHIFT");
    const resolved = resolveKey(keymap, 36, 0b000101);
    expect(resolved.tap).toBe("LSHIFT");
    expect(resolved.empty).toBe(false);
  });

  it("skips inactive layers between effective and base", () => {
    // only num (2) active without base: trans has no lower active label
    const resolved = resolveKey(keymap, 36, 0b000100);
    expect(resolved.empty).toBe(true);
    expect(resolved.tap).toBe("");
  });

  it("keeps hold labels from the resolved layer", () => {
    // keypos-37 on colemakdh holds "num"
    const resolved = resolveKey(keymap, 37, 0b000001);
    expect(resolved.hold).toBe("num");
  });
});
