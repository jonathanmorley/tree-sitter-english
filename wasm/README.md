# WebAssembly language bundle (web-tree-sitter)

The Pages structure demo parses with the real grammar in the browser
via `docs/pkg/tree-sitter-english.wasm`, loaded by the vendored
`web-tree-sitter` runtime (`docs/pkg/wt/`), highlighted through
`queries/highlights.scm`.

## Rebuild

```sh
wasm/build.sh [output.wasm]   # default: docs/pkg/tree-sitter-english.wasm
```

Prerequisites: pinned tree-sitter CLI (for the wasi-sdk it caches),
a `wasm32-wasip1` Rust target, and the `rust-src` component. See
`build.sh` for the exact recipe and the Nix `libtinfo` note.

## Why this shape

- `tree-sitter build --wasm` cannot link the Rust external scanner
  (it compiles `scanner.c`, which does not exist here), so the script
  replays its exact link with the scanner built separately: the
  hermetic `bindings/rust/scanner.rs` compiled for `wasm32-wasip1`
  with a PIC-rebuilt std (`-Zbuild-std`), then linked by wasi-sdk
  clang with the CLI's flags.
- `wasi_shim.c` defines the three symbols Rust's wasi target expects
  from a C library (`errno`, `_CLOCK_REALTIME`, `_CLOCK_MONOTONIC`).
  New missing symbols surface as `Language.load` errors naming the
  symbol — define with WASI-correct values and relink.
- The demo copy of the queries lives at
  `docs/queries/highlights.scm` (regenerate by copying
  `queries/highlights.scm`); the runtime copy at `docs/pkg/wt/` is
  `web-tree-sitter@0.27.0` verbatim (matches the pinned CLI).
