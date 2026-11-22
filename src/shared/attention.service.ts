import { setAttention } from "./api.service";

let views = 0;

/**
 * Marks a view of the plugin as on screen until the returned release runs.
 * The agent probes PCs every 15 s meanwhile and every 60 s otherwise.
 */
export function attend(): () => void {
  if (views++ === 0) void setAttention(true).catch(() => undefined);
  let released = false;
  return () => {
    if (released) return;
    released = true;
    if (--views === 0) void setAttention(false).catch(() => undefined);
  };
}
