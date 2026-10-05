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
- `parenthetical` (plain, no inner end) and `complete_parenthetical`
  (inner end-mark, self-terminated sentence) must stay disjoint by
  construction: plain dies where an end mark appears, complete dies
  without one. Overlap (optional inner end on plain, or plain as a
  sentence alternative) conflicts. Complete takes no trailing closers —
  they would eat its own `)`. One dot cannot serve two levels, so
  dot-inside-parens always consumes the mark inside.
- The typed AST (`crates/english`) covers only nodes the grammar really
  produces (flat clauses; no noun/verb phrases — SVO fields were tried
  upstream and dropped, see `grammar.js` Tier 3).
- Scanner lookahead after the token must use `advance(false)`:
  `advance(true)` past `mark_end` corrupts the token range (observed as
  a zero-length `subordinator`). Same reason the dot/apostrophe/hyphen
  branches advance-then-break: the lexer rewinds to the mark on
  success, so over-consumed tail chars are re-lexed, not lost.
- External-scanner refusal rewinds fully (letter-dot-letter precedent),
  but the *caller* still decides: `DashRefused` (dash consumed, no
  boundary) must `return false` so the internal token matches from the
  run start; `NoDash` (spaces only) must FALL THROUGH to word lexing,
  because words are external-only and unreachable after a `false`.
  Returning false at a word position, or falling through past a
  consumed dash, both silently corrupt the tree (observed as vanished
  words, a swallowed em-dash joining two clauses, and 50+ corpus
  failures). `advance(true)` excludes chars from the token range
  (leading-whitespace skipping); `advance(false)` includes them — a
  `NoDash` fall-through with false-skipped spaces yields
  space-prefixed word tokens.
- Corpus tests are TDD: add the failing expectation to
  `test/corpus/*.txt` first. Note the input model: lines strictly
  between the header and `---` are joined verbatim, so N blank lines
  there feed N-1 newlines (single newlines are extras; doubles are
  `paragraph_break`).
- Never enshrine a misparse: a test that pins a known-wrong reading
  (`30 sharp` as an elaborating clause for the time `10:30`) is a
  placeholder, not a pass. When a misreading is identified, write the
  test asserting the correct tree, watch it fail, and fix the grammar
  or scanner — however small the fix (`number` absorbing `10:30`).
  Residuals are things that still ERROR after a fix attempt, or that
  a recorded cost/benefit call defers with triggers — never readings
  known to be wrong. The `verify` no-predicate/margin lists are the
  standing source of candidate misreadings.
- After every `grammar.js` edit, check for duplicate rule keys
  (`grep -n '^    [a-z_]*: \$' grammar.js` must show each once): JS
  silently keeps the last, so a duplicated rule shadows the real one
  and `generate` happily builds the wrong grammar. This has bitten
  three times (`parenthetical`, `clause`, `_sentence_end`).
- Prose audit: `cargo run -p english --example audit -- [files...]`
  counts ERROR/MISSING nodes (with contexts and separate prose vs
  transcription histograms) and diffs `examples/` against `.parse.txt`
  snapshots. Moby-Dick (Gutenberg 2701, kept out of the repo in `/tmp`)
  is the scale corpus, measured from `CHAPTER 1. Loomings.` onward.
  Gutenberg `_`/`*` markup buckets as transcription: it is out of
  grammar scope (an encoding of typography, not prose; the README's
  error-recovery contract already covers it), so only the prose
  histogram measures grammar quality.
- Node iteration (`children()`/`child()`, cursor or index) never yields
  MISSING nodes for hidden-rule aux symbols (e.g.
  `_sentence_end_token1`); only `to_sexp()` shows them. The corpus
  harness compares `to_sexp` output for this reason — a named-only
  walker passes tests that hide recoveries.

## Queued (not started)

- Markup tolerance (later): whether Gutenberg `_`/`*` deserves grammar
  treatment (`extras`, visible nodes, or a documented input pre-pass)
  is deliberately undecided; the audit buckets it as transcription
  until then. Do not let the transcription count drive grammar design.

- Em-dash residuals: leading-dash dialogue, doubled `——` (redaction).

