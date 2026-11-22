import type { PowerState } from "../shared/api.types";
import type { PowerKind } from "./power.types";

export const QAM_ERROR = "Action failed. Open settings and try again.";
export const QAM_UNAVAILABLE = "PC status is unavailable.";
export const STOP_WARNING = "Unsaved work may be lost.";

export const POWER_STATUS: Record<PowerState, string> = {
  online: "Online",
  offline: "Offline",
  checking: "Checking…",
  starting: "Starting…",
  stopping: "Stopping…",
};
const ACTION_NAMES: Record<PowerKind, string> = {
  start: "Start",
  stop: "Stop",
  fix: "Fix",
  checking: "Checking",
  starting: "Starting",
  stopping: "Stopping",
};
export const POWER_LABELS: Record<PowerKind, string> = {
  ...ACTION_NAMES,
  checking: POWER_STATUS.checking,
  starting: POWER_STATUS.starting,
  stopping: POWER_STATUS.stopping,
};

export const actionLabel = (kind: PowerKind, name: string) =>
  `${ACTION_NAMES[kind]} ${name}`;
export const stopTitle = (name: string) => `${ACTION_NAMES.stop} ${name}?`;
