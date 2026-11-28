EXIT_TIMEOUT_SECONDS = 4.5
# ? Longer than the agent's own request deadlines (10 s at most); only a stuck agent reaches it.
# ? Watches are long-polls and never use it.
CALL_TIMEOUT_SECONDS = 30.0
# ! A watch that nobody waits for (the frontend gave up on it) ends after this; it must stay below
# ! the frontend's stall limit.
WATCH_TIMEOUT_SECONDS = 30.0
LINE_LIMIT = 1024 * 1024
# ? Release names are `<arch>-<os>` with compiler architecture names, like the CLI and host app
# ? files.
ARCHITECTURES = {"amd64": "x86_64", "arm64": "aarch64"}
MAX_RESTARTS = 3
REFUSED_EXIT = 1
# ! Agent error lines reach the Decky log and the user; keep each one short.
STDERR_LINE_LIMIT = 200
