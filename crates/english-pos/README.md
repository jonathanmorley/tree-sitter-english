# english-pos

Statistical part-of-speech tagging for English prose (17 Universal POS
tags), as a post-parse pass over segmented text — never grammar (word
classes in `grammar.js` were tried upstream and dropped; see Tier 3).

A greedy perceptron: `Model::tag` decodes surface tokens left to right
(lowercased internally; shape features read the raw forms). Features
are 64-bit FNV-1a hashes extracted with zero per-token allocation
(scratch buffer reused; weights ride dense `[f32; 17]` arrays), so
tagging runs ~450k tokens/sec. Weights live in `weights/upos.json`,
trained by `crates/english-pos-train` on UD English-EWT: dev 90.35%,
test 90.53%, 1.51 MB.

```rust
let model = english_pos::Model::from_json(include_str!("weights/upos.json"))?;
let tags = model.tag(&["Time", "flies", "like", "an", "arrow", "."]);
// [Noun, Verb, Adp, Det, Noun, Punct]
```

Wired: `english_pos::tag_sentence` parses with the `english` crate,
reads lossless [`english::Sentence::tokens`] (no dropped subordinators
or joiners), expands contractions into UD pieces (`don't` → `do` +
`n't`, curly `’` normalized), and tags. Hidden punctuation (commas,
sentence-final marks) has no grammar node and is excluded.

Demo: `cargo run -p english-pos --example tag -- <file>` prints
`word/TAG` per sentence via `tag_sentence`.

## Performance

From `cargo run --release -p english-pos --example bench --
<file>` on Moby-Dick (Gutenberg 2701, from `CHAPTER 1. Loomings.`
onward: 1.23 MB, 10,542 sentences, 225,138 pieces; medians of 5):

| Stage | Time |
|---|---|
| parse (tree-sitter) | 252 ms |
| pieces (wiring) | 93 ms |
| tag (perceptron) | 176 ms (~17 µs/sentence, ~450k tok/s) |

Interactive edits avoid the full pass: `Document::update` re-parses
incrementally (~18 ms) and `TagCache` retags only changed sentences
(one-word edit: 29 ms, 10,541 hits / 1 miss). Keystroke path ≈ 47 ms
on book-size input.
