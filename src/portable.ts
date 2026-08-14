/** Frontend mirrors of the portable import summary returned by the backend. */

export interface ImportKeymapResult {
  profile: string;
  /** "embedded", "none", or "invalid". */
  status: string;
  valid: boolean;
}

export interface ImportSummary {
  profileCount: number;
  activeProfile: string;
  lightTheme: string;
  darkTheme: string;
  keymaps: ImportKeymapResult[];
  /** Imported profiles always reset to automatic device discovery. */
  deviceResetToAuto: boolean;
}
