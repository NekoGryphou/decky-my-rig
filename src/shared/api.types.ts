// - The only states the backend sends.
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

interface PcInput {
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

/** Arguments and result of every backend method, by its key in RPC_METHODS. */
export interface RpcSignatures {
  watchSnapshot: { args: [after: number]; result: SnapshotFrame };
  fetchSettings: { args: []; result: PcSettings[] };
  beginPairing: {
    args: [
      draftId: string,
      host: string,
      port: string,
      code: string,
      identifier: string | null,
    ];
    result: PairingDraftResult;
  };
  requestPairingCode: { args: [host: string, port: string]; result: void };
  savePairing: {
    args: [draftId: string, name: string, mac: string];
    result: PcSettings;
  };
  cancelPairing: { args: [draftId: string]; result: void };
  detectMac: { args: [host: string]; result: string };
  updatePc: {
    args: [identifier: string, changes: PcInput];
    result: PcSettings;
  };
  deletePc: { args: [identifier: string]; result: DeleteResult };
  startPc: { args: [identifier: string]; result: void };
  stopPc: { args: [identifier: string]; result: boolean };
  setAttention: { args: [visible: boolean]; result: void };
}
