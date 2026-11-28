#!/usr/bin/env bash
# Writes the release version from the tag into the manifests. Only a tag build
# (vX.Y.Z) changes anything; any other build keeps the 0.0.0 placeholder.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "${GITHUB_REF_TYPE:-}" != "tag" ]]; then
    echo "not a tag build; the version stays 0.0.0" >&2
    exit 0
fi
tag="$GITHUB_REF_NAME"
version="${tag#v}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "not a version: $tag (expected vX.Y.Z)" >&2
    exit 1
fi

python3 - "$version" <<'PY'
import re, sys
from pathlib import Path
version = sys.argv[1]

# ? Bytes in and out: text mode would write CRLF on Windows, and the stamp hashes these files.
def read(path):
    return Path(path).read_bytes().decode("utf-8")

def write(path, text):
    Path(path).write_bytes(text.encode("utf-8"))

def rewrite(path, pattern, replacement, count=1):
    text = read(path)
    new, n = re.subn(pattern, replacement, text, count=count, flags=re.M)
    if n != count:
        raise SystemExit(f"{path}: expected {count} version line(s), found {n}")
    write(path, new)

rewrite("Cargo.toml", r'^version = "[^"]*"', f'version = "{version}"')
rewrite("package.json", r'"version": "[^"]*"', f'"version": "{version}"')
rewrite("apps/windows-app/Directory.Build.props", r"<Version>[^<]*</Version>", f"<Version>{version}</Version>")

# Workspace members are the lock entries without a source (registry or git).
lock = read("Cargo.lock")
lock, n = re.subn(r'(name = "[^"]+"\nversion = )"[^"]*"(\n(?!source = ))', rf'\g<1>"{version}"\g<2>', lock)
if n == 0:
    raise SystemExit("Cargo.lock: no workspace packages found")
write("Cargo.lock", lock)

# The fuzz workspace pins the same packages, but its own crate keeps its version.
fuzz = read("fuzz/Cargo.lock")
fuzz, n = re.subn(r'(name = "myrig(?!-fuzz")[^"]*"\nversion = )"[^"]*"(\n(?!source = ))', rf'\g<1>"{version}"\g<2>', fuzz)
if n == 0:
    raise SystemExit("fuzz/Cargo.lock: no myrig packages found")
write("fuzz/Cargo.lock", fuzz)
print(version)
PY
