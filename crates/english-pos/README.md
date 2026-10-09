# english-pos

Statistical part-of-speech tagging for English prose (17 Universal POS
tags), as a post-parse pass over segmented text — never grammar (word
classes in `grammar.js` were tried upstream and dropped; see Tier 3).

A greedy perceptron: `Model::tag` decodes surface tokens left to right
(lowercased internally; shape features read the raw forms). Features
are 64-bit FNV-1a hashes extracted with zero per-token allocation
(scratch buffer reused; weights ride dense `[f32; 17]` arrays in a
trivial `u64`-keyed map — no SipHash re-hashing of pre-hashed ids), so
tagging runs ~2.5M tokens/sec on the tag pass (~570k end-to-end with
parse). Weights live in `weights/upos.json`,
trained by `crates/english-pos-train` on UD English-EWT (EWT:
UD_English-EWT contributors, CC BY-SA 4.0,
<https://github.com/UniversalDependencies/UD_English-EWT>):
dev 93.69%, test 94.10% greedy (3.52 MB with the tagdict table,
whole-number weights serialize as integers). Weights are the
entrywise mean of fifteen perceptrons trained on deterministically
shuffled train orders (LCG seeds 1–15, `english-pos-train` example
`ensemble` — the committed recipe, md5-identical reruns),
pruned at |w|<0.34 (`scripts/prune-weights.py` — prune tolerance
shrinks with K: 0.67 holds at K=3 but costs −15/−14 at K=15,
where finer agreement granularity carries signal; dev/test move
−1/−8 greedy);
EWT-only, no oracle data. Production decodes
through a width-2 beam re-decode plus seventeen gated correction
rules (`correction.rs`: lexicon-backed, relativizer shapes,
participle repair — each admitted with EWT-majority and gate
deltas, 17-for-31 with rejections recorded): dev 93.77%,
test 94.16%. Tagger weights embed a 14,563-word tagdict (words
seen under one tag in training) for the inference fast path —
byte-identical decode is measured per weights/rules change,
never assumed (see the train README probe record).

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
| pieces (wiring) | 83 ms |
| tag (perceptron) | 104 ms (~10 µs/sentence, ~2.1M tok/s; ~550k end-to-end) |

Interactive edits avoid the full pass: `Document::update` re-parses
incrementally (~14 ms) and `TagCache` retags only changed sentences
(one-word edit: 24 ms, 9,972 hits / 1 miss). Keystroke path ≈ 39 ms
on book-size input.

Piece buffers reuse across sentences where ownership allows
(`append_sentence_pieces`), and the tree walk itself stages no
`Vec`s (`for_each_*` callbacks over the cursor, not collected
children) — see `english/src/lib.rs`.

## Verification

`cargo run -p english-pos --example verify -- [files...]` parses,
tags (`Model::tag_margins`), and reports incoherent sentences:
`error` (ERROR/MISSING nodes, transcription bucketed
separately per the audit convention), `no-predicate` (verbless multi-word runs outside
fragments, titles, and headings), `joiner` (leading `;`/`:`), plus
the lowest-margin (most ambiguous) sentences for review. On Moby-Dick
it surfaces real tagger misses (`wears`/`glitters`→NOUN,
imperatives→NOUN) alongside grammar gaps (em-dash interruptions,
dialogue) — the former feed accuracy work, the latter the grammar
backlog. Exit status is always 0 (analysis tool, not a gate).
