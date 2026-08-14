export interface HudVisibility {
  layer: boolean;
  connection: boolean;
  gaps: boolean;
  firmwareDrops: boolean;
  battery: boolean;
  transport: boolean;
  modifiers: boolean;
}

export const DEFAULT_HUD_VISIBILITY: HudVisibility = {
  layer: true,
  connection: true,
  gaps: true,
  firmwareDrops: true,
  battery: true,
  transport: true,
  modifiers: true,
};

export interface Profile {
  name: string;
  svgPath: string;
  deviceMac: string;
  scale: number;
  hud: HudVisibility;
}
