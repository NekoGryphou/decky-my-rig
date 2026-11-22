const BACKEND_ERROR = /^Exception: (.+)$/;

/** The backend's message for the user, or the fallback when there is none. */
export function publicMessage(error: unknown, fallback: string): string {
  if (typeof error !== "object" || error === null) return fallback;
  const { pythonTraceback } = error as { pythonTraceback?: unknown };
  if (typeof pythonTraceback !== "string") return fallback;
  const lines = pythonTraceback.trim().split("\n");
  return BACKEND_ERROR.exec(lines[lines.length - 1] ?? "")?.[1] ?? fallback;
}
