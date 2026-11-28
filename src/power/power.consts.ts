import type { PowerViewState } from "./power.types";

export const RECONNECT_MS = 2000;
// ! The agent publishes at least once a minute. A longer silence means the backend was reloaded and
// ! this request will never be answered.
export const WATCH_STALL_MS = 90_000;
export const INITIAL_POWER_STATE: PowerViewState = {
  pcs: [],
  pending: new Map(),
  loaded: false,
  available: false,
  error: "",
};
