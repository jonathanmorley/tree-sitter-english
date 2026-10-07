/* WASI libc shims for the web-tree-sitter language bundle.
 *
 * The bundle links `-nostdlib`, so a few symbols Rust's wasi target
 * expects from a C library must be defined here. Values follow WASI:
 * clocks are `__wasi_clockid_t` (REALTIME=0, MONOTONIC=1); `errno` is
 * a plain zero cell (single-threaded; the scanner never sets it).
 * If `Language.load` ever reports a new missing symbol, define it
 * here with its WASI-correct value and relink via `build.sh`.
 */
int errno = 0;
unsigned _CLOCK_REALTIME = 0;
unsigned _CLOCK_MONOTONIC = 1;