- Em-dash interruptions (~247× on Moby-Dick) and parentheticals
  (~256×): the remaining error budget after hyphens. TDD with corpus
  tests, same as the hyphen slice. DONE 2026-09-26/27, in slices:
  (a) interruptions (external `_interruption`, dash run + absorbed
  closers on blank/EOF, sentence alternative aliased to `em_dash`);
  (b) colon+dash handoff (`this:—` + blank, same token after an
  internal colon); (c) colon handoff (external `_colon_handoff`,
  aliased to `colon`; times like `10:30` refuse, unchanged);
  (d) parentheticals inside subordinate clauses; (e) `;`- and em-dash
  joins inside parentheticals (note: em-dash joins in subordinate
  interiors were tried and REVERTED — bare-word continuation after
  the join makes `but` strand alone; only delimited joins are safe
  inside non-sentence repeats);
  (f) `&` as conjunction where valid;
  (g) apostrophe-hyphen elisions (`sou'-wester`);
  (h) ASCII `--` dashes: internal `--+` → `em_dash`, interruption
  runs extended, `end_ahead` + hyphen-branch `dash_run_passed`
  (trailing closed-class degrades before `--`, mirroring unicode).
  Audit prose errors 35→9 (Moby), verify 15→4. Cross-book (Austen /
  Doyle / Stevenson prose): `--` support collapses Austen 450→4
  and Stevenson 336→37 verify-error sentences (Doyle steady at 16);
  remaining classes are verse/song lyrics, headings with verbs,
  navigation coordinates (`62o 17′ 20″`), epitaphs, speaker labels,
  illustration captions (bucketed as transcription with `[]^{}`),
  and front/back matter.
  Residuals deliberately left: leading-dash dialogue,
  complete-parenthetical interiors without joins (`(unasked too!)` needs paren-architecture rethink), `R&D`-style mid-clause
  `&`, em-dash joins in subordinate interiors (attach ambiguity).
  Revisit triggers (2026-09-27): never for aesthetics — only if
  constructs start *erroring*, or a scheduling domain or NER consumer
  needs entities. Times were closed under this bar the other way:
  `10:30` misread cleanly as elaboration, so per the no-misparse rule
  it got a real token (number absorbs `:MM(:SS)`; EWT keeps times
  whole, 284× NUM, so UD alignment holds with no wiring change).
  NOTE (resolved 2026-09-27): built the real 0.27.0 CLI from source
  (`cargo install tree-sitter-cli --version 0.27.0 --root /tmp/tscli`)
  and regenerated — zero `src/` delta, so the 0.26.11 output was
  already identical and the big `parser.c` diff is 100%
  feature-driven table renumbering (2 new externals + new rules), not
  version churn. Source-built CLI remains the fallback whenever the
  npx prebuilt breaks (here: it needed absent GLIBC_2.39).

