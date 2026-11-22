import type { PcSnapshot } from "../shared/api.types";
import { ISSUE_LABELS, ISSUE_SETTINGS_SUFFIX } from "./issue.messages";

export function issueMessage(issue: NonNullable<PcSnapshot["issue"]>): string {
  return `${ISSUE_LABELS[issue]}${ISSUE_SETTINGS_SUFFIX}`;
}
