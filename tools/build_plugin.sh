#!/usr/bin/env bash
# Builds the Decky plugin zip into out/release: the frontend, the Python relay
# and an agent per platform.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# - rust-toolchain.toml and .cargo/config.toml apply from the repo root.
cd "$repo_root"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/out/cargo}"
export npm_config_store_dir="${npm_config_store_dir:-$repo_root/out/cache/pnpm}"
source "$repo_root/tools/prerequisites.sh"
need_command cargo "install rustup from https://rustup.rs, then rustup toolchain install"
need_rust_toolchain "$repo_root"
need_rust_target x86_64-unknown-linux-musl
need_rust_target aarch64-unknown-linux-musl
need_command cc "install a C compiler (Debian/Ubuntu: sudo apt install build-essential)"
need_command clang "install clang for the aarch64 build (Debian/Ubuntu: sudo apt install clang)"
need_command pnpm "install Node.js 22.12 or newer, then npm install --global pnpm@12.2.1"
need_path "$repo_root/node_modules" "plugin packages" "pnpm install --frozen-lockfile"
report_missing

version="$(cat "$repo_root/VERSION")"
mkdir -p "$repo_root/out/release"
(cd "$repo_root" && cargo xtask plugin "$@" > /dev/null)
cp "$repo_root/out/build/decky-plugin/myrig-plugin-v$version.zip" "$repo_root/out/release/"
echo "$repo_root/out/release/myrig-plugin-v$version.zip"