- Statistical POS tagging as a post-parse pass (never grammar rules —
  Tier 3 showed why): DONE v1 (`crates/english-pos` + train binary,
  greedy perceptron on UD English-EWT (dev 91.84% / test 92.05%
  with in-domain oracle data, 1.98 MB weights). DONE wiring (`Sentence::tokens` + `split_contraction`

  - `tag_sentence`/`tag_document` in `english-pos/src/wire.rs`; hidden
    punctuation excluded by construction). DONE speed (2026-09-26):
    u64 FNV-1a features + dense `[f32; 17]` rows, zero per-token alloc
    (accuracy bit-identical: 22720/22717); tag 4288→~170 ms on Moby-Dick
    (~450k tok/s, ~17 µs/sentence). DONE incremental:
    `Document::update` (prefix/suffix `InputEdit` + `Tree::edit` before
    reparse — without it reuse reads stale ranges and silently drops
    shifted text) + `TagCache` (sentence-text key, pieces cached);
    one-word-edit keystroke path ≈ 47 ms on book-size input (17 ms
    reparse + 29 ms retag, 10541/1 hit/miss). Bench harness:
    `cargo run --release -p english-pos --example bench -- <file>`.
    DONE moby prose eval (2026-09-26): `tests/moby.rs` gains 19
    hand-tagged sentences from Moby-Dick ch.1-2 (~190 tokens, bar
    0.84; measured 0.855 with 27 misses: titlecase OOV both directions,
    preposition-chain collapse, -s/imperative verbs, this-DET
    cascades). Oracle calls documented in-file; next oracle-train data
    works from this eval.
    DONE verify (2026-09-26): `verify` example (parse + `tag_margins`
    coherence checks: error / no-predicate / joiner kinds, transcription
    bucketing, title/fragment excuses, lowest-margin review list).
    Needs `Sentence::has_error`/`Clause::has_error` (full-subtree walk,
    anonymous children included) and `Model::tag_margins` (best minus
    runner-up). On Moby-Dick it finds real tagger misses (3sg `-s`
    verbs → NOUN, imperatives → NOUN) and grammar gaps (fed to the
    em-dash/parenthetical backlog below). Exit 0 by design (analysis,
    not a gate).
    Hyperparam note (resolved 2026-09-26): adopted iters=20/min-count=1
    (dev 91.70%/test 91.79%, canonical intact), then joint oracle
    training (see train README: dev 91.84%/test 92.05%; a finetune
    variant scored 91.88%/91.99% but was an accidental two-stage, so
    the clean single-run joint protocol won). A third oracle batch
    (ch.36, ~170 tok) was measured and REJECTED 2026-09-27: EWT dev
    -51 / test -27 (Moby -5): forensics shows closed-class turbulence
    both ways (`that` SCONJ↔PRON ±50s, ADP→ADV +40, ADJ overfire)
    from shared-prior drift — small-data noise, not signal. Banked in
    /tmp; oracle at scale needs bigger batches or per-class targeting.
    Training hygiene (learned 2026-09-27): md5-tag weight files at
    every step, verify bytes (not echoed intent) after each
    train/restore, and rebuild test binaries after weights change
    (`include_str!` is compile-time). Retrains are deterministic
    (proven by identical md5s) — suspect inputs/paths first. The sweep peak,
    iters=15/min-count=1 (dev 92.09%/test 91.85%), was rejected: it flips
    canonical "flies" VERB→NOUN (forensics: plural -s/-ies + noun-noun
    t-1 memorization outvotes its single VERB observation in EWT; see
    `forensics` example). w+t-1 and suf+t-1 conjunctions hurt dev at both
    min-counts (sparse-conjunction overfit); neither was adopted. Plain
    (unaveraged) perceptron beat Collins averaging here (33% vs 88%
    pilot) — see train README. Training data stays out of the repo
    (`scripts/fetch-ud.sh` pins revisions: GUM/LinES are CC BY-NC-SA and
    cannot ship here, EWT is CC BY-SA). Multi-treebank concat rejected
    2026-09-26, and mechanical harmonization
    (`scripts/harmonize-ud.mjs`, rules M1–M6 + J3) recovers only ~0.4
    of ~1.9 points — remainder is domain divergence plus deferred items
    (participles, name parts). EWT-only stands; full record in
    `crates/english-pos-train/CANONICAL.md`).

- `Clause::words()` drops subordinators (separate accessor), which
  silently loses tokens for consumers. DONE: `tokens()` iterator
  (`Token`/`TokenKind` over all visible child kinds, parentheticals
  flattened) on `Clause` and `Sentence`; `words()` kept for backcompat.

- `package.json` is still upstream-minimal (no author/repository);
  expanding it to the full canonical template is a node-bindings
  decision, not yet taken.

- Push prep: DONE 2026-09-21 (GitHub remote `origin` created, 38-commit
  stack signed with SSH key, pushed to
  github.com/jonathanmorley/tree-sitter-english). Tangled `upstream`
  push still pending: push-only SSH remote configured, blocked on
  approving `knot.xenolandscapes.com` host keys into known_hosts.

- External-oracle sentence diff (NOT STARTED): use Punkt / spaCy
  `ssplit` offline as differential oracles, never as runtime deps
  (Python, heavy, non-incremental; would break the 37 ms reparse /
  58 MB budget in `README.md` and the dependency-free runtime).
  Goal: find sentence-boundary disagreements against this grammar.
  Steps: (1) small throwaway script under `scripts/` (not a crate
  dep) runs Punkt + spaCy over `examples/*.txt` and Moby-Dick from
  `CHAPTER 1. Loomings.` onward; (2) diff against
  `cargo run -p english --example audit`; (3) triage each delta as
  grammar miss / oracle miss / transcription (`_`/`*`); (4) grammar
  misses feed corpus TDD in `test/corpus/*.txt` per the input-model
  note above. Acceptance: script documented, disagreements listed
  with counts, at least the top prose class converted to corpus
  tests or a constrained scanner fix with zero `generate` conflicts.
  License: dev-time use only, nothing copied into the repo.

