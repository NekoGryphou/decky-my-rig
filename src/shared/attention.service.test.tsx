import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// noinspection TypeScriptUnresolvedReference
vi.mock("@decky/api", async () => (await import("./decky.fixtures")).deckyApi);
// noinspection TypeScriptUnresolvedReference
vi.mock("@decky/ui", async () => (await import("./decky.fixtures")).deckyUi);

import { attend } from "./attention.service";
import { mocks, resetDeckyMocks } from "./decky.fixtures";

const flush = () => vi.advanceTimersByTimeAsync(0);
let releases: Array<() => void>;
const view = () => {
  const release = attend();
  releases.push(release);
  return release;
};

beforeEach(() => {
  vi.useFakeTimers();
  resetDeckyMocks();
  mocks.attention.mockResolvedValue(undefined);
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  releases = [];
});
afterEach(async () => {
  // - The service keeps its count between tests, so every view leaves.
  for (const release of releases) release();
  await flush();
  vi.useRealTimers();
});

describe("attention", () => {
  it("tells the agent a view is on screen and that the last one left", async () => {
    const release = view();
    await flush();
    expect(mocks.attention.mock.calls).toEqual([[true]]);
    release();
    await flush();
    expect(mocks.attention.mock.calls).toEqual([[true], [false]]);
  });

  it("sends nothing while another view takes over in the same turn", async () => {
    const first = view();
    await flush();
    first();
    view();
    await flush();
    expect(mocks.attention.mock.calls).toEqual([[true]]);
  });

  it("tries again on the next view change after a failure, and reports it", async () => {
    mocks.attention.mockRejectedValueOnce(new Error("offline"));
    view();
    await flush();
    expect(console.error).toHaveBeenCalledTimes(1);
    view();
    await flush();
    expect(mocks.attention.mock.calls).toEqual([[true], [true]]);
  });

  it("releases a view only once", async () => {
    const first = view();
    view();
    await flush();
    first();
    first();
    await flush();
    expect(mocks.attention.mock.calls).toEqual([[true]]);
  });
});
