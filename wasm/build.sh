#!/bin/sh
# Build the web-tree-sitter language bundle for the Pages demo:
#   wasm/build.sh [output.wasm]
# Default output is docs/pkg/tree-sitter-english.wasm.
#
# Recipe (mirrors `tree-sitter build --wasm`, whose exact flags were
# read from the CLI 0.27 loader source):
#   1. Compile the Rust external scanner for wasm32-wasip1 as a PIC
#      staticlib. `-Zbuild-std` is load-bearing: prebuilt std objects
#      are non-PIC and the shared link rejects absolute relocations.
#      Needs the rust-src component; RUSTC_BOOTSTRAP unlocks -Zbuild-std
#      on stable (no nightly required).
#   2. Link parser.c + wasi_shim.c + libscanner.a with the CLI-cached
#      wasi-sdk clang (downloaded automatically by any
#      `tree-sitter build --wasm` run).
#
# Nix note: the cached wasi-sdk clang needs libtinfo.so.6; provide it
# via LD_LIBRARY_PATH from nixpkgs ncurses, e.g.
#   export LD_LIBRARY_PATH=$(nix build --print-out-paths \
#     --no-link nixpkgs#ncurses)/lib:$LD_LIBRARY_PATH
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT=${1:-"$ROOT/docs/pkg/tree-sitter-english.wasm"}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT INT TERM

WASI_CLANG="$HOME/.cache/tree-sitter/wasi-sdk/bin/clang"
if [ ! -x "$WASI_CLANG" ]; then
    echo "wasi-sdk clang not found; run 'tree-sitter build --wasm' once first" >&2
    exit 1
fi

mkdir -p "$WORK/scanner-build/src"
cp "$ROOT/bindings/rust/scanner.rs" "$WORK/scanner-build/src/lib.rs"
cat > "$WORK/scanner-build/Cargo.toml" << 'EOF'
[package]
name = "scanner"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["staticlib"]
EOF

export RUSTC_BOOTSTRAP=1
export RUSTFLAGS="-Crelocation-model=pic -Copt-level=s"
cargo build --release --target wasm32-wasip1 \
    -Zbuild-std=std,panic_abort \
    --manifest-path "$WORK/scanner-build/Cargo.toml"
SCANNER_LIB=$(echo "$WORK"/scanner-build/target/wasm32-wasip1/release/libscanner.a)

# shellcheck disable=SC2086
"$WASI_CLANG" --target=wasm32-wasip1 \
    -o "$OUT" \
    -fPIC -shared --no-wasm-opt -Os \
    -Wl,--export=tree_sitter_english \
    -Wl,--allow-undefined -Wl,--no-entry \
    -nostdlib -fno-exceptions -fvisibility=hidden \
    -I "$ROOT/src" \
    "$ROOT/src/parser.c" "$ROOT/wasm/wasi_shim.c" "$SCANNER_LIB"
echo "wrote $OUT"
