import { callable } from "@decky/api";
import { RPC_METHODS } from "./api.consts";
import type { RpcSignatures } from "./api.types";

type RpcApi = {
  [Key in keyof typeof RPC_METHODS]: (
    ...args: RpcSignatures[Key]["args"]
  ) => Promise<RpcSignatures[Key]["result"]>;
};

// * One callable per entry of RPC_METHODS, so a method exists once.
export const api = Object.fromEntries(
  Object.entries(RPC_METHODS).map(([key, method]) => [key, callable(method)]),
) as RpcApi;
