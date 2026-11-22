#!/usr/bin/env bash
# Every check that runs on Linux; CI runs exactly this.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# - rust-toolchain.toml and .cargo/config.toml apply from the repo root.
cd "$repo_root"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPATH="$repo_root/py_modules"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/out/cargo}"
export RUFF_CACHE_DIR="${RUFF_CACHE_DIR:-$repo_root/out/cache/ruff}"
export MYPY_CACHE_DIR="${MYPY_CACHE_DIR:-$repo_root/out/cache/mypy}"
export npm_config_store_dir="${npm_config_store_dir:-$repo_root/out/cache/pnpm}"

source "$repo_root/tools/prerequisites.sh"
need_command cargo "install rustup from https://rustup.rs, then rustup toolchain install"
need_rust_toolchain "$repo_root"
need_rust_component rustfmt
need_rust_component clippy
need_rust_target x86_64-unknown-linux-musl
need_command cc "install a C compiler (Debian/Ubuntu: sudo apt install build-essential)"
need_command cargo-deny "cargo install cargo-deny --locked"
need_command cargo-machete "cargo install cargo-machete --locked"
need_command betterleaks "download from https://github.com/betterleaks/betterleaks/releases"
need_command openssl "install OpenSSL; the integration tests make TLS identities with it"
need_command pnpm "install Node.js 22.12 or newer (CI uses 24), then npm install --global pnpm@12.2.1"
need_path "$repo_root/node_modules" "plugin packages" "pnpm install --frozen-lockfile"
need_path "$repo_root/out/quality-venv/bin/mypy" "ruff and mypy" "bash tools/bootstrap_quality.sh"
report_missing

betterleaks git "$repo_root" --config "$repo_root/tools/betterleaks.toml" --no-banner --log-level warn
# * The plugin adapter's tests also drive the real agent binary.
cargo build --manifest-path "$repo_root/Cargo.toml" --locked -p myrig-cli --bin myrig
MYRIG_BINARY="$CARGO_TARGET_DIR/debug/myrig" python3 -m unittest discover -s "$repo_root/tests/plugin" -q
# - Lint and format the Python, then type-check it.
python_sources=("$repo_root/main.py" "$repo_root/py_modules")
"$repo_root/out/quality-venv/bin/ruff" check "${python_sources[@]}" "$repo_root/tests/integration"
"$repo_root/out/quality-venv/bin/ruff" format --check "${python_sources[@]}" "$repo_root/tests/integration"
# ? The integration tests import their helpers as top-level modules, so their folder goes on mypy's path.
MYPYPATH="$repo_root/py_modules:$repo_root/tests/integration" \
    "$repo_root/out/quality-venv/bin/mypy" --config-file "$repo_root/pyproject.toml" "${python_sources[@]}"
MYPYPATH="$repo_root/py_modules:$repo_root/tests/integration" \
    "$repo_root/out/quality-venv/bin/mypy" --config-file "$repo_root/pyproject.toml" "$repo_root/tests/integration"
(cd "$repo_root" && "$repo_root/out/quality-venv/bin/ruff" check tests/plugin \
    && "$repo_root/out/quality-venv/bin/ruff" format --check tests/plugin \
    && MYPYPATH=py_modules "$repo_root/out/quality-venv/bin/mypy" \
        --config-file pyproject.toml --explicit-package-bases tests/plugin)

cargo fmt --manifest-path "$repo_root/Cargo.toml" --all --check
for portable in myrig-protocol myrig-host-core myrig-client; do
    tree="$(cargo tree --manifest-path "$repo_root/Cargo.toml" --locked --offline -p "$portable" --target x86_64-pc-windows-msvc --prefix none --depth 1)"
    if printf '%s\n' "$tree" | grep -E '^windows[a-z_-]* v'; then
        printf '%s must not depend on Windows crates\n' "$portable" >&2
        exit 1
    fi
done
if grep -rnE --include='*.rs' --exclude=main.rs --exclude-dir=host-windows --exclude-dir=client-linux --exclude-dir=client-windows \
    'cfg\((not\()?windows|target_os' "$repo_root/crates"; then
    printf 'platform selection must stay in the platform crates or main.rs\n' >&2
    exit 1
fi
cargo test --manifest-path "$repo_root/Cargo.toml" --locked --workspace --all-features
cargo clippy --manifest-path "$repo_root/Cargo.toml" --locked --workspace --all-features --all-targets -- -D warnings
cargo deny --manifest-path "$repo_root/Cargo.toml" check
cargo deny --manifest-path "$repo_root/fuzz/Cargo.toml" check
# ! The fuzz targets have to build with the crate versions releases ship, so every
#   crate both lock files name needs a version the workspace lock also has.
drift="$(awk 'FNR == 1 { file++; name = "" }
    /^name = / { gsub(/"/, "", $3); name = $3 }
    /^version = / { gsub(/"/, "", $3); if (name == "") next
        if (file == 1) shipped[name] = shipped[name] " " $3 " "
        else if (name in shipped && index(shipped[name], " " $3 " ") == 0) print name, $3 }' \
    "$repo_root/Cargo.lock" "$repo_root/fuzz/Cargo.lock")"
[[ -z "$drift" ]] || { printf 'fuzz/Cargo.lock differs from Cargo.lock:\n%s\n' "$drift" >&2; exit 1; }
cargo machete "$repo_root/crates"
# ! The host CLI and the notifier reach the service only through the pipe, so neither
#   may link the host core (TLS, pairing crypto).
for client in myrig-host myrig-notifier; do
    if cargo tree --manifest-path "$repo_root/Cargo.toml" --locked -p "$client" -e normal \
        --target x86_64-pc-windows-msvc -i myrig-host-core > /dev/null 2>&1; then
        echo "$client links myrig-host-core" >&2
        exit 1
    fi
done

# ? Build the harness here, the way tests/integration/host_fixtures.py runs it.
#   Compiling it inside the harness's 60 s startup wait would make the first test
#   flaky.
cargo test --manifest-path "$repo_root/Cargo.toml" --locked --quiet -p myrig-host-core --test http_harness --no-run
python3 -m unittest discover -s "$repo_root/tests/integration" -q

pnpm --dir "$repo_root" audit --prod --audit-level high
pnpm --dir "$repo_root" test
pnpm --dir "$repo_root" run lint
pnpm --dir "$repo_root" run typecheck
pnpm --dir "$repo_root" exec knip --no-progress
(cd "$repo_root" && node_modules/.bin/jscpd --config .jscpd.json \
    main.py py_modules src crates apps/windows-app/src)
