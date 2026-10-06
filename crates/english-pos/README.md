# english-pos

Statistical part-of-speech tagging for English prose (17 Universal POS
tags), as a post-parse pass over segmented text — never grammar (word
classes in `grammar.js` were tried upstream and dropped; see Tier 3).

A greedy perceptron: `Model::tag` decodes surface tokens left to right
(lowercased internally; shape features read the raw forms). Features
are 64-bit FNV-1a hashes extracted with zero per-token allocation
(scratch buffer reused; weights ride dense `[f32; 17]` arrays in a
trivial `u64`-keyed map — no SipHash re-hashing of pre-hashed ids), so
tagging runs ~2.1M tokens/sec on the tag pass (~550k end-to-end with
parse). Weights live in `weights/upos.json`,
trained by `crates/english-pos-train` on UD English-EWT plus
in-domain oracle data: dev 91.84%, test 92.05%, 1.76 MB
(whole-number weights serialize as integers).

```rust
let model = english_pos::Model::from_json(include_str!("weights/upos.json"))?;
let tags = model.tag(&["Time", "flies", "like", "an", "arrow", "."]);
// [Noun, Verb, Adp, Det, Noun, Punct]
```

Wired: `english_pos::tag_sentence` parses with the `english` crate,
reads lossless \[`english::Sentence::tokens`\] (no dropped subordinators
or joiners), expands contractions into UD pieces (`don't` → `do` +
`n't`, curly `’` normalized), and tags. Hidden punctuation (commas,
sentence-final marks) has no grammar node and is excluded.

Demo: `cargo run -p english-pos --example tag -- <file>` prints
`word/TAG` per sentence via `tag_sentence`.

## Performance

From `cargo run --release -p english-pos --example bench -- <file>` on Moby-Dick (Gutenberg 2701, from `CHAPTER 1. Loomings.`
onward: 1.21 MB, 9,973 sentences, 220,436 pieces; medians of 6):

| Stage | Time |
|---|---|
| parse (tree-sitter) | 213 ms |
| pieces (wiring) | 87 ms |
| tag (perceptron) | 104 ms (~10 µs/sentence, ~2.1M tok/s; ~550k end-to-end) |

Interactive edits avoid the full pass: `Document::update` re-parses
incrementally (~14 ms) and `TagCache` retags only changed sentences
(one-word edit: 26 ms, 9,972 hits / 1 miss). Keystroke path ≈ 40 ms
on book-size input.

## Verification

`cargo run -p english-pos --example verify -- [files...]` parses,
tags (`Model::tag_margins`), and reports incoherent sentences:
`error` (ERROR/MISSING nodes, transcription `_`/`*` bucketed
separately), `no-predicate` (verbless multi-word runs outside
fragments, titles, and headings), `joiner` (leading `;`/`:`), plus
the lowest-margin (most ambiguous) sentences for review. On Moby-Dick
it surfaces real tagger misses (`wears`/`glitters`→NOUN,
imperatives→NOUN) alongside grammar gaps (em-dash interruptions,
dialogue) — the former feed accuracy work, the latter the grammar
backlog. Exit status is always 0 (analysis tool, not a gate).
