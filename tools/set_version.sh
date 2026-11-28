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

def rewrite(path, pattern, replacement, count=1):
    text = Path(path).read_text(encoding="utf-8")
    new, n = re.subn(pattern, replacement, text, count=count, flags=re.M)
    if n != count:
        raise SystemExit(f"{path}: expected {count} version line(s), found {n}")
    Path(path).write_text(new, encoding="utf-8")

rewrite("Cargo.toml", r'^version = "[^"]*"', f'version = "{version}"')
rewrite("package.json", r'"version": "[^"]*"', f'"version": "{version}"')
rewrite("apps/windows-app/Directory.Build.props", r"<Version>[^<]*</Version>", f"<Version>{version}</Version>")

# Workspace members are the lock entries without a source (registry or git).
lock = Path("Cargo.lock").read_text(encoding="utf-8")
lock, n = re.subn(r'(name = "[^"]+"\nversion = )"[^"]*"(\n(?!source = ))', rf'\g<1>"{version}"\g<2>', lock)
if n == 0:
    raise SystemExit("Cargo.lock: no workspace packages found")
Path("Cargo.lock").write_text(lock, encoding="utf-8")

# The fuzz workspace pins the same packages, but its own crate keeps its version.
fuzz = Path("fuzz/Cargo.lock").read_text(encoding="utf-8")
fuzz, n = re.subn(r'(name = "myrig(?!-fuzz")[^"]*"\nversion = )"[^"]*"(\n(?!source = ))', rf'\g<1>"{version}"\g<2>', fuzz)
if n == 0:
    raise SystemExit("fuzz/Cargo.lock: no myrig packages found")
Path("fuzz/Cargo.lock").write_text(fuzz, encoding="utf-8")
print(version)
PY
