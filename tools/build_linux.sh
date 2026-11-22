#!/usr/bin/env bash
# Builds the Linux CLI archives into out/release.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# - rust-toolchain.toml and .cargo/config.toml apply from the repo root.
cd "$repo_root"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/out/cargo}"
source "$repo_root/tools/prerequisites.sh"
need_command cargo "install rustup from https://rustup.rs, then rustup toolchain install"
need_rust_toolchain "$repo_root"
need_rust_target x86_64-unknown-linux-musl
need_rust_target aarch64-unknown-linux-musl
need_command cc "install a C compiler (Debian/Ubuntu: sudo apt install build-essential)"
need_command clang "install clang for the aarch64 build (Debian/Ubuntu: sudo apt install clang)"
report_missing

version="$(cat "$repo_root/VERSION")"
release="$repo_root/out/release"
mkdir -p "$release"
for arch in x86_64 aarch64; do
    cargo build --manifest-path "$repo_root/Cargo.toml" --locked --release -p myrig-cli \
        --target "$arch-unknown-linux-musl"
    tar -czf "$release/myrig-cli-v$version-$arch-linux.tar.gz" \
        -C "$CARGO_TARGET_DIR/$arch-unknown-linux-musl/release" myrig
done
ls -1 "$release"
