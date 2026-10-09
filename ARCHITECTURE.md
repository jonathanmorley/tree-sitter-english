# Architecture

This repo is a pipeline of narrow layers. Text flows one way —
each layer sees only its input — from segmentation to labels to
phrases:

```
text → (1) grammar+scanner → tree → (2) typed AST → words
     → (3) POS tagger → (word, tag) → (4) chunker → phrases
     → (5) dependency parser → heads+labels → (6) lint → findings
```

## 1. Segmentation: `grammar.js` + `bindings/rust/scanner.rs`

Owns block structure and nothing else: `source_file > paragraph >
sentence > clause`. Paragraphs split at blank lines, sentences at
`.?!`, clauses at coordinating conjunctions (`and/but/or…`),
subordinators (`because/that/which/lest…`), and `;`/`:`/`—` joins.
Clauses are flat runs of words — no subjects, verbs, or objects.

The hard part is the period: `Mr.` and `H.M.S.` must not end a
sentence. A Rust external scanner tracks the last word and applies
three rules in order: single-letter initial (`J. Smith`) → inside;
known abbreviation (16 curated items) → inside; lowercase/digit
ahead (`p. 42`) → inside; otherwise the sentence ends. Initialism
runs and decimals are absorbed wholesale by `dotted`/`number`
tokens (including `10:30`-style times). The grammar is held at zero
`generate` conflicts — a hard requirement, since ambiguity here
means nondeterministic trees.

Error contract: unknown characters produce ERROR nodes and parsing
continues. Only the prose error histogram measures quality;
markup, front/back matter, verse, and headings bucket separately
as transcription (see README's scope section).

## 2. Typed AST: `crates/english`

A borrowed, typed wrapper (`Document/Paragraph/Sentence/Clause/
Word`) over the raw tree so analysis code never matches on kind
strings. Adds incremental reparse (`Document::update`: prefix/
suffix `InputEdit` + `Tree::edit` before reparse — without it,
reuse reads stale ranges) and lossless `tokens()` iterators that
flatten parentheticals and keep subordinators and joiners.

## 3. POS tagging: `crates/english-pos` (+ `-train`)

Labels each word with one of 17 Universal POS tags. A greedy
left-to-right perceptron (u64 FNV-1a hashed features, dense
`[f32; 17]` rows in a trivially-hashed map, zero per-token alloc
beyond one lowercase pass — ~2.5M tok/s on the tag pass, 1.87 MB
integer-encoded weights incl. a 14,563-word tagdict, entrywise
median of fifteen perceptrons on shuffled train orders
(EWT-only, no oracle;
dev 93.63% / test 94.11% greedy; beam+rules 93.67% /
94.13%)),
followed by a width-2 joint re-decode of low-margin spans
(`BEAM_MARGIN_T` 2.0, cap 8: 20–22% of sentences, ~6% of tokens
rescored) and fourteen gated correction rules (lexicon-backed
`have-verb`/`to-prep`/`to-verb`, relativizer/complementizer shapes
like `that-rel`/`that-ccomp`, most recently participle repair
`pass-by` and `quite`-adverb `quite-adv` — each admitted with
EWT-majority and gate deltas, 14-for-27 with rejections recorded).
Wiring (`wire.rs`) splits
contractions UD-style (`don't` → `do` + `n't`), normalizes curly
quotes, and excludes hidden punctuation. `Model::tag_margins`
(best minus runner-up) flags uncertain tokens for review;
`TagCache` (sentence-text key) makes a one-word edit cost one
retag instead of a book. A `correction.rs` engine applies
gated rewrite rules as a post-pass; both morphology-only rules
tried so far measured net-negative and were rejected — the
standing lesson is that morphology without a lexicon cannot beat
NOUN base rates.

## 4. Chunking: `crates/english-chunk`

Groups the tag stream into flat, non-overlapping phrases —
`[the green fields] [sat] [in the sun]` — following the
CoNLL-2000 shared task ("shallow parsing"): mark phrase spans
without resolving what attaches to what. That unanswered question
is the point: attachment is full parsing and out of scope, while
flat chunks carry most of the practical value. Tag patterns are
translated from Penn Treebank tags to UD tags.

The algorithm is a priority-ordered greedy machine over
`(piece, Tag)` pairs: at each position take the first matching
shape (Punct, Subord, Conj, Particle, Interj, Noun, Verb, Prep,
Adverb, Adj, Other) and consume its maximal run. Each token is
consumed exactly once — O(n), no backtracking, no regex —
returning index-span `Chunk`s with a single allocation.

## The Tier-3 rule

Word classes and phrase structure never go in the grammar. The
first commit of this repo attempted subject/verb/object rules and
dropped them the same day: a context-free grammar must commit to
one reading, so non-SVO sentences (`The book that I read was
good`) became ERRORs. Everything since — POS as post-pass,
chunks as grouping, dependencies and lint findings as further
post-passes — is that lesson applied. `AGENTS.md` records
the full history and backlog.

## Worked example

`The cat sat on the mat.` → one paragraph, one sentence, one
clause → words → tags `DET NOUN VERB ADP DET NOUN` → chunks
`[The cat]/Noun [sat]/Verb [on the mat]/Prep`. Three phrases,
six tokens, one left-to-right pass each stage.

## Budgets (Moby-Dick scale)

| Measure | Result |
|---|---|
| Full parse | ~210 ms (body text; full file higher) |
| Tag pass | ~87 ms (~9 µs/sentence) |
| One-word-edit keystroke path | ~40 ms (reparse + retag) |
| Dep parse (beam4) | ~4 s/book — batch/save-pass, never keystroke |
| Peak memory | 58 MB |
| Tagger weights | 1.87 MB, zero-dependency pure Rust |
| Dep weights | 32 + 3.8 MB lazy Tier-1 pair (gitignored, never vendored) |

Transformers (~97-98% UPOS vs our 94.13) were measured against
these lines and evicted: 50–1000× slower, 7–220× larger, plus a
foreign runtime in a dependency-free core. They contribute
offline as oracle labelers, distilled into the small model —
never on the keystroke path. Realistic ceiling for this
architecture is ~95; that is the declared victory condition.

## Measurement discipline

Bars are set before work (Moby-Dick hand-tagged set, cross-genre
set), EWT-majority is never contradicted without oracle support,
weight files are md5-tracked, corpus tests are written
test-first, and rejections with measurements count as results.

## Open risks

Compounding greed across greedy stages — measured at the
tag→chunk seam and closed against: all 22 genre cascade
misses decode with confident margins (≥ 2.0, most ≥ 10; two more
tag misses absorbed losslessly), so no confidence signal the
chunker could read would reach a single miss. Past the chunker the
compounding is structural, not greedy: tagger misses destroy head
evidence (gold→pipeline UAS 83.9→77.8), untrainable-around from
below (twice confirmed) and unfixable-from-below at the tag→chunk
seam — the joint tag-parse program measured the addressable
remainder (+0.83 headroom) and stopped. `AGENTS.md` carries the
itemized backlog.
