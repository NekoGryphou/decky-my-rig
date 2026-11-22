import { callable } from "@decky/api";
import type {
  DeleteResult,
  PairingDraftResult,
  PcInput,
  PcSettings,
  SnapshotFrame,
} from "./api.types";
import { RPC_METHODS } from "./api.consts";

export const fetchSettings = callable<[], PcSettings[]>(
  RPC_METHODS.fetchSettings,
);

export const beginPairing = callable<
  [
    draftId: string,
    host: string,
    port: string,
    code: string,
    identifier: string | null,
  ],
  PairingDraftResult
>(RPC_METHODS.beginPairing);
export const requestPairingCode = callable<[host: string, port: string], void>(
  RPC_METHODS.requestPairingCode,
);
export const savePairing = callable<
  [draftId: string, name: string, mac: string],
  PcSettings
>(RPC_METHODS.savePairing);
export const cancelPairing = callable<[draftId: string], void>(
  RPC_METHODS.cancelPairing,
);
export const detectMac = callable<[host: string], string>(
  RPC_METHODS.detectMac,
);

export const updatePc = callable<
  [identifier: string, changes: PcInput],
  PcSettings
>(RPC_METHODS.updatePc);

export const deletePc = callable<[identifier: string], DeleteResult>(
  RPC_METHODS.deletePc,
);

export const startPc = callable<[identifier: string], void>(
  RPC_METHODS.startPc,
);

export const stopPc = callable<[identifier: string], boolean>(
  RPC_METHODS.stopPc,
);

export const watchSnapshot = callable<[after: number], SnapshotFrame>(
  RPC_METHODS.watchSnapshot,
);

export const setAttention = callable<[visible: boolean], void>(
  RPC_METHODS.setAttention,
);
