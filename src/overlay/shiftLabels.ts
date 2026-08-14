import { MOD_LSFT, MOD_RSFT } from "../telemetry";

const US_SHIFTED: Readonly<Record<string, string>> = {
  "1": "!",
  "2": "@",
  "3": "#",
  "4": "$",
  "5": "%",
  "6": "^",
  "7": "&",
  "8": "*",
  "9": "(",
  "0": ")",
  "-": "_",
  "=": "+",
  "[": "{",
  "]": "}",
  "\\": "|",
  ";": ":",
  "'": '"',
  "`": "~",
  ",": "<",
  ".": ">",
  "/": "?",
};

export function shiftActive(modifiers: number): boolean {
  return (modifiers & (MOD_LSFT | MOD_RSFT)) !== 0;
}

/** Applies a deliberately US-only Shift preview to a keymap-drawer tap label. */
export function shiftedTapLabel(label: string, modifiers: number): string {
  if (!shiftActive(modifiers)) return label;
  if (/^[a-z]$/i.test(label)) return label.toUpperCase();
  return US_SHIFTED[label] ?? label;
}
