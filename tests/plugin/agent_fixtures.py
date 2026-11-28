"""A stand-in agent speaking the JSON-lines protocol, one behavior per mode."""

import json
import sys
import time
from pathlib import Path

MODE = sys.argv[1]
MARKER = Path(sys.argv[2]) if len(sys.argv) > 2 else None
if MODE == "crash-once" and MARKER is not None and not MARKER.exists():
    # - A crash on the first run only: a restarted agent works.
    MARKER.touch()
    sys.exit(3)
if MODE == "refuse":
    # - The real agent's startup refusal: one public message, exit code 1.
    print("Cannot load PC configuration; existing data was not reset.", file=sys.stderr)
    sys.exit(1)
if MODE == "oversize":
    # - A reply line over the client's limit from an agent that stays alive.
    print("x" * 200, flush=True)
    sys.stdin.read()
    sys.exit(0)
if MODE == "noisy":
    # - An error line over the limit, then an ordinary one.
    print("y" * 200, file=sys.stderr, flush=True)
    print("after the long line", file=sys.stderr, flush=True)
if MODE == "wedge":
    # - Alive but silent after the first snapshot: only a timeout can notice.
    print(
        json.dumps(
            {
                "event": "snapshot",
                "frame": {
                    "revision": 1,
                    "pcs": [],
                    "test_stops": {"count": 0, "name": None},
                    "closed": False,
                    "refreshed": False,
                },
            }
        ),
        flush=True,
    )
    if MARKER is not None and not MARKER.exists():
        MARKER.touch()
        sys.stdin.readline()
        time.sleep(30)
if MODE == "loud":
    # - One long error line with control characters inside.
    print("a\x1b[31mb\x00" + "z" * 5000, file=sys.stderr, flush=True)
if MODE == "linger":
    # - Ignores stdin EOF, like an agent that hangs on exit.
    sys.stdin.read()
    time.sleep(30)
if MODE == "junk":
    # - Valid JSON that is not a reply: each line must be ignored.
    for junk in ("1", "[]", '{"id": [1]}', '{"event": "snapshot"}', '{"error": 1}'):
        print(junk, flush=True)
print(
    json.dumps(
        {
            "event": "snapshot",
            "frame": {
                "revision": 1,
                "pcs": [],
                "test_stops": {"count": 0, "name": None},
                "closed": False,
                "refreshed": False,
            },
        }
    ),
    flush=True,
)
for line in sys.stdin:
    request = json.loads(line)
    if MODE == "exit":
        sys.exit(3)
    if MODE == "hang" and request["method"] == "begin_pairing":
        # - Tells the test the request arrived; it is never answered.
        print("pairing received", file=sys.stderr, flush=True)
        continue
    if MODE == "error":
        reply = {"id": request["id"], "error": "PC was not found."}
    else:
        reply = {
            "id": request["id"],
            "result": {"method": request["method"], "params": request["params"]},
        }
    print(json.dumps(reply), flush=True)
