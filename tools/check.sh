#!/usr/bin/env bash
# Runs every check from WSL: the Linux checks, then the Windows ones on this PC,
# reached through WSL interop.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/tools/prerequisites.sh"
need_command powershell.exe "run from WSL on the Windows PC; elsewhere run tools/check_linux.sh"
need_command wslpath "run from WSL on the Windows PC; elsewhere run tools/check_linux.sh"
report_missing
bash "$repo_root/tools/check_linux.sh"
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
    -File "$(wslpath -w "$repo_root/tools/windows/check_windows.ps1")"
