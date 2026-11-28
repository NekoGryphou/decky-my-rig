#!/usr/bin/env bash
# Writes the release version into the manifests. The version comes from the tag
# (v1.2.3) when CI builds a tag, else from the latest tag, else 0.0.0. The
# repository keeps 0.0.0 and CI rewrites it; nothing else holds a version.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "${GITHUB_REF_TYPE:-}" == "tag" ]]; then
    tag="$GITHUB_REF_NAME"
else
    tag="$(git describe --tags --match 'v[0-9]*' --abbrev=0 2>/dev/null || echo v0.0.0)"
fi
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
