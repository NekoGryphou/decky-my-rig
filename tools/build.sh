#!/usr/bin/env bash
# Builds every release file into out/release.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bash "$repo_root/tools/build_linux.sh"
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
    -File "$(wslpath -w "$repo_root/tools/windows/build_windows.ps1")"
bash "$repo_root/tools/build_plugin.sh" --windows x86_64
