import type { PcSnapshot } from "../shared/api.types";
import { ISSUE_LABELS } from "./issue.messages";
import { POWER_STATUS } from "./power.messages";
import type { PendingAction, PowerAction } from "./power.types";

/** The one place that decides what a PC's button offers. */
export function stateAction(
  pc: PcSnapshot,
  pending?: PendingAction,
): PowerAction {
  if (pending === "start") return { kind: "starting" };
  if (pending === "stop") return { kind: "stopping" };
  if (pc.state === "offline") return { kind: "start", action: "start" };
  if (pc.state !== "online") return { kind: pc.state };
  if (pc.issue) return { kind: "fix", action: "fix" };
  return { kind: "stop", action: "stop" };
}

export function statusMessage(pc: PcSnapshot): string {
  return pc.issue ? ISSUE_LABELS[pc.issue] : POWER_STATUS[pc.state];
}
