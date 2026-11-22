#!/usr/bin/env bash
# Installs the pinned ruff and mypy for the plugin's Python into out/quality-venv.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 -m venv "$repo_root/out/quality-venv"
"$repo_root/out/quality-venv/bin/pip" install --disable-pip-version-check --no-cache-dir \
    ruff==0.17.3 mypy==2.4.1
