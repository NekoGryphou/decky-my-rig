import type { PcSnapshot } from "../shared/api.types";

export type PendingAction = "start" | "stop";

export interface PowerViewState {
  pcs: PcSnapshot[];
  pending: ReadonlyMap<string, PendingAction>;
  loaded: boolean;
  available: boolean;
  error: string;
}

export type PowerKind =
  PendingAction | "fix" | "checking" | "starting" | "stopping";

export interface PowerAction {
  kind: PowerKind;
  action?: PendingAction | "fix";
}

export interface StopConfirmationProps {
  pc: PcSnapshot;
  closeModal?: () => void;
  /** Runs once the question leaves the screen, however it was answered. */
  onDone?: () => void;
}
