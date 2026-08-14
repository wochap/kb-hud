import type { ReactNode } from "react";
import { useTheme } from "./useTheme";

/**
 * Mounts shared theming for a window root. Reads the GTK/system appearance,
 * subscribes to live appearance and configuration changes, and applies the
 * resolved semantic tokens to the document root. Both the settings and overlay
 * roots render inside this provider.
 */
export function ThemeProvider({ children }: { children: ReactNode }) {
  useTheme();
  return <>{children}</>;
}
