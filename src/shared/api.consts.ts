export const RPC_METHODS = {
  watchSnapshot: "watch_snapshot",
  fetchSettings: "settings",
  beginPairing: "begin_pairing",
  requestPairingCode: "request_pairing_code",
  savePairing: "save_pairing",
  cancelPairing: "cancel_pairing",
  detectMac: "detect_mac",
  updatePc: "update_pc",
  deletePc: "delete_pc",
  startPc: "start",
  stopPc: "stop",
  setAttention: "set_attention",
} as const;
