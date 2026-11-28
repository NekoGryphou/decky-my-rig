import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SnapshotFrame } from "../shared/api.types";

// noinspection TypeScriptUnresolvedReference
vi.mock(
  "@decky/api",
  async () => (await import("../shared/decky.fixtures")).deckyApi,
);
// noinspection TypeScriptUnresolvedReference
vi.mock(
  "@decky/ui",
  async () => (await import("../shared/decky.fixtures")).deckyUi,
);

import { mocks } from "../shared/decky.fixtures";
import { RECONNECT_MS, WATCH_STALL_MS } from "./power.consts";
import { PowerService } from "./power.service";

interface Request {
  resolve: (frame: SnapshotFrame) => void;
  reject: (error: Error) => void;
}
let requests: Request[];
let service: PowerService;
const pc = { id: "pc", name: "Gaming PC", state: "online" } as const;
const frame = (
  revision: number,
  issue?: "authentication",
  count = 0,
  name: string | null = null,
): SnapshotFrame => ({
  revision,
  refreshed: true,
  pcs: [{ ...pc, ...(issue ? { issue } : {}) }],
  test_stops: { count, name },
  closed: false,
});
async function publish(value: SnapshotFrame) {
  const request = requests.shift();
  if (!request) throw new Error("no snapshot subscription");
  request.resolve(value);
  await Promise.resolve();
}

beforeEach(() => {
  vi.clearAllMocks();
  requests = [];
  mocks.watch.mockImplementation(
    () =>
      new Promise<SnapshotFrame>((resolve, reject) =>
        requests.push({ resolve, reject }),
      ),
  );
  mocks.toast.mockImplementation(() => ({ dismiss: vi.fn() }));
  service = new PowerService();
});
afterEach(async () => {
  service.stop();
  for (const request of requests)
    request.resolve({ ...frame(0), closed: true });
  await Promise.resolve();
  vi.useRealTimers();
});

