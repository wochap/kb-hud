import { describe, expect, it } from "vitest";

import { MOD_LSFT, MOD_RSFT } from "../../telemetry";
import { shiftedTapLabel } from "../shiftLabels";

describe("US Shift label preview", () => {
  it("maps letters, digits, and punctuation for either Shift", () => {
    expect(shiftedTapLabel("a", MOD_LSFT)).toBe("A");
    expect(shiftedTapLabel("1", MOD_RSFT)).toBe("!");
    expect(shiftedTapLabel("/", MOD_LSFT)).toBe("?");
    expect(shiftedTapLabel(";", MOD_RSFT)).toBe(":");
  });

  it("does not transform without Shift", () => {
    expect(shiftedTapLabel("/", 0)).toBe("/");
  });

  it("leaves non-printable and already-shifted labels alone", () => {
    expect(shiftedTapLabel("ENTER", MOD_LSFT)).toBe("ENTER");
    expect(shiftedTapLabel("F1", MOD_LSFT)).toBe("F1");
    expect(shiftedTapLabel("VOL UP", MOD_LSFT)).toBe("VOL UP");
    expect(shiftedTapLabel("?", MOD_LSFT)).toBe("?");
  });
});
