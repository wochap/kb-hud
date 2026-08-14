import { describe, expect, it } from "vitest";

import { DEFAULT_APPEARANCE, type Appearance, type ThemeId } from "../../profile";
import {
  activeThemeId,
  DEFAULT_DARK_THEME,
  DEFAULT_LIGHT_THEME,
  PALETTES,
  resolveTheme,
  resolveThemeId,
} from "../index";
import { buildThemeTokens } from "../tokens";
import { isDarkFlavor } from "../palettes";

const ALL_FLAVORS: ThemeId[] = ["latte", "frappe", "macchiato", "mocha"];

describe("default mappings", () => {
  it("defaults light mode to Latte and dark mode to Mocha", () => {
    expect(DEFAULT_LIGHT_THEME).toBe("latte");
    expect(DEFAULT_DARK_THEME).toBe("mocha");
    expect(DEFAULT_APPEARANCE.lightTheme).toBe("latte");
    expect(DEFAULT_APPEARANCE.darkTheme).toBe("mocha");
  });

  it("resolves Latte for light appearance with default config", () => {
    expect(activeThemeId(DEFAULT_APPEARANCE, "light")).toBe("latte");
  });

  it("resolves Mocha for dark appearance with default config", () => {
    expect(activeThemeId(DEFAULT_APPEARANCE, "dark")).toBe("mocha");
  });

  it("resolves Mocha when appearance config is missing entirely", () => {
    expect(activeThemeId(null, "dark")).toBe("mocha");
    expect(activeThemeId(undefined, "light")).toBe("latte");
  });
});

describe("all four palette choices", () => {
  it("can resolve every bundled flavor for either appearance", () => {
    const lightAll: Appearance = { lightTheme: "frappe", darkTheme: "frappe" };
    expect(activeThemeId(lightAll, "light")).toBe("frappe");

    for (const flavor of ALL_FLAVORS) {
      const appearance: Appearance = { lightTheme: flavor, darkTheme: flavor };
      expect(activeThemeId(appearance, "light")).toBe(flavor);
      expect(activeThemeId(appearance, "dark")).toBe(flavor);
    }
  });

  it("has canonical palette data for every flavor", () => {
    for (const flavor of ALL_FLAVORS) {
      expect(PALETTES[flavor]).toBeDefined();
      expect(PALETTES[flavor].blue).toMatch(/^#[0-9a-f]{6}$/i);
    }
  });
});

describe("Blue primary accents", () => {
  it("uses Blue as the primary app accent in every flavor", () => {
    for (const flavor of ALL_FLAVORS) {
      const tokens = buildThemeTokens(PALETTES[flavor], isDarkFlavor(flavor));
      expect(tokens.app.primary).toBe(PALETTES[flavor].blue);
      expect(tokens.app.ring).toBe(PALETTES[flavor].blue);
    }
  });

  it("uses Blue for ordinary pressed-key fills and a distinct color for resolved modifiers", () => {
    for (const flavor of ALL_FLAVORS) {
      const tokens = buildThemeTokens(PALETTES[flavor], isDarkFlavor(flavor));
      expect(tokens.overlay.activeFill).toBe(PALETTES[flavor].blue);
      expect(tokens.overlay.resolvedModifierFill).toBe(PALETTES[flavor].peach);
      expect(tokens.overlay.resolvedModifierFill).not.toBe(
        tokens.overlay.activeFill,
      );
    }
  });
});

describe("fallbacks", () => {
  it("falls back to the per-appearance default for an unknown saved theme id", () => {
    expect(resolveThemeId("solarized", "light")).toBe("latte");
    expect(resolveThemeId("solarized", "dark")).toBe("mocha");
    expect(resolveThemeId("", "dark")).toBe("mocha");
    expect(resolveThemeId(null, "light")).toBe("latte");
    expect(resolveThemeId(undefined, "dark")).toBe("mocha");
  });

  it("applies fallback theme through resolveTheme for unknown ids", () => {
    const appearance: Appearance = {
      lightTheme: "dracula" as ThemeId,
      darkTheme: "nord" as ThemeId,
    };
    expect(resolveTheme(appearance, "light").themeId).toBe("latte");
    expect(resolveTheme(appearance, "dark").themeId).toBe("mocha");
  });

  it("treats dark flavors as dark and Latte as light", () => {
    expect(isDarkFlavor("latte")).toBe(false);
    expect(isDarkFlavor("frappe")).toBe(true);
    expect(isDarkFlavor("macchiato")).toBe(true);
    expect(isDarkFlavor("mocha")).toBe(true);
  });
});

describe("label shadow contrast", () => {
  it("contrasts label shadows with label colors", () => {
    // Dark flavors: light labels, dark shadow (crust is dark).
    for (const flavor of ["frappe", "macchiato", "mocha"] as ThemeId[]) {
      const tokens = buildThemeTokens(PALETTES[flavor], true);
      expect(tokens.overlay.tapLabel).toBe(PALETTES[flavor].text);
      expect(tokens.overlay.tapLabelShadow).toBe(PALETTES[flavor].crust);
      expect(tokens.overlay.tapLabelShadow).not.toBe(tokens.overlay.tapLabel);
    }
    // Latte: dark labels, light shadow (crust is light).
    const latte = buildThemeTokens(PALETTES.latte, false);
    expect(latte.overlay.tapLabelShadow).toBe(PALETTES.latte.crust);
    expect(latte.overlay.tapLabelShadow).not.toBe(latte.overlay.tapLabel);
  });
});

describe("resolved theme structure", () => {
  it("returns palette, tokens, and darkness flag together", () => {
    const resolved = resolveTheme(DEFAULT_APPEARANCE, "dark");
    expect(resolved.themeId).toBe("mocha");
    expect(resolved.isDark).toBe(true);
    expect(resolved.palette).toBe(PALETTES.mocha);
    expect(resolved.tokens.app.background).toBe(PALETTES.mocha.base);
  });
});
