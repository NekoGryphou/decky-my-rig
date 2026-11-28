import { api } from "./api.service";

let views = 0;
// * What the agent last acknowledged; a failed update leaves it unchanged, so the next view change
// * sends again.
let reported = false;
let syncing = false;

/**
 * Marks a view of the plugin as on screen until the returned release runs.
 * The agent probes PCs every 15 s meanwhile and every 60 s otherwise.
 */
export function attend(): () => void {
  views++;
  schedule();
  let released = false;
  return () => {
    if (released) return;
    released = true;
    views--;
    schedule();
  };
}

// - A page change releases one view and takes another in the same turn; the
//   agent only needs to hear about the state they end in.
function schedule(): void {
  void Promise.resolve().then(sync);
}

async function sync(): Promise<void> {
  if (syncing) return;
  syncing = true;
  try {
    while (reported !== views > 0) {
      const wanted = views > 0;
      try {
        await api.setAttention(wanted);
        reported = wanted;
      } catch (failure) {
        console.error("Decky My Rig could not update its attention.", failure);
        return;
      }
    }
  } finally {
    syncing = false;
  }
}
