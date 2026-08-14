export type ConnectionStatus = "connected" | "connecting" | "disconnected";

/** Full keyboard state emitted by the backend on every valid record. */
export interface TelemetryState {
  connection: ConnectionStatus;
  pressed: number[];
  activeLayers: number;
  sequence: number;
  timestampMs: number;
  gaps: number;
  error?: string;
}

export const TELEMETRY_STATE_EVENT = "telemetry-state";

export const DISCONNECTED_STATE: TelemetryState = {
  connection: "disconnected",
  pressed: [],
  activeLayers: 1,
  sequence: 0,
  timestampMs: 0,
  gaps: 0,
};
