# english

Typed Rust AST for English prose, backed by tree-sitter.

The grammar only segments prose into paragraphs, sentences and flat
clauses; this crate wraps the concrete tree in borrowed, typed nodes
(`Document`/`Paragraph`/`Sentence`/`Clause`/`Word`) so analysis code
never matches on raw kind strings. There are deliberately no noun/verb
phrase types.

```rust
let doc = english::parse("Hello world.");
for sentence in doc.paragraphs()[0].sentences() {
    println!("{}", sentence.clauses()[0].text());
}
```

## Tests

- `tests/basic.rs`: API behavior, mirroring key corpus cases.
- `tests/corpus.rs`: runs all of `test/corpus/*.txt` like
  `tree-sitter test` does. The CLI only links C/C++ scanners, so once
  the external scanner is rewritten in Rust this is the corpus runner.

Run with `cargo test --workspace` from the repo root (bare
`cargo test` only covers the root package).
