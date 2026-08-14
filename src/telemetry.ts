export type ConnectionStatus = "connected" | "connecting" | "disconnected";
export type EndpointTransport = "usb" | "ble";
export type SplitStatus = "connected" | "disconnected";

export const MOD_LCTL = 1 << 0;
export const MOD_LSFT = 1 << 1;
export const MOD_LALT = 1 << 2;
export const MOD_LGUI = 1 << 3;
export const MOD_RCTL = 1 << 4;
export const MOD_RSFT = 1 << 5;
export const MOD_RALT = 1 << 6;
export const MOD_RGUI = 1 << 7;

export const FIELD_POSITIONS = 1 << 0;
export const FIELD_LAYERS = 1 << 1;
export const FIELD_MODIFIERS = 1 << 2;
export const FIELD_HID_INDICATORS = 1 << 3;
export const FIELD_DEFAULT_LAYER = 1 << 4;
export const FIELD_ENDPOINT = 1 << 5;
export const FIELD_CENTRAL_BATTERY = 1 << 6;
export const FIELD_PERIPHERAL_BATTERY = 1 << 7;
export const FIELD_SPLIT_STATUS = 1 << 8;

/** Full keyboard state emitted by the backend on every valid v2 frame. */
export interface TelemetryState {
  connection: ConnectionStatus;
  snapshot: boolean;
  pressed: number[];
  activeLayers: number;
  changedFields: number;
  validFields: number;
  modifiers: number;
  sequence: number;
  timestampMs: number;
  firmwareDrops: number;
  gaps: number;
  hidIndicators?: number;
  defaultLayer?: number;
  transport?: EndpointTransport;
  bleProfile?: number;
  centralBatteryPct?: number;
  peripheralBatteryPct?: number;
  splitStatus?: SplitStatus;
  error?: string;
}

export const TELEMETRY_STATE_EVENT = "telemetry-state";

export const DISCONNECTED_STATE: TelemetryState = {
  connection: "disconnected",
  snapshot: false,
  pressed: [],
  activeLayers: 1,
  changedFields: 0,
  validFields: 0,
  modifiers: 0,
  sequence: 0,
  timestampMs: 0,
  firmwareDrops: 0,
  gaps: 0,
};
