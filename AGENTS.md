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
    0.84; measured 0.87 with 24 misses: titlecase OOV both directions,
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

- PDTB subordinator audit (NOT STARTED): audit `SUBORDINATORS` in
  `bindings/rust/scanner.rs` (currently 24 items) against the PDTB
  explicit-connective list. Goal: close coverage gaps (`lest`,
  `albeit`, etc.). Non-goal: importing PDTB hierarchy or typing
  elaboration vs contrast in-grammar (would reintroduce the LR
  conflicts Tier 2 removed; classification belongs in a post-pass
  over `Sentence::tokens`). Steps: (1) build candidate list with
  PDTB source cited; (2) check each against Moby-Dick audit +
  corpus; (3) add only words with prose evidence, one corpus test
  per word, TDD first. Acceptance: audit table (candidate / evidence
  / added-or-rejected-with-reason), full suite green, zero
  `generate` conflicts. `however`/`therefore` stay plain words.

- Abbreviation harvest (NOT STARTED): audit `ABBREVIATIONS` in
  `bindings/rust/scanner.rs` (currently 15 items) the same way.
  Goal: harvest candidates via Punkt + Moby-Dick audit misses
  (`p.`, `ch.`, `vol.` already partly covered; check `fig/no/st/jr`
  gaps). Non-goal: a large auto-imported list — over-listing keeps
  real sentence breaks wrongly open and is worse than a miss.
  Steps: hand-curate minimal additions with prose evidence, one
  corpus test each (`test/corpus/abbreviations.txt`). Acceptance:
  list delta with per-item justification, suite green.

- Greedy NP-chunker post-pass (NOT STARTED): CoNLL-2000 chunking as
  pattern, not code import. New crate (e.g. `crates/english-chunk`)
  over `tag_sentence` output (`english-pos/src/wire.rs`), never new
  NP/VP rules in `grammar.js` (Tier 3 showed why). Linear greedy
  only — must hold the keystroke budget (~47 ms on book-size input;
  bench via `cargo run --release -p english-pos --example bench`).
  Steps: (1) spec chunk tagset + `split_contraction` handling;
  (2) implement + unit tests; (3) Moby spot-eval mirroring
  `crates/english-pos/tests/moby.rs` (small hand-tagged set, bar
  recorded before work). Acceptance: chunker runs inside bench
  budget, eval recorded, grammar untouched.

- Input-contract formalization (NOT STARTED): write the README
  error-recovery / scope contract the audit already implements
  (prose histogram is the quality measure; `_`/`*` markup and
  front/back matter, epitaphs, speaker labels, stage directions are
  out of scope). CCG / Link Grammar stay as `garden_path` test
  inspiration only. Acceptance: one README paragraph + audit bucket
  names match it exactly.

- Second-genre eval set (NOT STARTED): Moby-Dick overfit guard.
  Add a small hand-tagged eval (~20 sentences, accuracy bar set
  before work) from a different genre (e.g. news or academic prose
  in `examples/` style) alongside `tests/moby.rs`. Training data
  rules unchanged (`scripts/fetch-ud.sh` pins; GUM/LinES eval-only,
  CC BY-NC-SA cannot ship). Acceptance: eval committed, bar
  recorded, no model change in the same commit.

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
  corpus test each; (c) MacIntyre contractions (`gonna→gon+na`,
  `cannot→can+not`, `destructive.py`) as `split_contraction`
  candidates only after UD-EWT verification (EWT may keep `gonna`
  whole). Reject: global-munge `split()` pipelines (non-incremental,
  breaks `advance`/`mark_end` contract), TweetTokenizer monolith
  (URLs/handles/emoji belong in transcription; its own `redos`
  timeouts prove the budget miss), sonority syllabifier, TextTiling
  as detector (topic shifts ≠ paragraphs; quadratic + numpy deps —
  oracle at most).

- Tagger correction layer, Brill-style post-pass (NOT STARTED):
  source `nltk/tag/brill.py:137-166` templates, `brill_trainer.py:93`
  admission (`max_rules`, `min_score=2` net-error-reduction).
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
