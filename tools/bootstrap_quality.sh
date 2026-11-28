#!/usr/bin/env bash
# - Installs the pinned ruff and mypy into out/quality-venv. Every package must
#   match its hash in quality_requirements.txt.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 -m venv "$repo_root/out/quality-venv"
"$repo_root/out/quality-venv/bin/pip" install --disable-pip-version-check --no-cache-dir \
    --require-hashes --only-binary=:all: -r "$repo_root/tools/quality_requirements.txt"
