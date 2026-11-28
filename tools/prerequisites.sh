# shellcheck shell=bash
# - Sourced by the check and build scripts. They declare what they need, and
#   report_missing lists each missing tool with how to install it, then stops
#   before any work starts.
missing=()

need_command() {
    command -v "$1" >/dev/null 2>&1 || missing+=("$1: $2")
}

need_path() {
    [[ -e "$1" ]] || missing+=("$2: $3")
}

# - The toolchain rust-toolchain.toml names, with its components and targets.
need_rust_toolchain() {
    local channel
    channel="$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$1/rust-toolchain.toml")"
    rustup toolchain list 2>/dev/null | grep -q "^$channel-" \
        || missing+=("Rust $channel: rustup toolchain install (in the repository)")
}

need_rust_target() {
    rustup target list --installed 2>/dev/null | grep -qx "$1" \
        || missing+=("Rust target $1: rustup target add $1")
}

need_rust_component() {
    rustup component list --installed 2>/dev/null | grep -q "^$1" \
        || missing+=("Rust component $1: rustup component add $1")
}

report_missing() {
    if ((${#missing[@]})); then
        printf 'Missing prerequisites:\n' >&2
        printf '  - %s\n' "${missing[@]}" >&2
        exit 1
    fi
}
