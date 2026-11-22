import type { PowerViewState } from "./power.types";

export const RECONNECT_MS = 2000;
export const INITIAL_POWER_STATE: PowerViewState = {
  pcs: [],
  pending: new Map(),
  loaded: false,
  available: false,
  error: "",
};
