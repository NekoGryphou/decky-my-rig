// * The only states the backend sends.
export type PowerState =
  "checking" | "offline" | "starting" | "online" | "stopping";

export interface PcSnapshot {
  id: string;
  name: string;
  state: PowerState;
  issue?: "authentication" | "compatibility" | "identity";
}

export interface PcSettings {
  id: string;
  name: string;
  host: string;
  port: number;
  mac: string;
}

export interface PcInput {
  name: string;
  host: string;
  port: string;
  mac: string;
}

export interface SnapshotFrame {
  revision: number;
  refreshed: boolean;
  pcs: PcSnapshot[];
  test_stops: { count: number; name: string | null };
  closed: boolean;
}

export interface PairingDraftResult {
  draft_id: string;
  name: string;
  mac: string | null;
  mac_error: string | null;
}

/** Whether the PC also forgot this device when its settings were deleted. */
export interface DeleteResult {
  unpaired: boolean;
}
