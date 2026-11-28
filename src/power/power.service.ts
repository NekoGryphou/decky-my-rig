import { api } from "../shared/api.service";
import type { SnapshotFrame } from "../shared/api.types";
import { NotificationService } from "./notifications/notifications.service";
import {
  INITIAL_POWER_STATE,
  RECONNECT_MS,
  WATCH_STALL_MS,
} from "./power.consts";
import { publicMessage } from "../shared/error.utils";
import { POWER_STATUS, QAM_ERROR, QAM_UNAVAILABLE } from "./power.messages";
import { stateAction, statusMessage } from "./power.utils";
import type { PendingAction, PowerViewState } from "./power.types";

export class PowerService {
  private state: PowerViewState = INITIAL_POWER_STATE;
  private listeners = new Set<() => void>();
  private notifications = new NotificationService();
  private generation = 0;
  private active = false;
  private actionError = "";
  private retry: ReturnType<typeof setTimeout> | undefined;

  getSnapshot = (): PowerViewState => this.state;
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  start(): void {
    if (this.active) return;
    this.active = true;
    const generation = ++this.generation;
    this.notifications.reset();
    this.actionError = "";
    this.update(INITIAL_POWER_STATE);
    void this.watch(generation);
  }

  stop(): void {
    this.active = false;
    this.generation++;
    if (this.retry !== undefined) clearTimeout(this.retry);
    this.retry = undefined;
    this.notifications.reset();
  }

  /** Runs a power action if the PC's current state still offers it. */
  async act(id: string, action: PendingAction): Promise<void> {
    if (!this.active || !this.state.available || this.state.pending.has(id))
      return;
    const pc = this.state.pcs.find((candidate) => candidate.id === id);
    if (!pc || stateAction(pc).action !== action) return;
    const generation = this.generation;
    this.actionError = "";
    this.update({
      pending: new Map(this.state.pending).set(pc.id, action),
      error: "",
    });
    try {
      if (action === "start") await api.startPc(pc.id);
      else await api.stopPc(pc.id);
    } catch (failure) {
      if (this.current(generation)) {
        const reason = publicMessage(failure, QAM_ERROR);
        this.actionError = `${pc.name}: ${reason}`;
        this.update({ error: this.actionError });
      }
    } finally {
      if (this.current(generation)) {
        const pending = new Map(this.state.pending);
        pending.delete(pc.id);
        this.update({ pending });
      }
    }
  }

  statusOf(id: string): string {
    const pc = this.state.pcs.find((candidate) => candidate.id === id);
    return pc ? statusMessage(pc) : POWER_STATUS.checking;
  }

  private update(changes: Partial<PowerViewState>): void {
    this.state = { ...this.state, ...changes };
    for (const listener of this.listeners) listener();
  }

  private current(generation: number): boolean {
    return this.active && generation === this.generation;
  }

  private async watch(generation: number): Promise<void> {
    let after = -1;
    while (this.current(generation)) {
      let frame: SnapshotFrame;
      try {
        frame = await subscribe(after);
      } catch {
        if (this.current(generation)) {
          this.update({
            loaded: true,
            available: false,
            error: QAM_UNAVAILABLE,
          });
          this.retry = setTimeout(() => {
            this.retry = undefined;
            void this.watch(generation);
          }, RECONNECT_MS);
        }
        return;
      }
      if (!this.current(generation)) return;
      if (frame.closed) {
        this.notifications.reset();
        this.active = false;
        this.update({ loaded: true, available: false, error: QAM_UNAVAILABLE });
        return;
      }
      // - The backend ends an unanswered watch with the frame already shown.
      if (frame.revision === after) continue;
      this.notifications.update(frame.pcs, frame.test_stops);
      if (frame.refreshed) this.actionError = "";
      this.update({
        pcs: frame.pcs,
        loaded: true,
        available: true,
        error: this.actionError,
      });
      after = frame.revision;
    }
  }
}

/// One snapshot request. A request that never answers is rejected after the
/// stall limit, so the watch loop retries instead of waiting forever.
function subscribe(after: number): Promise<SnapshotFrame> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error("snapshot stalled")),
      WATCH_STALL_MS,
    );
    api.watchSnapshot(after).then(
      (frame) => {
        clearTimeout(timer);
        resolve(frame);
      },
      (error: unknown) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

export const powerService = new PowerService();
