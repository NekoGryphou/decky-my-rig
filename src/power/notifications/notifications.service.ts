import { toaster } from "@decky/api";
// noinspection ES6PreferShortImport
import type { PcSnapshot, SnapshotFrame } from "../../shared/api.types";
// noinspection ES6PreferShortImport
import { EditorNavigation } from "../../shared/navigation.service";
// noinspection ES6PreferShortImport
import { PRODUCT_NAME } from "../../shared/product.messages";
import { issueMessage } from "../issue.utils";
import { testShutdownsReceived } from "./notifications.messages";

/** Toasts for new PC issues and test-mode stops. */
export class NotificationService {
  private issues = new Map<
    string,
    ReturnType<typeof toaster.toast> | undefined
  >();
  private testCount: number | undefined;

  update(pcs: PcSnapshot[], stops: SnapshotFrame["test_stops"]): void {
    const current = new Set<string>();
    for (const pc of pcs) {
      if (!pc.issue) continue;
      const key = `${pc.id}:${pc.issue}`;
      current.add(key);
      if (!this.issues.has(key)) {
        this.issues.set(
          key,
          toaster.toast({
            title: PRODUCT_NAME,
            body: `${pc.name}: ${issueMessage(pc.issue)}`,
            onClick: () => EditorNavigation.open(pc.id),
            showToast: true,
          }),
        );
      }
    }
    for (const [key, notification] of this.issues) {
      if (!current.has(key)) {
        notification?.dismiss();
        this.issues.delete(key);
      }
    }
    if (this.testCount !== undefined && stops.count > this.testCount) {
      const count = stops.count - this.testCount;
      toaster.toast({
        title: count === 1 && stops.name ? stops.name : PRODUCT_NAME,
        body: testShutdownsReceived(count),
        showToast: true,
      });
    }
    this.testCount = stops.count;
  }

  reset(): void {
    for (const notification of this.issues.values()) notification?.dismiss();
    this.issues.clear();
    this.testCount = undefined;
  }
}