- PDTB subordinator audit (DONE 2026-09-27): candidate PDTB
  subordinators checked against Moby evidence + EWT tags. ADDED
  `lest` (7x Moby, pure subordinator, EWT-absent) and `supposing`
  (5x conditional, EWT-absent) with corpus tests; multiword `so that`/`as if` verified parsing via composition (no action).
  REJECTED with reasons: `provided`/`given`/`considering` (EWT
  VERB-dominant main verbs — adding would break those parses),
  `granted` (1 conditional vs main-verb use + EWT VERB),
  `albeit` (no Moby evidence), `regardless` (ADV) /
  `notwithstanding` (prepositional). Moby error counts unchanged
  (these parsed as flat words before — the win is structural:
  subordinate clauses now marked); EWT-safe by construction
  (absent from EWT) with suite green. `however`/`therefore` stay
  plain words.

- Abbreviation harvest (SLICE DONE 2026-09-27; Punkt port still
  open): Moby pattern `\b[A-Z][a-z]{1,4}\. [A-Z]` shows only covered
  abbrevs (Mr/Mrs/St/Dr/No) plus true sentence ends (`Ahab. A`,
  `Whale. I`, `sun. W`) — no unknown-abbrev gap there. Extended
  search adds: `rev` (Rev. Henry, 2x, EWT-absent) and `mt`
  (Mt. Hecla, 1x, title-pattern; EWT mt words don't constrain dots)
  → ADDED with corpus tests. Rejected: `etc` (2x, genuinely
  ambiguous mid-list vs sentence-final — statistical, not list),
  `Ex` (single odd `U.S. Ex. Ex.`), `albeit` (0x). List delta:
  15 → 17. Suite green; EWT-safe (both absent from EWT).
  Full Punkt-port harvest remains future work. Standing rule:
  no large auto-imported list — over-listing keeps real sentence
  breaks wrongly open and is worse than a miss.

- Greedy NP-chunker post-pass (DONE 2026-10-05): CoNLL-2000 chunking as
  pattern, not code import. New crate `crates/english-chunk`
  over `tag_sentence` output (`english-pos/src/wire.rs`), never new
  NP/VP rules in `grammar.js` (Tier 3 showed why). Linear greedy —
  bench on Moby-Dick (Gutenberg 2701, `/tmp`, from
  `CHAPTER 1. Loomings.`): 10,523 sent / 225,048 pieces → 141,152
  chunks in 6.1 ms release (0.6 µs/sent), ~13% of the 47 ms
  keystroke budget's full-book parse+tag; negligible per keystroke.
  Steps all done: (1) spec chunk tagset + `split_contraction`
  handling (`README.md`; contraction pieces chunk by their own tags);
  (2) implement + unit tests (`tests/basic.rs`: 7 tests incl.
  lone-DET/NUM nouns and a no-zero-width-span tiling invariant —
  the eval caught bare `NUM` (`voted 5 to 3`) emitting an empty Adj
  chunk, fixed to lone-NUM nouns; plus a dead standalone-Adj arm
  removed after the release build warned); (3) Moby spot-eval
  (`tests/moby.rs`: 10 sentences, bar 1.0 pin) + genre end-to-end
  (`tests/genre.rs`: 20 hand-chunked news/academic sentences, one
  canonical table feeding a rule-exact test and a model-tag cascade
  test). Measured: rule 20/20; end-to-end 9/20 sentences (0.450,
  bar 0.43), token chunk-kind 151/173 (0.873 ≈ tagger 0.884 —
  minimal cascade amplification); all 22 token misses trace to
  tagger misses (`that`→NOUN, 3sg `-s`→NOUN, `after`→SCONJ); a
  tag-exact-implies-chunk-exact assert pins zero
  chunker-introduced sentence errors. Grammar untouched. Follow-up
  stays open: `mwe.py` longest-match trie post-pass over
  `Sentence::tokens` for `in spite of`-class multiwords.

- Input-contract formalization (DONE 2026-09-27): README gained
  the Scope and error-recovery contract section (prose histogram is
  the quality measure; transcription bucket named exactly as the
  audit counts it). Also fixed a stale line claiming ellipses are
  unhandled (mid-sentence `ellipsis` nodes + terminal `...` exist
  with corpus tests).

- Second-genre eval set (DONE 2026-09-27): `tests/genre.rs` — 20
  hand-composed sentences (10 news-report, 10 academic-expository),
  hand-tagged UD-style with oracle discipline (EWT counts checked:
  `several`→ADJ 49:0, existential `is`→VERB 249:5; guideline
  overridden once by EWT-majority: `such`→ADJ 72:24; attributive
  `tenth`→ADJ despite EWT's single nominal). Original text (no
  license exposure: GUM/LinES cannot ship, EWT test would
  double-count). Measured 0.884 with 20 genuine misses (titlecase
  OOV, preposition chains, `-s`/imperative verbs, `that`-cascades),
  bar 0.86, no model change in this commit. Cross-genre gold table
  (GUM test splits) lives in train README alongside.

- NLTK Punkt trainer port, harvest-only offline (NOT STARTED):
  source `nltk/tokenize/punkt.py` (1880 lines; Kiss & Strunk 2006).
  Trainer passes: word-split keeping periods glued → type counts →
  Dunning log-likelihood reclassify (hardcoded `p2=0.99`, scalings
  for length/period-count/no-period penalty, `ABBREV=0.3`) →
  first-pass annotate → orthography bitmask table → pair pass
  (rare-abbrev backoff `<5`, sent-starter counts, initial/ordinal-
  only collocations) → finalize (`COLLOCATION=7.88`,
  `SENT_STARTER=30`). Reference English params: 155 abbrev types,
  36 collocations, 38 sent starters, ~20k ortho entries (~237KB,
  WSJ-trained — carries `sales`/WSJ names, proves retrain need).
  Runtime order per boundary: collocation veto → abbr+ortho adds
  break → abbr+starter adds break → initial/number+ortho-false
  removes break. New vs `bindings/rust/scanner.rs:22-32`: (a)
  unsupervised discovery ranking, (b) per-type ortho bitmasks
  (ours fires rule ③ on any lowercase-next), (c) sent-starter list
  for abbr-dot-that-ends-sentence, (d) initial/ordinal collocations.
  Scope: offline `scripts/` binary only (same pattern as
  `crates/english-pos-train`), outputs sorted candidate lists for
  human curation into the abbreviation-harvest item — never runtime
  tables (breaks 37ms/58MB budget). Faithful port ~600-900 lines;
  harvest-only subset (tokenize+counts+LL ranking) ~250 lines.
  Pitfalls: `_word_tokenize_fmt` keeps periods glued; numeric/initial
  regexes use `[^\W\d]` (Unicode letters count); `typ[:-1]` slicing
  is verbatim-don't-fix; overlap-dedup load-bearing; train on
  literary prose, not WSJ. License: NLTK Apache-2.0 — reimplement,
  don't copy regexes/wordlists verbatim. Acceptance: candidate
  lists with LL scores on Moby-Dick, top items dispositioned per
  harvest criteria, zero grammar/runtime changes in this item.

- Scanner micro-guards from NLTK tokenizers (NOT STARTED):
  (a) digit-guarded colon: `treebank.py` `([:,])([^\d])` and
  `toktok.py` `:(?!//)` are the exact shape for the `10:30` residual
  — apply same guard in `scan_colon_handoff`
  (`bindings/rust/scanner.rs`), TDD in `test/corpus/*.txt`;
  (b) dash coverage: `destructive.py` `[\u2012-\u2015]` vs scanner
  `is_dash` U+2013/2014 only — extend to U+2012/U+2015 or map them,
  corpus test each (DONE 2026-09-27: range extended + bar test);
  (c) MacIntyre contractions as `split_contraction` candidates —
  DONE 2026-09-27 for `gonna→gon+na` (EWT gon/VERB + na/PART; GUM
  88/104 confirms) and `cannot→can+not` (zero gold instances
  anywhere, but UD-convention-unambiguous with ultra-known parts —
  zero-risk); DEFERRED `wanna`/`gotta` (zero gold + `wan`/pale
  collision). Reject: global-munge `split()` pipelines (non-incremental,
  breaks `advance`/`mark_end` contract), TweetTokenizer monolith
  (URLs/handles/emoji belong in transcription; its own `redos`
  timeouts prove the budget miss), sonority syllabifier, TextTiling
  as detector (topic shifts ≠ paragraphs; quadratic + numpy deps —
  oracle at most).

- Tagger correction layer, Brill-style post-pass (ENGINE DONE
  2026-09-27, NO RULES SHIPPED): `english-pos/src/correction.rs`
  holds `Rule` + `apply_rules` (pre-pass snapshot semantics) with unit
  tests, `Model::tag_margins` feeds it, trainer `--correct` reports
  accuracy + fires with a first-N printer. Rule (1) (`s-verb`)
  measured and REJECTED: net-negative on EWT at every threshold
  (τ=2: dev ±0 / test −2 on ties; τ=8: dev −4 / test −2) with zero
  Moby fires — plural `-ies`/`-us` share the shape; gates are now
  double-bounded `0 < margin < threshold` (ties carry no signal).
  Rule (2) (`imperative-0`: pos-0 NOUN→VERB on morphology + complement
  next) measured and REJECTED 2026-09-27: 0% precision (EWT dev fires
  `Lifts`/`Dentist`/`someplace`, all gold NOUN/ADV; ∞-threshold flips
  ordinary nouns en masse). Morphology without a lexicon cannot beat
  NOUN base rates at pos-0; the per-form EWT-majority variant is a
  tagdict and stays rejected. Source
  `nltk/tag/brill.py:137-166` templates, `brill_trainer.py:93`
  Two rules target the recorded misses: (1) NOUN→VBZ where word
  matches `[a-z]+s$` (not `ss`/`-ness`), prev ∈ {PRON,NOUN,PROPN},
  next ∈ {DET,ADV,ADP,end} — fixes `wears/glitters→NOUN`;
  (2) pos-0 NOUN→VB where word is verb-base-form and next ∈
  {DET,ADJ,ADP,PRON} — fixes imperatives. Gate both on
  `tag_margins` < τ (`lib.rs:175-182`; precedent
  `sequential.py:648-660` `cutoff_prob`), so the 92% stays
  untouched. Companion OOV work: Titlecase×position shape
  conjunction + TnT-style cap-split suffix backoff
  (`tnt.py:202-206,501-665`, infrequent≤10, maxlen 10) + suffixes
  4-5 (`-tion/-ment`) with stem-length guard; prep-chains need
  right-tag context (only a post-pass sees it); `this`-cascades need
  2-wide re-decode of low-margin spans, never tagdict (locks the
  wrong tag early). Admit rules only with EWT-majority + oracle-set
  support (CANONICAL.md discipline; EWT-domain rules hurt Moby —
  same divergence that killed concat). Cost: predicate list, bytes
  not MB; features (`lib.rs:101-102`) and `TagCache` untouched.
  Explicitly rejected: Collins averaging (falsified 33% vs 88%),
  HMM Baum-Welch EM, tagdict behavior change (fast-path-only
  optional), more `w+t-1` conjunctions (already hurt dev),
  stemming-as-backoff (destroys the `-s` signal; `flies→fli`
  conflates the rejected direction). Reporting steal: TnT
  seen/OOV-split scores + first-N-errors printer
  (`tnt.py:1023-1119`) → extend `verify` columns with seen/OOV +
  margin. Acceptance: rules + τ recorded, Moby + EWT-dev deltas
  reported separately, bench budget held.

- NP-chunker implementation notes (extends the greedy-chunker item
  above; NOT STARTED): source `nltk/chunk/regexp.py` rule semantics
  only — ChunkRule→maximal-run wrapper starting from
  `{<DT|PRP$>?<JJ.*>*<NN.*>+}` translated to UD tags;
  StripRule→strip leading VBG/IN; SplitRule→split on DT/CC;
  MergeRule→optional `of`-PP attach. Implement as single-pass state
  machine over `tag_sentence` (piece, Tag) vecs, NOT regex-over
  -`<DT><NN>`-string (`regexp.py:211-219` ReDoS timeouts prove the
  cost; O(rules×n) with backtracking breaks keystroke budget).
  `split_contraction` pieces already carry tags — consume directly.
  Do not import PTB patterns verbatim. Later candidate (not this
  item): `mwe.py` longest-match trie post-pass over
  `Sentence::tokens` for `in spite of`-class multiwords — same
  never-grammar reason as Tier 3.

- Neural runtimes evicted, transformers as oracles only (DECIDED
  2026-09-27): Brill is not SOTA (transformers reach ~97-98% UPOS
  on EWT vs our 92.05) but is the best-fit correction layer under
  the budgets — bytes not MB, no weight invalidation, net-error
  admission. Measured budget gaps (Moby-Dick 10,542 sent / 225k
  pieces, CPU single-thread estimates): BERT-base fp32 ~3-10 min
  full-doc (~1000×), ~440MB weights (220×), ~1GB peak (15×),
  torch/ONNX ~100MB+ deps; DistilBERT ONNX int8 ~10-30s (~100×),
  ~60MB (30×), ~300MB (5×); TinyBERT int8 ~5-15s (~50×), ~14MB
  (7×), ~150MB (2.5×) — vs 176ms / 1.98MB JSON / 58MB / zero deps.
  Keystroke single-sentence is the least-bad line (a distilled
  forward at ~1-5ms could fit 47ms) but full parse/bench/verify/
  audit, cold start, cross-platform prebuilts (C/Swift/Go/Node/
  Python bindings), and float nondeterminism (vs md5-identical
  retrains) all fail. Even Tok2Vec-CNN (~10-20MB, ~20-50k tok/s)
  is ~10× slower/larger for ~1 point. Standing protocol: transformers
  generate labels offline, distilled into the 2MB greedy model via
  oracle-data joint training — never the keystroke path. Only
  architecturally-compatible spike if ever revisited: `tract`
  (pure-Rust ONNX, no C++) — transformer ops poorly covered and
  slower, so spike, not plan. ONNX itself: open protobuf op-graph
  format + runtime (train in torch, ship without it; int8 quant,
  graph fusion; `ort` Rust crate over C++ ~15MB).

- Accuracy roadmap 92→95, SOTA 97 out of scope (AGREED 2026-09-27):
  ceiling for a linear discrete-feature model is ~94.5-95.5; the last
  ~2 points need a context-sensitive encoder that breaks every budget
  (see item above). Declare victory at 95 with 2MB deterministic, not
  97 with 400MB. Ordered by ROI, all inside size/latency budgets:
  (1) silver distillation at scale — large per-class-targeted oracle
  batches (titlecase OOV, prep-chains, 3sg, imperatives) over
  book-domain text, ch.36 batch-size discipline (small batches drift
  shared priors); zero runtime change; expect +1-2;
  (2) char n-gram + cluster features — suffixes 4-5,
  Titlecase×position, cap-split backoff, Brown/word2vec-256 clusters
  as one feature (~1MB word→u8 map, hashing absorbs it, min-count
  prunes to ~2-4MB); expect +1-1.5;
  (3) lexicon backoffs (bytes) — verb-base-form list, name
  gazetteer, `-ness`/`-ous` vetoes, consulted only below margin τ
  (EWT-safe by construction, same argument as `lest`/`supposing`);
  expect +0.5, mostly Moby-side;
  (4) beam-2 re-decode of low-margin spans only (~2× on \<10% of
  sentences, keystroke stays ~30ms); expect +0.2-0.5, mainly
  `this`-cascades. Not to do: wider dense features without data
  (`w+t-1` overfit repeats), tagdict behavior change, averaging
  (falsified), morphology-without-lexicon rules (both Brill rules
  rejected with measurements). Each step: EWT dev/test + Moby,
  canonical-`flies` veto, eval-before-model-change, md5 hygiene.

- Architecture assessment (AGREED 2026-09-27): layering is sound —
  deterministic incremental segmentation → statistical labels →
  phrase grouping, each failure mode contained, Tier-3 rule holding
  across all backlog items; measurement discipline (budgets, bars
  before work, EWT-majority veto, md5 hygiene, TDD corpus) is why
  rejections read as progress. Three risks carried openly:
  (1) compounding greed — three greedy stages, no joint inference,
  no confidence (`tag_margins`) flowing into the chunker; error
  cascade unquantified end-to-end;
  (2) correction layer 0-for-2 — engine shipped, both morphology
  rules rejected, so lexicons + beam-2 + distillation scale-up are
  now the load-bearing unproven pieces;
  (3) chunker has no accuracy number — bespoke UD tagset means
  CoNLL scores aren't comparable; needs a hand-tagged chunk set
  with a bar (`moby.rs`/`genre.rs` discipline) before claiming the
  stack works. Biggest single probe: score chunks (not just tags)
  end-to-end on the genre eval.
