#!/usr/bin/env bash
set -euo pipefail

seconds="${1:-60}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# - fuzz/rust-toolchain.toml applies from the fuzz crate.
cd "$repo_root/fuzz"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/out/cargo}"
source "$repo_root/tools/prerequisites.sh"
need_command cargo "install rustup from https://rustup.rs"
need_rust_toolchain "$repo_root/fuzz"
cargo fuzz --version >/dev/null 2>&1 \
    || missing+=("cargo-fuzz: cargo install cargo-fuzz --version 0.13.2 --locked")
report_missing
# ! The fuzz lock must not be rewritten under the run; see check_linux.sh.
cargo fetch --locked
# ! cargo-fuzz builds for the target it was built for itself. CI downloads
#   a static musl build, and sanitizers can't link musl statically.
host="$(rustc -vV | sed -n 's/^host: //p')"

# ! A failing command substitution in a for list does not stop the script.
targets="$(cargo fuzz list)"
[[ -n "$targets" ]] || { echo "No fuzz targets found." >&2; exit 1; }
for target in $targets; do
    corpus="$repo_root/out/fuzz/corpus/$target"
    artifacts="$repo_root/out/fuzz/artifacts/$target"
    mkdir -p "$corpus" "$artifacts"
    log="$repo_root/out/fuzz/$target.log"
    echo "fuzz $target (${seconds}s)"
    if ! cargo fuzz run --target "$host" "$target" "$corpus" -- \
        -max_total_time="$seconds" -artifact_prefix="$artifacts/" >"$log" 2>&1; then
        grep -E -A3 'panicked|ERROR' "$log" >&2 || tail -20 "$log" >&2
        echo "fuzz $target failed; the input is in $artifacts" >&2
        exit 1
    fi
done
