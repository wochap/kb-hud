import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { OverlayApp } from "./overlay/OverlayApp";
import { SettingsView } from "./settings/SettingsView";
import { ThemeProvider } from "./theme/ThemeProvider";
import "./index.css";

function currentWindowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "overlay";
  }
}

function root() {
  switch (currentWindowLabel()) {
    case "settings":
      return <SettingsView />;
    case "overlay":
      return <OverlayApp />;
    default:
      return <App />;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>{root()}</ThemeProvider>
  </React.StrictMode>,
);