describe("plugin-scope power state", () => {
  it("notifies with no panel mounted and deduplicates until recovery", async () => {
    service.start();
    await publish(frame(1, "authentication"));
    expect(mocks.toast).toHaveBeenCalledTimes(1);
    expect(mocks.toast.mock.calls[0]?.[0].body).toContain("Gaming PC");
    await publish(frame(2, "authentication"));
    expect(mocks.toast).toHaveBeenCalledTimes(1);
    const toast = mocks.toast.mock.results[0]?.value;
    await publish(frame(3));
    expect(toast.dismiss).toHaveBeenCalledTimes(1);
    await publish(frame(4, "authentication"));
    expect(mocks.toast).toHaveBeenCalledTimes(2);
  });

  it("dismisses issue toasts when the agent closes", async () => {
    service.start();
    await publish(frame(1, "authentication"));
    const toast = mocks.toast.mock.results[0]?.value;
    await publish({ ...frame(2), closed: true });
    expect(toast.dismiss).toHaveBeenCalledTimes(1);
  });

  it("refuses a stale Stop action after a new authentication issue", async () => {
    service.start();
    await publish(frame(1));
    await publish(frame(2, "authentication"));
    await service.act(pc.id, "stop");
    expect(mocks.stop).not.toHaveBeenCalled();
    expect(service.getSnapshot().pending.size).toBe(0);
  });

  it("preserves an action error through its failure snapshot until a successful refresh", async () => {
    service.start();
    await publish(frame(1));
    mocks.stop.mockRejectedValue(new Error("private action failure"));
    await service.act(pc.id, "stop");
    expect(service.getSnapshot().error).toContain("Gaming PC");
    await publish({ ...frame(2), refreshed: false });
    expect(service.getSnapshot().error).toContain("Gaming PC");
    await publish({ ...frame(3), refreshed: true });
    expect(service.getSnapshot().error).toBe("");
  });

  it("retains a test Stop notification after its invoking view is gone", async () => {
    let finish: ((test: boolean) => void) | undefined;
    mocks.stop.mockImplementation(
      () =>
        new Promise<boolean>((resolve) => {
          finish = resolve;
        }),
    );
    service.start();
    await publish(frame(1));
    const action = service.act(pc.id, "stop");
    expect(service.getSnapshot().pending.has(pc.id)).toBe(true);
    await publish(frame(2, undefined, 1, "Gaming PC"));
    finish?.(true);
    await action;
    expect(mocks.toast.mock.calls[0]?.[0]).toMatchObject({
      title: "Gaming PC",
      body: "Test shutdown received.",
    });
    expect(service.getSnapshot().pending.has(pc.id)).toBe(false);
    expect(service.getSnapshot().pcs[0]?.state).toBe("online");
  });

  it("accounts for coalesced test Stops without replaying the initial baseline", async () => {
    service.start();
    await publish(frame(1, undefined, 4, "Old PC"));
    expect(mocks.toast).not.toHaveBeenCalled();
    await publish(frame(3, undefined, 6, "Second PC"));
    expect(mocks.toast.mock.calls[0]?.[0].body).toBe(
      "2 test shutdowns received.",
    );
    await publish(frame(4, undefined, 6, "Second PC"));
    expect(mocks.toast).toHaveBeenCalledTimes(1);
  });

  it("does not poll from the frontend while its subscription is healthy", async () => {
    vi.useFakeTimers();
    service.start();
    await publish(frame(1));
    await vi.advanceTimersByTimeAsync(60_000);
    expect(mocks.watch).toHaveBeenCalledTimes(2);
    expect(service.getSnapshot().pcs[0]?.name).toBe("Gaming PC");
  });

  it("subscribes again when a subscription never answers", async () => {
    vi.useFakeTimers();
    try {
      service.start();
      expect(mocks.watch).toHaveBeenCalledTimes(1);
      await vi.advanceTimersByTimeAsync(WATCH_STALL_MS + RECONNECT_MS);
      expect(mocks.watch).toHaveBeenCalledTimes(2);
      requests[requests.length - 1]?.resolve(frame(1));
      await vi.advanceTimersByTimeAsync(0);
      expect(service.getSnapshot().pcs[0]?.name).toBe("Gaming PC");
    } finally {
      vi.useRealTimers();
    }
  });

  it("ignores the answer of an abandoned subscription", async () => {
    vi.useFakeTimers();
    service.start();
    await publish(frame(1));
    const orphan = requests.shift();
    await vi.advanceTimersByTimeAsync(WATCH_STALL_MS + RECONNECT_MS);
    expect(mocks.watch).toHaveBeenCalledTimes(3);
    // - The backend ends the abandoned watch later with a frame of its own.
    orphan?.resolve(frame(9, "authentication"));
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.toast).not.toHaveBeenCalled();
    expect(service.getSnapshot().pcs[0]?.issue).toBeUndefined();
    expect(mocks.watch).toHaveBeenCalledTimes(3);
  });

  it("does not republish the frame it already shows", async () => {
    service.start();
    await publish(frame(1));
    const before = service.getSnapshot();
    // - The backend ends a long wait with the revision the caller has.
    await publish(frame(1));
    expect(service.getSnapshot()).toBe(before);
    expect(mocks.watch).toHaveBeenCalledTimes(3);
  });

  it("ignores a late subscription from a disposed plugin lifetime", async () => {
    service.start();
    service.stop();
    service.start();
    await publish(frame(1, "authentication"));
    expect(mocks.toast).not.toHaveBeenCalled();
    await publish(frame(2));
    expect(service.getSnapshot().available).toBe(true);
    expect(service.getSnapshot().pcs[0]?.issue).toBeUndefined();
  });

  it("clears an RPC error on recovery and stops reconnecting on disposal", async () => {
    vi.useFakeTimers();
    service.start();
    await publish(frame(1));
    requests.shift()?.reject(new Error("private backend detail"));
    await vi.advanceTimersByTimeAsync(0);
    expect(service.getSnapshot().available).toBe(false);
    expect(service.getSnapshot().error).not.toContain("private backend detail");
    await vi.advanceTimersByTimeAsync(2000);
    await publish(frame(2));
    expect(service.getSnapshot().error).toBe("");
    expect(service.getSnapshot().available).toBe(true);
    requests.shift()?.reject(new Error("disconnected"));
    await vi.advanceTimersByTimeAsync(0);
    service.stop();
    const calls = mocks.watch.mock.calls.length;
    await vi.advanceTimersByTimeAsync(20_000);
    expect(mocks.watch).toHaveBeenCalledTimes(calls);
  });
});
