import { describe, expect, it } from "vitest";
import deleteResult from "../../tests/fixtures/plugin-api/delete-result.json";
import pairingDraft from "../../tests/fixtures/plugin-api/pairing-draft.json";
import pairingDraftNoMac from "../../tests/fixtures/plugin-api/pairing-draft-no-mac.json";
import pcSettings from "../../tests/fixtures/plugin-api/pc-settings.json";
import snapshotFrame from "../../tests/fixtures/plugin-api/snapshot-frame.json";
import snapshotFrameClosed from "../../tests/fixtures/plugin-api/snapshot-frame-closed.json";
import type {
  DeleteResult,
  PairingDraftResult,
  PcSettings,
  PcSnapshot,
  SnapshotFrame,
} from "./api.types";

// * What the wire carries, field by field; Record<keyof T, ...> makes the compiler demand an entry
// * for every field a type gains.
type Kind =
  | "string"
  | "number"
  | "boolean"
  | "string|null"
  | "object"
  | "array"
  | readonly string[];
type Shape<T> = Record<keyof T, Kind>;

const POWER_STATES = ["checking", "offline", "starting", "online", "stopping"];
const PC_SNAPSHOT: Shape<Omit<PcSnapshot, "issue">> = {
  id: "string",
  name: "string",
  state: POWER_STATES,
};
const SNAPSHOT_FRAME: Shape<SnapshotFrame> = {
  revision: "number",
  refreshed: "boolean",
  pcs: "array",
  test_stops: "object",
  closed: "boolean",
};
const PC_SETTINGS: Shape<PcSettings> = {
  id: "string",
  name: "string",
  host: "string",
  port: "number",
  mac: "string",
};
const PAIRING_DRAFT: Shape<PairingDraftResult> = {
  draft_id: "string",
  name: "string",
  mac: "string|null",
  mac_error: "string|null",
};
const DELETE_RESULT: Shape<DeleteResult> = { unpaired: "boolean" };

const FIXTURES: Record<string, Record<string, unknown>> = {
  "pc-settings": pcSettings,
  "pairing-draft": pairingDraft,
  "pairing-draft-no-mac": pairingDraftNoMac,
  "delete-result": deleteResult,
  "snapshot-frame": snapshotFrame,
  "snapshot-frame-closed": snapshotFrameClosed,
};
function fixture(name: string): Record<string, unknown> {
  const value = FIXTURES[name];
  if (!value) throw new Error(`no fixture ${name}`);
  return value;
}

function matches(value: unknown, kind: Kind): boolean {
  if (typeof kind !== "string") return kind.includes(value as string);
  if (kind === "string|null")
    return value === null || typeof value === "string";
  if (kind === "array") return Array.isArray(value);
  if (kind === "object") return typeof value === "object" && value !== null;
  return typeof value === kind;
}

function problems(value: Record<string, unknown>, shape: object): string[] {
  const kinds = shape as Record<string, Kind>;
  return [
    ...Object.keys(value)
      .filter((key) => !(key in kinds) && key !== "issue")
      .map((key) => `unexpected field ${key}`),
    ...Object.entries(kinds)
      .filter(([key, kind]) => !matches(value[key], kind))
      .map(([key]) => `field ${key} has the wrong type`),
  ];
}

describe("backend payloads", () => {
  it.each([
    ["pc-settings", PC_SETTINGS],
    ["pairing-draft", PAIRING_DRAFT],
    ["pairing-draft-no-mac", PAIRING_DRAFT],
    ["delete-result", DELETE_RESULT],
    ["snapshot-frame", SNAPSHOT_FRAME],
    ["snapshot-frame-closed", SNAPSHOT_FRAME],
  ])("%s matches its TypeScript type", (name, shape) => {
    expect(problems(fixture(name), shape)).toEqual([]);
  });

  it("matches every PC of a frame and its test stops", () => {
    const frame = fixture("snapshot-frame");
    for (const pc of frame.pcs as Record<string, unknown>[])
      expect(problems(pc, PC_SNAPSHOT)).toEqual([]);
    expect(frame.test_stops).toEqual({ count: 2, name: "Salon" });
    expect((frame.pcs as PcSnapshot[]).map((pc) => pc.issue)).toEqual([
      undefined,
      "authentication",
    ]);
  });

  it("detects a field of the wrong type", () => {
    const settings = { ...fixture("pc-settings"), port: "47991" };
    expect(problems(settings, PC_SETTINGS)).toEqual([
      "field port has the wrong type",
    ]);
  });
});
