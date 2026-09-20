# AGENTS.md

Working notes for this repo. Unsigned commits until just before pushing
(signing happens once, at the end).

## Toolchain (`nix develop`)

- Generate with the pinned CLI, not the flake's:
  `npx -y tree-sitter-cli@0.27.0 generate` (package.json pin).
  The shell's CLI is 0.26.9; generating with it churns only
  `src/tree_sitter/array.h`, so don't.
- Tests: `cargo test --workspace`. Bare `cargo test` at the root only
  covers the root package.
- Lint/format: `cargo fmt --check`, `cargo clippy --workspace --all-targets`,
  `nix fmt`, `nix flake check` — all must be clean.

## The CLI cannot test this grammar

The external scanner is Rust (`bindings/rust/scanner.rs`), which the
tree-sitter CLI cannot link, so `tree-sitter test` / `tree-sitter parse`
do not work. The corpus (`test/corpus/*.txt`) runs under cargo via
`crates/english/tests/corpus.rs` instead.

Beware the stale cache: the CLI caches builds at
`~/.cache/tree-sitter/lib/english.dylib` and will happily pass tests
against a deleted scanner. Delete it if CLI results look suspicious.

## `tree-sitter init --update` is not safe to run blindly

- It duplicated the `let dir` line in `Package.swift` (its Swift
  migration re-applies against its own template output). Always review
  its diff; revert hunks like that one.
- Hand edits in `bindings/rust/lib.rs` (doctest sample, `mod scanner`)
  and `bindings/rust/build.rs` (clippy allow) survive it — the updater
  only patches narrow template sections. Verified, not assumed.
- Upstream template bug: the pyproject `, email = "PARSER_AUTHOR_EMAIL"`
  placeholder constant doesn't match its own
  `{ email = ..., name = ... }` template, so it is never substituted.
  Fixed by hand; a re-run will not re-break it (nor fix it).

## Grammar constraints (learned the hard way)

- `source_file` keeps blank-line `paragraph_break` as a positional
  separator so same-line sentences never split into paragraphs. Making
  the paragraph core optional for empty input is fine, but the trailing
  break repeat must live *inside* the optional — outside it the leading
  and trailing repeats conflict (`source_file_repeat1` ambiguity) and
  `generate` fails. Zero `generate` conflicts is a hard requirement.
- The typed AST (`crates/english`) covers only nodes the grammar really
  produces (flat clauses; no noun/verb phrases — SVO fields were tried
  upstream and dropped, see `grammar.js` Tier 3).
- Corpus tests are TDD: add the failing expectation to
  `test/corpus/*.txt` first. Note the input model: lines strictly
  between the header and `---` are joined verbatim, so N blank lines
  there feed N-1 newlines (single newlines are extras; doubles are
  `paragraph_break`).
