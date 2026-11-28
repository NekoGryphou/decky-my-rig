import { describe, expect, it } from "vitest";
import { publicMessage } from "./error.utils";

const FALLBACK = "Something went wrong.";

function failure(lastLine: string): { pythonTraceback: string } {
  return {
    pythonTraceback: `Traceback (most recent call last):\n  File "main.py", line 1\n${lastLine}\n`,
  };
}

describe("publicMessage", () => {
  it("shows a plain exception's message", () => {
    expect(publicMessage(failure("Exception: Wrong code."), FALLBACK)).toBe(
      "Wrong code.",
    );
  });

  it("falls back for other exceptions so internals are not shown", () => {
    expect(publicMessage(failure("KeyError: 'token'"), FALLBACK)).toBe(
      FALLBACK,
    );
  });
});
