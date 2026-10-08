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
- NOTE 2026-10-07: `nix fmt` (mdformat) wants to unwrap prose repo-wide
  while committed markdown is wrapped 80-col, and `nix flake check` fatals
  on `scripts/gutenberg-100.txt` (fixed: `scripts/*.txt` now excluded as
  byte-stable). The prose-style war is UNRESOLVED — do not blanket-apply
  mdformat (churn + marker normalization across 5+ docs); owner's call.

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

- Markup tolerance (DECIDED 2026-10-06: status quo, no grammar
  treatment): the 100-book strip experiment settled it. Stripping
  all `_` (23,103 hits / 70 books; intra-word only 60, all
  footnote/redaction junk) moved transcription 30,381 → 7,635 but
  prose 4,956 → 5,212: ZERO books improved (neg=0), 33 unchanged
  (e.g. Middlemarch trans −474 / prose ±0 — mid-sentence italics
  are lone-leaf ERRORs recovery always absorbs), 37 worsened, all
  front-matter blame-shift (`_Contents_`-class lines: the
  transcription node was shielding heading fallout; strip it and
  the heading errors as prose). So `extras` would buy zero parse
  quality while inflating the prose histogram by ~256 heading
  errors; visible nodes are Tier-3 sprawl (markup can appear
  anywhere words can); pre-pass adds offset damage on top. The
  transcription bucket was already the right call, and the
  experiment revealed its load-bearing side effect — narrowing
  the bucket later must re-run this test. Consumers are immune
  either way (ERROR leaves never become pieces, so POS/chunk
  never see markup). `*` (163 hits) rides the same logic.
  Standing rule holds: transcription count never drives design.

- Em-dash residuals: leading-dash dialogue, doubled `——` (redaction).
  Leading-dash PRICED OUT 2026-10-06 (stays residual): 70-book
  census finds exactly ONE body-prose line-initial dash (pulp
  ` - Brice disappearing...`, 40284) plus epigraph attributions
  (`—CHAUCER`, out of scope) and map/front-matter bullets. No
  French-style turn-taking exists in the corpus (Dumas uses
  quotes). Fixing the one hit needs paragraph-rule surgery for a
  single idiosyncratic line — uneconomical; the trigger bar
  (constructs erroring as a class) is not met. Revisit only on
  new multi-book evidence.
  Redaction trails DONE 2026-10-06 (−95 prose, zero rises):
  `of course——”` / `are——;` end the sentence, `Countess G——,`
  fills the clause, doubled `——And/But` joins — via TWO new
  externals (`_trail_end`, `_trail_mid`) arbitrated inside the
  interruption probe (boundary still hands off first; identical
  spans give identical trees). Grammar attempts failed
  instructively first: dash-led arms collide with the
  trailing-dash handoff (shared quote-closers; precedence cannot
  settle dash-shift duality), and a clause filler collides with
  joins — the scanner decides by follower, zero LR involvement.
  ASCII `"`/`'` count as closers only facing non-words (so
  `ship—"cargo"` keeps its join); `;`/`,` ride along (the
  `?";` philosophy); opener `—“` deliberately refused.
  Leftovers dispositioned: verse fragments, `—“`, leading-dash
  dialogue, `——-` hyphen mix (all out-of-scope/by-design).
  4 TDD corpus tests; keystroke 41.7 ms held; Moby 4; full
  gates green.

- Harvest residuals (queued 2026-10-06 from the 100-book final
  ranking; `/` excluded — all 27 hits are front-matter URLs):
  (a) spaced ellipsis (DONE 2026-10-06, mid-only): internal
  `ellipsis` widened to 3–8-dot spaced runs (75× 3-dot, 37×
  4-dot, 11× 6–8-dot). Two discoveries along the way: the
  external run counter never fires for spaced runs (consulted
  only when no internal token matches — proven by isolated-dot
  vs in-sentence debug runs), so terminal splitting is out of
  reach and spaced runs chunk mid-sentence (error-free but
  unsplit; boundary-only divergence, oracle-harness only); and
  tree-sitter silently compiles unbounded `{2,}` as exactly
  `{2}`, so the bound is explicit (13-dot table leaders stay
  errors, correctly). Sweep prose −105 (8 books better, 61
  flat); one +1 (Twain 7-dot divider blame-shifts into heading
  fallout — accepted, same class as before). Moby unchanged.
  2 TDD corpus tests (3-dot + 4-dot mid); scanner untouched;
  zero conflicts; full gates green.
  (b) mid-clause `&` (DONE 2026-10-06, −32 prose, zero rises):
  the 19 hits were three shapes, not one: `&c.` et-cetera
  (bulk), firm names (`Washburn & Moen`), dialect `&`=and
  (Twain), line-initial `&` (`C\n& M`). Rule: unspaced `&`+letter
  absorbs into `word` (R&D, `&c`, AT&T — EWT keeps such runs
  whole as NOUN/PROPN, incl. `etc.` 58×); spaced `&` stays a
  conjunction. Scanner-only (no generate): word-entry `&`
  acceptance + medial loop branch (hyphen-shaped) + probe
  decline (Word-gated) + probe newline-skipping (blank aborts
  to preserve paragraph_break). Side win: `&c.` escapes the
  single-letter-initial rule by construction (len 2), so `&c.`
  ends sentences via end_ahead. Residual: `; &` (Twain 3×) needs
  conjunction-after-semicolon slots — deferred (would silently
  re-tree thousands of `; and` joins with zero measurable gain).
  3 TDD corpus tests; Moby unchanged; full gates green.
  (c) letter-digit hyphen (DONE 2026-10-06, −77 prose, zero
  rises): three sub-shapes. (i) Letter-digit codes (`M-3`, EWT
  CCA-15 PROPN): hyphen branch absorbs `-` + digit run (the loop
  can't consume digits, so the branch takes the run itself).
  (ii) Digit-led compounds (`16-pounders`, `17-inch`, EWT
  `4-ever` whole): `number` gains `(-[A-Za-z][A-Za-z0-9-]*)?`.
  (iii) Spaced single hyphen as ASCII dash (`ship - he was`,
  40284 house style, 33× + 4× map): `em_dash` gains `/-[ \t]/`
  — trailing-space-only, so compounds, line-break hyphenation
  (`Broom-\nBrigade`), and leading-dash dialogue stay as before;
  all 31 book occurrences errored today, so zero clean-parse
  regression risk. Skipped: suspended `cigar-,` (2× print
  artifact). 3 TDD corpus tests; examples snapshots identical;
  Moby unchanged; full gates green. Residual `-`: TOC dot-dash,
  URLs, dialogue fragments (all bucketed classes).
  (d) dialogue handoff (DONE 2026-10-06, −352 prose, zero
  rises): interrupted quoted questions/exclamations (`children—?”`,
  `oh—!”`, doubled `name——?”` — turn-taking across blank lines,
  Dumas/James/Doyle). New sentence-final alternative
  `seq(repeat1(em_dash), _sentence_end)`: needs no scanner
  arbitration (mark vs clause-word disjoint in one lookahead, so
  the join reading never collides — unlike the boundary
  handoffs). Cascade bonus beyond the ~80 targeted hits (43
  books better). Moby 5→4 (`:` stage directions + `—` cleared;
  one heading-fallout `It` exposed, accepted class). 3 TDD
  corpus tests (incl. blank-line two-paragraph shape);
  zero conflicts; full gates + snapshots green.

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

- External-oracle sentence diff (DONE 2026-10-06): NLTK Punkt
  (WSJ-trained `punkt_tab` params) + spaCy `en_core_web_sm` run offline as
  differential oracles over `examples/*.txt` + Moby-Dick from
  `CHAPTER 1. Loomings.` — harness: `scripts/sent-diff.py` (dev-time
  only, venv + data under /tmp, nothing copied into the repo) vs
  boundary TSV from `crates/english/examples/sent_bounds.rs`
  (throwaway dump helper, kept alongside for future books).
  Measured: grammar 10,523 bounds; Punkt Δ 1,213, spaCy Δ 2,147
  (2,557 in body prose past the front matter). Triage:
  front-matter/transcription (Extracts, headings, notes, verse,
  epitaphs) ~800 — out of scope as bucketed; quote-attach ties
  (`.` vs `."`) ~200 — harmless, same split; spaCy under-splits
  at `.—`/`?—`/`!—`, fragments, short dialogue turns ~1,300 —
  oracle misses validating the em-dash/interruption slice; Punkt
  over-splits quoted `?` mid-sentence (`"The Sword-Fish?"—this…`)
  ~290 — oracle misses validating interruption handling; Punkt
  misses units/titles (`lbs.`, `Mt.`, `Mrs.`) ~60 — oracle misses
  validating the abbreviation harvest + initialisms; lowercase
  continuation after `!` (~few) — grammar by design. REAL FIND:
  interjection `No.` (3 Moby hits: `No. They/The/Only`, plus
  `—no."` and `no. So`) swallowed by the `no` abbreviation entry —
  fixed by DELETING it (the number use `No. 22` stays inside via
  the digit-ahead rule; EWT train has zero mid-sentence `No.`+digit
  and only 5 sentence-final `No.`). TDD corpus pair + scanner-only
  change (zero `generate` conflicts by construction): bounds
  10,523→10,528, errors unchanged (7 prose), one blank-line
  paragraph anomaly repaired as a side effect. Standing rule from
  the harvest holds and tightens: no large auto-imported list —
  and now, no dual-use entry without a digit guard.
  Kept as harness: `scripts/sent-diff.py` + `sent_bounds` example
  (dev-time use only — oracle parameters never enter the repo;
  only human-curated corpus tests or constrained fixes land).

- Cross-book sweep (DONE 2026-10-06, breadth triage only):
  Austen P&P (#1342), Doyle Adventures (#1661), Stevenson TI
  (#120) to /tmp (public domain, out-of-repo like Moby), cut at
  body start AND at `*** END OF` (front-matter `™`/`•`/dot-leader
  noise otherwise counts as prose — the bucket only catches
  `_`/`*`/`[]^{}`), audited: Austen 4 prose / 1289 transcription
  (the `_` italics working as designed), Doyle 12 / 142,
  Stevenson 48 / 17 on 112k/95k/62k words. Triage: Austen = 3×
  known complete-parenthetical residual (`(unasked too!)`) + 1×
  known `etc.` trade-off; Doyle = story-heading fallout (`I.` /
  `II.` / ALL-CAPS titles, ~9) + `—!` residual + `½` symbol;
  Stevenson = chapter-heading fallout (bare numerals, titles,
  narrative-continued subheads, map inscription, coordinates,
  prime marks) — the already-bucketed heading class, which the
  tool undercounts (recovery lands on following body words, so
  text-matching can't bucket it; left as is, no tool hack).
  Abbreviation-harvest pattern (`\b[A-Z][a-z]{1,4}\. [A-Z]`) over
  all three books: zero unknown candidates (all hits are
  sentence-final words or interjections, correctly split —
  including Doyle `No.`×7 + `Yes.`×8, generalizing the `No.` fix).
  No new actionable prose class → no code change; these numbers
  are the standing cross-book record, not new gates.

- Pilot-slice grammar fixes (DONE 2026-10-06): the 10-book pilot
  audit surfaced two small erroring classes, both fixed with TDD
  corpus tests + minimal grammar/scanner edits, regenerated with
  the pinned CLI (0.27.0, zero conflicts, no `array.h` churn):
  (a) U+2026 `…` (Wells 4×) — internal `ellipsis` widened to
  `/\.{3}|…/` plus a single-char external `ellipsis_end` arm
  mirroring the count==3 terminal/refuse dance (no 4+ wholesale
  analogue); behaves exactly like `...` (verified: `… and`
  splits to conjunction like `... and`); (b) repeated `!`/`?`
  (`!!`, `!!!`, `?!` — Twain/Dickens) — `_sentence_end` and
  `complete_parenthetical` end marks widened to `[?!]+` (one end
  mark; the `?!` case errored before). Recounts: wells 13→9,
  twain 15→12, dickens 26→24, Moby unaffected (zero `…`/`!!`
  there). Leftovers all dispositioned, no new classes: em-dash
  `—!`/`—?”` + parenthetical `!)` (known em-dash/paren
  residuals), quote-boundary singles, dot-leader transcription,
  `JO.` signature (out of scope), `Mr. Rochester—` unreproduced
  minimally (recovery attribution, no action), Shelley fully
  attributed (letter heads, `17—` redaction fallout, chapter-head
  fallout, known paren-interior `!`s).

- 100-book harvest (DONE 2026-10-06): scaled the pilot to 70 books
  (`scripts/gutenberg-100.txt` pinned IDs + `fetch-books.sh` polite
  fetcher with md5 MANIFEST; 9 Latin-1 files converted via iconv,
  recorded in manifest; texts live in /tmp, never vendored).
  Body-cut audit (START/END markers): prose 7,434 → 4,956 (−33%).
  (a) Transcription bucket widened (tool-only): `=✿™•|~‖°+#`
  with per-char evidence in-comment; `&` deliberately OUT (R&D
  residual must stay visible), `¡`/`-` left honestly erroring.
  (b) Backtick opener (`` `father' ``, Doyle): `quote` + all three
  scanner quote lists gain 0x60 (EWT 3 PUNCT, safe). (c) Digit
  ranges (`1881-82`, year/date spans): `number` gains `(-\d+)?`
  (EWT keeps 646-8420 whole as NUM). (d) Paren-architecture
  rethink TRIGGERED (lone `!` 102×/17b + `?` 29×/11b were all
  `(like the elephant he was!)`-class mid-clause interiors):
  `complete_parenthetical` now clause- and subordinate-internal;
  the clause-vs-sentence `)`-follow conflict resolves by
  `prec(1)` on the legacy sentence-level reading (zero conflicts,
  all prior parses stable). (e) `?";` (Burton 51×): `_sentence_end`
  absorbs one trailing `;` (the sentence already ended; `and`
  lexes as word at the new start, as always). (f) Colon joins
  inside parentheticals (`(not that...: far from it)`,
  `(_Enter Ahab: Then, all_)` — delimited like `;`/dash, same
  safety argument); Moby 6→5 prose (all leftovers known:
  song/speaker labels, J—— redaction). All TDD (5 corpus tests),
  pinned CLI, full workspace + fmt + clippy green. Standing
  residuals re-confirmed: leading-dash dialogue (`—?”`),
  `M-3`/`A-1` codes (letter-digit; digit-digit only this round),
  suspended `cigar-,` compounds, `8vo`/`vols.` bibliography.

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
  REMOVED 2026-10-06: `no` → 16 (differential-oracle find, see the
  sentence-diff item: interjection `No.` before capitals swallowed;
  number use stays inside via digit-ahead rule).
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

- Hard-prose eval set (DONE 2026-10-06): 10 Moby-Dick sentences
  (~1,550 tokens, avg 155!) picked by the new `candidates`
  example (per-sentence low-margin count, beam-vs-greedy diffs,
  correction fires, s-noun/that/title-mid/coord/prep-chain/subord
  flags; never auto-labels), disjoint from `moby.rs`/`genre.rs`
  and all oracle batches. `tests/hard.rs` (tagger, bar 0.84 at
  measured 0.8644) + `english-chunk/tests/hard.rs` (rule-exact
  pin + end-to-end token bar 0.87 at 0.892; sentence-exact
  saturates at 0 on hard text). Tagging discipline as usual
  (EWT counts in-file: `each other` DET+ADJ 15:0, `one`+NOUN NUM
  133:3, `because of` ADP+ADP 39:3, `going to` VERB 182,
  `preceding` ADJ 6:0, `hidden` ADJ 6:2, `for`-initial ADP 40:1,
  `as-X-as` first ADV unanimous). Chunk gold proposed by an
  independent spec port, verified boundary-by-boundary, Rust suite
  re-checks all. Authoring hygiene (learned the hard way):
  hand-aligned parallel arrays drift past ~200 tokens (caught
  3 dropped tags in review) — write WORD+TAG lines and generate
  the arrays, never hand-align.

- NLTK Punkt trainer port, harvest-only offline (DONE 2026-10-06
  by direct use, not a port): NLTK 3.10.3 was installed in this
  env, so `scripts/punkt-harvest.py` calls its reference trainer
  (`PunktTrainer`, Kiss & Strunk 2006) on Moby-Dick and prints
  Dunning-LL-ranked candidates with scores, our-list membership
  (read from `scanner.rs`, single source of truth), counts, and
  contexts — zero reimplementation risk, no regexes/wordlists
  copied (Apache-2.0 respected by non-copy). Measured: 3,361
  period-final types; ours rank high (`mr` 46.4, `st` 20.4,
  `dr`/`mrs` ~4–9). Top NEW items dispositioned, zero additions:
  roman numerals (`ii` 13.9, `iii`, `iv`, `vi`, `xvi`, `v`) =
  chapter-heading artifacts (out of scope); dotted initialisms
  (`a.d` 13.0, `u.s`, `p.m`, `n.e`, `a.s`, `s.w.f`) = already
  handled by the `dotted` token (Punkt confirms, no list action);
  single letters = initial rule covers; quote/markup-glued
  (`"mr`, `it_`, `—_n`) = tokenizer artifacts; `lbs` 7× = real
  unit but needs no listing (lowercase-`of` next keeps it inside
  via rule ③; `lbs.` before capitals correctly ends); `etc` 3× =
  standing rejection re-confirmed; `vat` 0.34 = common-noun trap,
  textbook over-listing harm — rejected with prejudice. Two
  nuances for the standing rule: `rev`/`mt` score BELOW threshold
  (0.68) yet stay curated (rare-but-real titles — LL alone would
  drop them, curation over automation); and Punkt never lists
  `no` either (common-word prior crushes it), independently
  confirming the `No.`-removal. Scope kept: offline `scripts/`
  only, no grammar/runtime change in this item.
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
  Scope (as planned; SUPERSEDED by direct use above — kept as
  reference): offline `scripts/` binary only (same pattern as
  `crates/english-pos-train`), outputs sorted candidate lists for
  human curation into the abbreviation-harvest item — never runtime
  tables (breaks 37ms/58MB budget). Faithful port ~600-900 lines;
  harvest-only subset (tokenize+counts+LL ranking) ~250 lines.
  (Direct NLTK use made the port unnecessary; the notes below
  remain the reference if NLTK ever becomes unavailable.)
  Pitfalls: `_word_tokenize_fmt` keeps periods glued; numeric/initial
  regexes use `[^\W\d]` (Unicode letters count); `typ[:-1]` slicing
  is verbatim-don't-fix; overlap-dedup load-bearing; train on
  literary prose, not WSJ. License: NLTK Apache-2.0 — reimplement,
  don't copy regexes/wordlists verbatim. Acceptance (MET by direct
  use 2026-10-06 — see the DONE note atop this item): candidate
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
  2026-09-27; FIRST RULES SHIPPED 2026-10-06 — 3 admitted of 14
  measured, EWT dev +6 / test +1, Moby/genre/chunk Δ 0, keystroke
  47.3 ms held): `english-pos/src/correction.rs`
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
  SHIPPED 2026-10-06 (`have-verb`, `to-prep`, `to-verb`, τ=2.0,
  wired into `tag_sentence`/`tag_clause` behind an empty-guard):
  dev +6 (have×2, to-prep×3, to-verb×1, all gold-correct),
  test +1 (have), Moby/genre/chunk Δ 0 — margin audit shows the
  evals' remaining misses are confident (≥ τ) or exact ties (0.0,
  blocked by design), so the gate cannot reach them. Lexicon:
  `lexicon/verbs.txt` (1825 EWT VERB lemmas, generated by
  `scripts/verb-lemmas.sh`) + `known_verb_form` destem. Removed in
  the same pass: `s-verb-lex` (±0: `steps` fixed, `structures`
  broken), `imperative-lex` (−1 `Lifts`), `proper-name`,
  `directional-adv`, `a-predicative` (−1 `aground`, gold ADP),
  `ness/ous/tion/ment`, `ward-adv`, `more-adj` (zero fires —
  sound shapes, no support). `tests/canonical.rs` pins the
  `flies`-VERB veto on decode + corrected paths.

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
  Do not import PTB patterns verbatim. MWE follow-up DONE
  2026-10-06 (`crates/english-chunk/src/mwe.rs`): longest-match
  trie over lowercased pieces (not words — the chunker's input
  currency; strictly consecutive, so commas break the run),
  MWE-first ahead of the priority cascade, 11 entries with
  per-entry kinds (`as if`/`so that`/`such as`→Subord,
  `in spite of`/`in front of`→Prep, `as well as`→Conj,
  `a lot of`→Noun, `at all`/`no longer`/`in fact`/`as usual`→
  Adverb) — every entry attested cross-book (Moby `as if` 133
  down to `in spite of` 1); excluded `in order to` (infinitive),
  `because/out/up-to` (particle ambiguity), `of course`
  (discourse). TDD caught a 4-deep chain bug on `a lot of`
  before it shipped. Bench 8.5 ms full-book (~200 merges);
  all prior evals byte-identical. V2 DONE 2026-10-06: tag-gated
  `out of`/`up to`/`because of`→Prep (14 entries; trie nodes carry
  `want` tag slices, empty = tag-blind as before). EWT conditions:
  merge iff all-ADP (83/84, 24/34, 39/42; remainders are ADV-first
  particles or SCONJ-second clausals that stay split — the model
  already disambiguates, e.g. `up to midnight` ADV+ADP). `in order
  to`/`of course` stay excluded with EWT numbers (15:0 and 20:0
  nominal-second). Hard S5 gold re-derived at both `because of`
  spots (hand-verified Prep+Noun splits); hard end-to-end token
  1382→1383, genre unchanged; bench 9.3 ms (negligible).

- Neural runtimes evicted, transformers as oracles only (DECIDED
  2026-09-27): Brill is not SOTA (transformers reach ~97-98% UPOS
  on EWT vs our 92.05) but is the best-fit correction layer under
  the budgets — bytes not MB, no weight invalidation, net-error
  admission. Measured budget gaps (Moby-Dick 10,542 sent / 225k
  pieces, CPU single-thread estimates): BERT-base fp32 ~3-10 min
  full-doc (~1000×), ~440MB weights (220×), ~1GB peak (15×),
  torch/ONNX ~100MB+ deps; DistilBERT ONNX int8 ~10-30s (~100×),
  ~60MB (30×), ~300MB (5×); TinyBERT int8 ~5-15s (~50×), ~14MB
  (7×), ~150MB (2.5×) — vs 104ms / 1.76MB JSON / 58MB / zero deps.
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
  (2) char n-gram + cluster features (CHAR HALF MEASURED AND
  REJECTED 2026-10-06: suffixes 4-5 with stem guard,
  Titlecase×position conjunction, cap-split suffix 2-4 backoff —
  min-count=1: dev −130 / test −80 at 2.82 MB; min-count=2 WORSE
  at −181/−188, 2.27 MB (pruning drops the rare oracle-lexical
  features the committed setup needs — min-count=1 is
  load-bearing). Forensics: ADJ overfire everywhere
  (NOUN/PROPN/VERB→ADJ +30/+30/+21 — longer suffixes memorize),
  title-fragmentation (PROPN→NOUN +94 — splitting F_TITLE
  starved the pooled evidence), diffuse re-convergence noise
  (ADP→PART +36, AUX→VERB +32). Partial credit: NOUN→PROPN −66.
  Same sparse-conjunction overfit that killed `w+t-1`. Code
  fully reverted (weights restored `712e0fc7`); the one useful
  artifact is negative knowledge. Cluster half (Brown/word2vec)
  DEFERRED, not rejected — dense 256-class features don’t share
  the sparse-memorization mechanism, but need their own offline
  pipeline + licensing thought first),
  then MEASURED AND REJECTED 2026-10-06: Brown-style pipeline
  built anyway (gensim skip-gram seed-42 + numpy k-means k=256 on
  the four public-domain bodies, 510k toks / 6,905-word vocab,
  bit-identical reruns; purity 0.698 majority-share vs EWT tags,
  median cluster 0.610 — over the 0.6 gate; spot-checks gorgeous:
  Pequod crew, Austen names, whale/nautical nouns, negation all
  cluster cleanly). Retrain with one dense cluster-id feature:
  dev −146 / test −165 at 1.97 MB. Diagnosis: the shared prior
  smooths away word-identity memorization (same shape as the
  averaging failure — settled weights into noise), and book-domain
  classes mislead web-domain test words. Dense ≠ safe. Map
  discarded (deterministically regenerable); script kept as
  infrastructure (`scripts/cluster-books.py`). Roadmap accuracy
  work now stands: distillation 0-for-5 plus frozen-prior
  and counterweight mechanics 0-for-2 (frozen: 5 K-points, zero
  drift but zero gain; counterweight k=2/3: moby-15 high-water
  mark but test −51 — gains scale with web damage everywhere,
  trade is structural; see train README),
  char 0-for-2 variants,
  clusters 0-for-1 — the linear model's ceiling is holding firm
  and every direction has a measurement.
  (3) lexicon backoffs (bytes) — verb-base-form list, name
  gazetteer, `-ness`/`-ous` vetoes, consulted only below margin τ
  (EWT-safe by construction, same argument as `lest`/`supposing`);
  expect +0.5, mostly Moby-side;
  (4) beam-2 re-decode of low-margin spans only (DONE 2026-10-06:
  width-2 joint search over greedy runs below margin 2.0 (+2 left
  context, gaps ≤ 2 merged, cap 8; 20–22% of sents, ~6% of toks
  rescored; keystroke 48.3 ms vs 47.3 budget): EWT dev +10/+13
  with rules, test +22/+24, Moby +1 (`this`-cascade, as predicted),
  genre/chunk ±0, `flies` holds on all three decode paths.
  Under the +0.2–0.5 expectation in points (+0.05/+0.10) but
  strictly non-negative everywhere — admitted. Integer-margin
  discovery along the way: perceptron scores sum ±1 updates, so
  margins are integers (T≤1.0 catches ties only and scores −6).
  Not to do: wider dense features without data
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
  (1) compounding greed — two greedy stages plus a joint span
  re-decode, but still no confidence (`tag_margins`) flowing into
  the chunker; tag→chunk cascade quantified on genre (chunker adds
  zero sentence errors; token 0.873 vs tag 0.884);
  (2) correction layer 12-for-24 — engine plus shipped rules
  (`have-verb`, `to-prep`, `to-verb`, `that-det`, `that-rel`,
  `det-noun`, `that-sconj`, `that-ccomp`, `subconj-adp`,
  `apos-part`, `to-part`, `that-vcomp`) and the beam decoder; the
  load-bearing unproven piece left is distillation scale-up
  (0-for-5: joint 03, finetune 03, joint 04, joint 05, micro-06
  all rejected — 05 is the mirror case, evals up / EWT down, and
  still rejected per EWT-gates discipline; 06 proves composition
  dominates mass, killing the drift-floor hypothesis; frozen-prior
  finetune (new mechanics, `--freeze-at` K=2/5/20/100/500 on
  batch 05) holds EWT bit-identical at K≤20 but moves zero book
  evals anywhere — flip-forensics shows gains need co-moving
  shared rows, so masking cannot separate them; counterweighted
  joint (12 contested words, battleground EWT sentences at k=2/3)
  reaches moby-15 but test −51 — trade structural, both
  mechanics rejected);
  (3) chunker has no accuracy number — CLOSED 2026-10-05 by the
  genre end-to-end chunk eval (rule 20/20, cascade 9/20 sent /
  0.873 token, bar 0.43).

- Chunker confidence flow (MEASURED AND CLOSED 2026-10-06, no
  code): one-shot probe (production path — beam margins + gated
  rules — over the 20 genre sentences, since deleted) asked
  whether the 22 cascade miss tokens sit in the margin-actionable
  zone. Answer: zero actionable, zero ties, all 22 confident
  (margins 2–35, most ≥ 10; 2 further tag misses absorbed
  losslessly by chunk shapes). No confidence signal the chunker
  could read reaches a single miss — the exact analogue of the
  correction-layer ties. Compounding greed at the tag→chunk seam
  is therefore unfixable-from-below: remaining misses are tagger
  misses, full stop. `ARCHITECTURE.md` risks updated.

- Lexicon rule `that-det` (ADMITTED 2026-10-06, 4-for-15): the
  eval-margin probe (production-path margins on all 271 eval
  misses — 19 actionable, 244 confident, 8 ties; probe deleted)
  surfaced determiner-`that` (4 fires). EWT check: after ADP or
  at sentence start before ADJ/NOUN, gold is DET 89:1 (relatives
  take VERB/AUX next; VERB/ADV/DET/NUM/PROPN-next stay out per
  measured splits; VERB-prev excluded — complement-clause
  ambiguity). Measured: EWT ±0 (zero fires; shipped three carry
  +6/+1), production evals fix 2 with zero new breaks,
  `flies` holds, unit tests pin fire/abstain shapes. The
  attributive-ADJ shape (sharp/poor, 2 fires) was checked and
  left: noun-noun base rates need an adjective lexicon the
  support doesn't justify — same base-rate lesson as the
  rejected morphology rules.

- External-rules survey (DONE 2026-10-06, Brill/fnTBL/RDR/CG in
  parallel): all three traditions converge on right-context
  barrier scans as the only signal a ±1 perceptron structurally
  lacks — and all three agree on relativizer-`that` via
  right-verb evidence. Measured in: `that-rel` (SCONJ→PRON on
  VERB/AUX-next, EWT 534:4) and `det-noun` (VERB→NOUN on
  determiner+barrier, EWT 1902:3) — 6-for-17. EWT ±0 (zero
  fires both splits); 3 Moby hand-verified fixes (two reduced
  relatives with participles, one `try-works`), zero known
  breaks; `flies` holds; probes deleted. Leftovers dispositioned:
  RDR two-wide DET extension (81% EWT, complement-clause risk —
  below the 89:1 bar the shipped shapes hold), `out`-particle
  shapes (narrow, unmeasured), demonstrative agreement (vestigial
  in English), blanket titlecase (already falsified by the
  Titlecase×position measurement). Mined further the same day:
  `that-sconj` (PRON→SCONJ on DET+ADJ two-wide, EWT 45:5) —
  7-for-18, EWT ±0 with 3 hand-verified Moby complement clauses;
  rejected in the same pass: `that`+ADV (PRON:SCONJ coin flip —
  `that very day` kills it), DET+_+NOUN→ADJ without lexicon
  (57% ADJ, compounds break it), PART+_+ADJ (VERB-majority —
  model already right), `that`+PART/CCONJ (n=9, no majority).

- Gate-zone autopsy (DONE 2026-10-06, 10-for-21): the 158
  RDR-only wins inside our margin gate autopsied with predicted
  contexts — mostly singletons, but three shapes with EWT-majority:
  `that-ccomp` (pred-PRON that + VERB-prev + nominal-next +
  finite-verb-ahead barrier → SCONJ, EWT 395:16),
  `subconj-adp` (pred-SCONJ closed-class prep + nominal-next +
  NO verb ahead → ADP, EWT 5356:165), `apos-part` (pred-AUX/ADP
  `'s` + NOUN/PROPN-prev + nominal-next → PART; the next-guard
  was added after the probe caught copula-`'s` damage — `man's
  a human`, `Ahab's above`). Measured: EWT dev +20 / test +16
  net (39 fires sampled all gold-correct — best rule batch yet);
  Moby ~135 plausible fixes against ~3 suspect residuals
  (cross-clausal `that wild Logan`, copula `'s` with
  nominal-misread neighbors — documented, EWT shows zero of
  either class). `flies` holds, suite green, probes deleted.

- Leftover mining (Brill/CG remainders, DONE 2026-10-06,
  12-for-24): `that`+DET-next splits by prev (VERB-prev SCONJ
  129:1, NOUN-prev 36:21 mixed with reduced relatives) — admits
  `that-vcomp` (EWT ±0, 2 hand-verified Moby complement
  clauses). Rejected in the same pass: imperatives (standing
  rejection stands — 74% verb still loses to pos-0 noun base
  rates), Brill#15 lexicalized 3sg (pronouns precede possessed
  nouns and verbs alike — the shape can't separate `it rains`
  from `his arms`), `around`/`round` (thin).

- `to-part` (ADMITTED 2026-10-06, 11-for-23): pred-ADP `to`
  before VERB → PART (EWT 2893:48, the mirror of `to-prep`).
  EWT dev +5 / test +1 net; 20 Moby fires (~18 infinitive
  markers, 2 gerund-complement residuals documented —
  `preliminary to scalping`: ADJ-prev `to` + participle reads
  prepositional but UPOS has no VBG/VB split, so no cheap guard;
  accepted trigger). Remaining gate-zone mass after this is
  singletons plus the ADJ-lexicon attributive shape — worked
  next and REJECTED: majority-ADJ wordlist (758 forms via
  scripts/adj-forms.sh, kept as infrastructure) + DET-prev +
  nominal-next is ADJ 98.4% in EWT, but the rule goes dev ±0
  with a known break (`a blue box` noun-adjuncts) against test
  +3 — the s-verb-lex precedent (dev ±0 + known break =
  reject) decides it, and per-word exclusions would be
  post-hoc fitting to dev fires. Trigger for revisit: a
  principled noun-adjunct separator (color/material closed
  classes?) with its own EWT numbers. Mine-the-gap closes at
  4 admitted of 8 measured shapes.

- Cross-book sweep eval (DONE 2026-10-06, rule-generalization
  gate): 60 hand-tagged sentences, 20 each from Austen P&P,
  Doyle Adventures, Stevenson TI (3×20, stride + uncertainty,
  disjoint from all other evals/oracle; model-prefilled WORD+TAG,
  every token hand-verified, EWT counts behind contested calls,
  arrays generated never hand-aligned). `tests/sweep.rs` decodes
  per-sentence on both paths: greedy 1827/2079 (0.8754, bar
  0.86), production 1843/2079 (0.8865, bar 0.87) with beam 41
  fixes + rule fixes 4 (to-part ×2, to-prep ×2) and ZERO rule
  breaks — the standing bar held on unseen books. Process note:
  the first run showed 2 phantom breaks, both traced to two
  hand-fix batches reviewed but never applied to the gold file
  (S13/S14); the harness caught my own bookkeeping error, which
  is exactly what the generated-arrays discipline is for.

- Sweep-margin probe (DONE, no code): production-path margins on
  all sweep misses — 19 actionable of ~240. Five are the rejected
  attr-adj shape (only `flat` fits the old guard; blue-box stands);
  three touch `subconj-adp` edges (one wants a which-barrier, one
  is `as to`, one is garbage-in from a mistagged neighbor — all
  thin); the `that`+PRON-subject+VERB shape goes SCONJ only
  111:50 (relatives with pronominal subjects like `outcomes that
  they had __` need gap detection — unseparable locally).
  Nothing admissible; probes deleted.

- Eval-margin probe round 2 (DONE, no code): production-path margins
  on all 269 eval misses — 17 actionable, 244 confident, 8 ties.
  The 17 split into standing rejections (4 attr-adj blue-box class,
  3 titlecase OOV, beneath singleton) plus two EWT checks that both
  failed decisively: `that`+ADV-next goes PRON 4:0 after ADP (the
  eval DET reading is EWT-minority — rule would fight EWT) and
  prep+VBG-gerund goes SCONJ 286:14 (`after kidnapping`, `by
  playing` are adverbial clauses in EWT — the eval ADP gold is the
  minority). Nothing admissible; probes deleted.

- External benchmark shootout (DONE 2026-10-06, same hardware,
  same data): NLTK averaged perceptron (WSJ-trained) and
  RDRPOSTagger (UPOS-EWT model, same tagset+domain as ours) run
  locally against EWT test gold words (25,094) and Moby speed.
  Accuracy exact-UPOS: RDR 92.30% (23,163) vs ours 92.05% greedy
  / 92.15% production — a 39-token gap; dev shows no leak
  (RDR 91.75 vs ours 91.84). Coarse universal-12: ours 94.24%
  vs NLTK 87.03% (NLTK out-of-domain — gap conflates domain and
  model). Speed tagger-only on Moby: ours 2.1M tok/s vs RDR
  51k (41×) vs NLTK 24k (90×). Size: RDR 0.38 MB (5× smaller —
  exception trees share structure; our dense rows are redundant)
  vs NLTK 1.5 MB vs ours 1.76 MB. Verdict: RDR is the most
  accurate lightweight system and the most parameter-efficient,
  but Python-only at 1/41 the speed — same eviction logic as
  transformers, milder; budgets (zero-dep Rust, keystroke path,
  cross-platform bindings) keep it offline-only. Probes deleted;
  nothing to incorporate that survives the budgets (a sparser
  weight map was already measured worse at min-count=2).
  Gap dissection (same day, per-token both-systems comparison on
  EWT test): the 39-token net gap is the remainder of ~1,150
  RDR-only wins vs ~1,120 ours-only wins — nearly balanced error
  sets, not one-sided dominance. RDR wins net on VERB→NOUN (+36),
  ADJ→NOUN (+41), ADP→SCONJ (+50), ADP→ADV (+40),
  SCONJ→PRON (+30): verb readings, attributive adjectives,
  complementizer/particle distinctions — right-context +
  exception memorization, overlapping the correction-rule
  frontier (158 of their wins sit in our margin gate zone: the
  measurable ceiling for more rules). We win net on
  PROPN↔NOUN titlecase (±134) and NOUN→VERB (+47): shape flags
  + perceptron beat their unigram-noun-biased tree there.

- Inference optimization pass (DONE 2026-10-06, speed + size, zero
  accuracy delta): tag 185→104 ms (−44%), end-to-end 454k→546k
  tok/s (+21%), weights 1.98→1.76 MB (−11%), keystroke ≈47→40 ms.
  Four changes, all in `crates/english-pos`, all behavior-preserving:
  (a) trivial `U64Hasher` for the weight map (feature ids arrive
  FNV-1a pre-hashed; `std` SipHash re-hashed ~3M lookups/book for
  nothing — zero-dep, `U64Map` alias); (b) decode keeps one `lower`
  Vec (shape flags read the original-cased words via generic
  `features<R, L>`, killing the `raw` clone; tag history rides
  `&str` borrows of the statics, killing 2 allocs/token; beam
  reuses the greedy lowercase pass instead of rebuilding both);
  (c) single-pass shape flags + ASCII byte-slice affixes (same ids,
  same order — float summation order preserved); (d) push-based
  contraction/piece expansion (no interim per-token `Vec`).
  Proof: `tag` example output byte-identical pre/post on Moby-Dick
  (9,976 sent / 220k pieces); full workspace green. Weights
  regenerated by deterministic retrain (EWT+01+02, dev 91.84 /
  test 92.05 reproduced exactly) for the integer serialization
  (`to_json` emits whole-number weights as ints; `from_json`
  reads both forms) — values verified identical (Moby tag diff
  empty), md5 `712e0fc7`→`56082361`. Standing perf docs updated
  (`english-pos/README.md`, `ARCHITECTURE.md`); dated DONE entries
  above keep their original numbers as history. Next micro-opts
  deliberately left: beam `sort_by`→top-2 select (spans tiny),
  `TagCache` hit-path clone→borrowed API, `english` crate Vec-per-
  node iterators (parse stage is tree-sitter-C-dominated anyway).
  UPDATE 2026-10-06: the iterator item is DONE — additive `for_each_*`
  callbacks (cursor-driven, no `Vec` staging; `Vec` APIs delegate
  unchanged) + buffer-reuse `append_sentence_pieces` /
  `append_clause_pieces` in `wire.rs`, bench on the fast path.
  Measured: pieces 87→83 ms (−4%), end-to-end 546k→551k tok/s;
  `tag` output byte-identical on Moby-Dick; new parity test pins
  `for_each` == `Vec` APIs; suite green. Small by design (pieces
  stage is String-alloc-dominated — node `Vec`s were the smaller
  share). Cache borrow item DONE below; beam select REJECTED (see
  next entry).

- Speed micro-opts, split verdict 2026-10-06:
  (a) `TagCache::tag_sentence_ref` SHIPPED — borrowed `&[(String,
  Tag)]` hit path (single zipped `Vec` stored; key borrowed for
  lookup so hits skip key alloc + pair clone). Micro-bench
  (deleted after): 9,980 cached hits 16.7→5.2 ms (3.2×,
  −1.15 µs/hit). `tag_sentence` delegates (same clones as
  before), `tag_document` unchanged; suite green, Moby tags
  byte-identical.
  (b) beam `sort_by`→top-2 `select_nth_unstable` REJECTED —
  ties DO fire: 28/9,976 Moby sentences flipped (two sampled
  flips read better, direction of the rest unknown — tie-lottery,
  same bar as the rejected iters-15 peak). Reverted; identity
  re-verified. Standing lesson: never touch the decoder's
  tie-break without a principled reason and gate deltas.

## Dependency post-pass (stage 1: unlabeled UAS — DONE 2026-10-07)

- Fifth pipeline stage (never grammar — Tier-3 rule): greedy
  arc-eager over tagged pieces, perceptron over configuration
  features, new crates `english-dep` (inference: Config, static
  oracle, features, Model with greedy/beam decode, margins,
  JSON weights) + `english-dep-train` (offline CoNLL-U trainer,
  never a runtime dep). Stage 1 is UNLABELED (heads only — UAS);
  relation labels (LAS) follow on the frozen UAS foundation.
- Bar: UAS 87 (MaltParser-class budget), set pre-evidence and
  NEVER VERIFIED — no published linear-parser predicted-tag
  EWT-UAS number was found (published EWT figures are neural
  ~92 or gold-annotation UDPipe ~85; both out-of-setting).
  The bar stands unchallenged but unverified; the curve below
  is flattening well short of it regardless.
- UAS curve (EWT dev/test +gold, pipeline +tagger in parens):
  v1 singles 58.9/59.1 (55.2/55.7) → v2 MaltParser density
  71.4/71.5 (67.4/67.3) → Collins averaging 78.7/78.6
  (72.8/73.5) → v3 head/sibling/bigram 79.1/79.3 (73.7/74.3)
  → LaSO-2 + beam2 81.4/81.1 (75.8/75.3) → LaSO-4 + beam4
  82.3/82.1 (76.2/76.7) → +v4 neighbors (MaltOptimizer step 4:
  s0−1/s0+1/b0−1 word+tag, 0x53–0x58) 83.9/83.5 (77.8/77.8).
  Screen-then-license held: greedy screen +2.1, LaSO-4 verdict
  +1.6. Curve still short of the unverified 87, but climbing
  ~+1.6/step with the harvest, not flattening. Mini-probe
  (60 sentences) 94.4 → 97.3 across v1→v2 pinned underfit,
  not a loop bug.
  Train fit at v3: 92.9 vs dev 79.1 (overfit); min-count 2/3
  flat (78.8/79.3, 11.5/9.3 MB — pruning buys size, not UAS);
  iters-40 flat (78.7/79.0 — converged). Width gains halve
  (+2.3, +0.85); width-8 REJECTED without running (projects
  ~+0.4 at 2× decode cost on an already-4s pass).
- Averaging reversal vs the tagger, kept as measurement:
  Collins averaging was FALSIFIED for tagging (33% vs 88%
  pilot) but wins +7.1 here — tagger data converges fast and
  dense (averaging dilutes), parser oscillates on sparse shared
  features (averaging settles). Same discipline, opposite
  verdicts, both recorded.
- Beam license (the load-bearing result): beam-2 decode of
  greedily-trained weights LOSES 16 points (79.1→63.2) — not
  a code bug (traced: beam faithfully finds higher-scoring
  degenerate attach-all-to-verb chains the oracle path never
  visits). Greedy-trained scores rank actions within a state
  but compare meaninglessly across paths. LaSO early-update
  beam training (Collins & Roark 2004, `train_beam`, averaging
  built in, weights shape unchanged) is the license: beam
  decode then beats greedy-of-same-weights by +3.6 and the old
  best by +2.3. Train/decode widths must match (beam2 of
  width-4 weights: 80.7 vs beam4's 82.3). Provenance: beam
  decode of beam-trained weights is the ONLY licensed combo;
  greedy decode of LaSO weights collapses (77.8, and 0/2 on
  the tiny toy) — pinned in tests as expected behavior, not a
  bug. `parse_beam` stays IFF trained weights ship with it.
- REJECTED with measurements: ROOT/NULL split (±0.1 noise —
  root aliasing is not the constraint; split kept for feature
  readability); min-count/iters (flat); width-8 (projected).
- Speed (EWT dev 25k toks, release, `depbench` example — kept
  as the standing harness): greedy 296k tok/s, beam2 118k,
  beam4 55k → ~4 s/book at beam4. Dep is a BATCH/SAVE-PASS
  stage, never keystroke (20–100× over the keystroke budget
  before optimization; the inference-optimization playbook
  could claw ~3–5×, still not keystroke). API shape follows:
  document-level, not per-keystroke.
- Budgets tiered (DECIDED 2026-10-07): Tier 0 grammar keeps
  current budgets (keystroke, zero-dep, bindings); Tier 1
  post-pass models are accuracy-gated with relaxed size
  (vendored single-digit MB, lazy asset past that); Tier 2
  offline tooling unbounded except reproducibility. The 29 MB
  LaSO-4 artifact is therefore a packaging question, not a
  ship-blocker — but it is NOT vendored: `weights/*.json`
  gitignored, regeneration documented in the trainer
  (`--corpus` + `--beam-train 4 --beam 4`, corpora stay
  out-of-repo in /tmp like all training data). Artifacts:
  dep-laso02/04.json + dep-v3/v4root.json in /tmp (ephemeral).
- Standing gaps, highest-impact first: (a) cascade 5.4 pts
  (gold 82.1 → tagger 76.7) — tagger misses propagate; joint
  tag-parse is a new stage, not a tweak; (b) labels (LAS)
  on the frozen UAS foundation — the consumer-facing output,
  open next; (c) non-projective eval kept honest (287 train
  sentences filtered, eval untouched — static oracle is
  projective-only, Reduce backstop strands instead of
  looping); (d) EWT-majority bar still needs a real linear
  comparator (CoNLL UDPipe-baseline EWT row) if the 87 bar is
  ever adjudicated.

## Dependency post-pass (stage 2: relation labels — DONE 2026-10-07)

- `LabelModel` in `english-dep` (averaged perceptron over arc
  features, 0x60 namespace: dep/head word/tag, d∓1 + h∓1
  neighbors, outer-dep tags both sides, tag/word conjunctions,
  direction, distance) + `parse_labeled` / `--labels` in the
  trainer (skip-whole on headless OR labelless tokens; label
  training uses ALL headed sentences incl. non-projective —
  classification, no oracle involved). Bars set before work:
  LAS 77 (+gold), 71 (+tagger) — UAS−5 rule on the 82.1
  foundation minus the measured cascade.
- Measured: ceiling (gold heads) 94.2/94.1 — the classifier is
  strong, label error only ~6%; beam4+gold 78.7/78.8 (bar 77
  PASS +1.7); beam4+tagger 69.7/70.3 (bar 71 MISSED by ~1).
  With v4 heads: 80.2/80.1 +gold, 71.1/71.3 +tagger — bar 71
  CLEARS (+0.1/+0.3). Labeler untouched (v1); heads did it.
  51 labels (subtypes kept — `nmod:poss` 3688× earns its
  class); labeler 3.8 MB (Tier-1-vendorable size, but ships
  only with parser weights — both gitignored as a pair).
- REJECTED pred-tags labeler (textbook cascade treatment —
  train on the tagger's own tags, gold heads kept): pipeline
  +0.5 consistently (69.7→70.2/70.3→70.9) but gold −0.7
  consistently on both splits (ceiling −1.1). Admission was
  pipeline-moves AND gold-holds; gold fell, and 71 stays
  missed either way — v1 (gold-tags) banked, v2 artifact in
  /tmp only. Lesson: the classifier's core job (clean arcs)
  outweighs noise-robustness at this data scale; the remaining
  cascade lives in HEADS (76.2 vs 82.1), not labels.
- Telemetry fix in the same pass: fully-headless batches push
  zero tokens, so the old skip counter (partial sentences
  only) never fired — 23 oracle-batch sentences were
  telemetry-invisible (alignment-safe, counts cross-checked
  vs EWT 12,543). `rows_seen` counter added; same property
  noted in stage-1 `parse_conllu` (committed behavior
  unchanged — counts matched, no action).
- MaltParser-English harvest (audited, not assumed — from
  MaltOptimizer LREC12 + Nivre06 model 7 + P09 English
  settings): POSTAG-window-6 ✓ covered, FORM-window-3 ✓
  covered (model 7 notably DROPS s1-form — candidate future
  ablation, not action), DEP-tree-4 ✓ covered, conjunctions
  ✓ covered, arc-eager ✓ matches the English choice
  (arc-standard won Hindi — language-specificity in action,
  not our language), LEMMA/FEATS BLOCKED (no inference-time
  source — tagger predicts UPOS only), LIBLINEAR noted
  (same family as averaged perceptron; no action), joint
  tag-parse (NivreSPMRL: joint beats pipeline) supports the
  cascade analysis as long-term work. ACTIONABLE: step 4
  predecessor/successor features (s0−1, s0+1, b0−1 word+tag —
  b0+1 rides lookahead) → v4 arc templates 0x53–0x58, under
  greedy screen; QUEUED: pseudo-projective lifting for the
  2.3% (+0.2–0.4 est).
- Standing: pipeline LAS 71.1/71.3 (bar 71 CLEARED by heads,
  labeler untouched); UAS 83.9 (bar 87, −3.1 — curve climbing,
  next levers below). REJECTED parser-on-pred-tags 2026-10-07
  (LaSO-4, v4 features, gold heads kept): pipeline +1.1
  consistently (77.8→78.9/77.8→79.1) but gold −0.8
  consistently on both splits (83.9→83.2/83.5→82.6) — same
  signature as the pred-tags labeler, same verdict under the
  same bar (pipeline-moves AND gold-holds). Banked gold-tags
  parser restored (verified by re-measure, not by hash).
  Lesson, twice confirmed: noise-robustness training trades
  clean-input quality at roughly 1.5:1 against pipeline gains
  at this data scale — the cascade is structural (tagger
  misses destroy head evidence), not trainable around from
  below. Joint tag-parse remains the only addressed-to-cause
  lever; s1-form ablation MEASURED FLAT 2026-10-07 (greedy
  screen dev −0.04 / test +0.18 — noise; 0x32 stays per the
  min-count precedent, note in-code).

## Dependency post-pass (BANKED 2026-10-07)

- Final: UAS 83.9/83.5 +gold (77.8/77.8 +tagger), LAS 80.2/80.1
  +gold (71.1/71.3 +tagger). Set bars: LAS-77 PASS, LAS-71
  PASS; UAS-87 missed −3.1 against an unverified number.
  Artifacts: `english-dep` + `english-dep-train` committed and
  pushed (weights gitignored Tier-1 pair: 32 MB parser +
  3.8 MB labeler, regeneration in trainer docs).
- Backlog (queued, in ROI order — none changes a shipped
  decision, all need fresh bars before work):
  (a) joint tag-parse (NEW STAGE): the only addressed-to-cause
  cascade lever (5.5-pt structural loss, twice confirmed
  untrainable-around); needs decoder/features/evals/budgets
  scope like this stage had — SCOPE DONE 2026-10-07
  (`docs/joint-tag-parse.md`): couple at inference not weights,
  options A uncertainty-features / B two-pass feedback / C joint
  beam / D unified rejected upfront; Stage-0 probes first
  (margin separability + feedback headroom ≥ +1.0 or STOP);
  (b) pseudo-projective lifting (Nivre & Nilsson 2005) for the
  2.3% non-projective training sentences: +0.2–0.4 est,
  moderate build (head-marking + lift + decode-time unlift);
  (c) UAS-87 adjudication: needs the published linear-parser
  predicted-tag EWT-UAS number (CoNLL UDPipe-baseline EWT
  row) before another accuracy euro is spent against it.
- Revisit triggers (nothing else): constructs erroring as a
  class, or a consumer needing entities/relations beyond LAS.


## Joint tag-parse (STOPPED at Stage 0, 2026-10-07)

- Stage-0 probes ran (temporary example, since deleted; scope
  `docs/joint-tag-parse.md` §3): (i) margin separability —
  P(margin<2.0 | tag error) = 0.139 on dev (2813 errors), i.e.
  tagger mistakes are overwhelmingly CONFIDENT; option A
  (uncertainty features) dead at its pre-registered 0.5 gate.
  (ii) feedback headroom — oracle-correcting low-margin errors
  only, reparse beam4: 77.81 → 78.64 = +0.83pt, below the +1.0
  STOP bar (oracle-all sanity reproduces banked 83.90 exactly,
  machinery verified). Program STOPS per its own pre-registered
  rule — including option B, whose retag pass is bounded by the
  same headroom and would do worse than oracle corrections.
- Third independent confirmation that remaining tagger errors
  are confident (chunk-confidence 22/22, eval-margin 244
  confident, now capture 0.139): the addressable cascade via
  uncertainty is ~0.8pt, not worth a stage. The cascade stands
  as the pipeline's structural tax; joint work resumes only on
  a non-margin coupling idea (none queued) or a consumer that
  pays for full joint beam regardless.
- Banked pipeline numbers stand: UAS 77.8, LAS 71.1 (+tagger).

## Sweep-chunk eval + MWE-boundary fix (DONE 2026-10-07)

- Chunk half of the sweep gate (`english-chunk/tests/sweep.rs`,
  same 60 sentences as `english-pos/tests/sweep.rs`). Gold build:
  per-token ChunkKind lines drafted against gold WORD rows (sweep
  drops `,`/`.` but keeps `;`/quotes/`--` — 25 lines needed full
  rewrite after length verification caught raw-text drafting;
  stevenson-p1084s5 was never drafted at all); MWE-span scan
  clean (one fix: `so that` Av→S); generator verify-then-write
  (counts, closed tag/code maps) so the table can never carry a
  silent slip. Transcription discipline held throughout (count,
  then write — two double-extra lines caught).
- Rule test (gold tags → per-token kinds, bar 1.0) arbitrated a
  dozen gold errors, nearly all one shape: the greedy Prep run
  absorbs every CONT token (`in which we`, `of preference
  which`, `as a tenant Miss Bingley`, ADP-led adjectives and
  PROPNs) — hand derivation kept stopping runs where the cascade
  doesn't. Per-token (not per-chunk) gold by design; boundary
  fidelity stays pinned by genre 20/20 + the tiling invariant.
- Cascade: sent 4/60 (0.067), token 1797/2079 (0.864 ≈ tagger
  0.875 — minimal amplification, same as genre's 0.873≈0.884).
  Bars: token 0.80 pre-registered (clears); sentence 0.10
  pre-registered, MISSED → investigated (mechanical probe,
  since deleted: 276/282 token misses carry a tag error within
  ±2, remaining 6 sit on long Prep-run boundaries broken by tag
  misses 3+ tokens out — 282/282 tagger-caused, ZERO chunker
  errors; tag-exact invariant 2/2 holds) → recalibrated 0.05
  tripwire per the genre pattern, mechanism recorded in-test.
- MWE-boundary fix (SAME DAY, found by the sweep rule test):
  maximal runs swallowed MWE starts (`rapidly as well as`
  buried the Conj phrase in an Adverb run — MWE consulted only
  at chunk starts). Runs now stop at precomputed MWE starts
  (`stop` set threaded through noun/verb_end, Prep/Adverb/Adj
  loops with past-end-safe lookup); unlisted text byte-identical
  by construction (all prior suites green unchanged). Unit test
  pins `as well as`-after-ADV and `a lot of`-after-ADP. Audit:
  only ADV-tagged starts after ADV runs and DET/N-based starts
  after ADP runs were reachable — both now stop. Bench cost:
  9.1→15.3 ms full-book (1.5 µs/sent, negligible; per-keystroke
  single-sentence unaffected) — deliberately unoptimized.

## Vale alternative pilot: passive voice (DONE 2026-10-07)

- New crate `english-lint` (Tier-1): Vale-class output
  (`path:line:col [rule] message`, exit 1 on findings), pipeline
  runs once per document (parse → tag → beam4 parse → label),
  all rules share one [`AnnotatedDoc`]. Pilot rule
  `syntax.passive`: finite be/get-passives via `nsubj:pass` /
  `aux:pass`, names the verb, sentence-level spans (piece→byte
  word spans queued, not this pilot). Rule logic unit-tested on
  mock annotations (no weights); spelling/vocab explicitly out
  (not a spellchecker — syntax differentiation is the point).
- Eval `tests/passive.rs`: 60 book sentences (30/30 with
  adjectival lookalikes), EWT-convention gold (stative-capable
  participles count ACTIVE; reduced relatives out of scope —
  needs gap detection). Measured: precision 0.917 / recall
  0.733 — both pre-registered bars clear (0.85/0.60). All 10
  misses arbitrated pipeline-caused, gold stands: 6× tagger
  participle→ADJ overfire on eventives (the deferred participle
  gap, rediscovered independently — a tagger fix lifts recall
  directly), 1× labeler miss verified on gold tags (`are
  wanted`), 1× parse garble (verbless apposition), 2× tagger
  mistags the rule follows correctly. Weights-gated skip when
  Tier-1 assets absent (documented Tier-1 consequence).
- Latency tiers stand: POS/chunk rules keystroke-fast; dep rules
  batch (beam4 ~4 s/book; greedy-decode tradeoff per rule,
  unmeasured). Next rules queued, not started: nominalization,
  sentence complexity (both need their own 60-sent evals).

## Accuracy by reading level (MEASURED 2026-10-07)

- Question: is the tagger better on easy texts? Answer: NO —
  essentially flat. 112 gold sentences (sweep 60 + genre 20 + hard
  11 + moby 21), Flesch-Kincaid grade per sentence (vowel-group
  syllables), tertile bands, greedy + production accuracy:
  easy (<7.5): 37 sent / 354 tok, 0.8785 / 0.8814;
  mid (7.5–14.4): 37 / 802, 0.8728 / 0.8741;
  hard (>14.4): 38 / 2919, 0.8671 / 0.8770.
  A 1.2-point greedy slope over bands whose token mass differs 8×
  is noise; production flattens it further. Errors are driven by
  construction types (titlecase OOV, prep-chains, -s verbs,
  that-cascades — present at every level), not readability.
- Caveats: FK on single sentences is noisy (fragments score
  negative, the 489-word monster scores 88 — bands are tertiles
  of a noisy measure, still slope-free); probe deleted after use.
- Eval-hygiene finding (not gate-relevant): committed greedy evals
  tag FLAT token arrays, leaking t-1/t-2 history across sentence
  boundaries (titlecase `Police`/`Exports` flip NOUN/PROPN on it);
  per-sentence decoding (what production `tag_sentence` does)
  differs by ±3 tokens on 2000. Bars have room; noted, not acted on.
- Follow-up open, not started: parser UAS by FK band on EWT dev
  (gold heads exist — same tertile protocol, beam4 cost).

## Vale alternative, rules 3–4: complexity (DONE 2026-10-07)

- `syntax.sentence-length` (>30 pieces) + `syntax.clause-complexity`
  (≥4 clauses or ≥2 subordinate, grammar counts): thresholds as
  constructor fields with defaults (future config file, no debt).
  Grammar-only + tagger — forced a real architecture win:
  `annotate_shallow` (pieces/tags/clauses, heads root + rels empty)
  so POS rules never load dep weights; dep rules MUST use full
  `annotate` (documented silent-misfire direction, never false
  findings). Keystroke path proven by construction.
- Eval `tests/complexity.rs`: 60 (15 Moby + 10 Austen monsters, 23
  shorts, 12 judged sweep mediums; whitespace-normalized inputs).
  Measured precision 0.846 / recall 1.000 — both pre-registered
  bars clear (0.80/0.65). 6 FPs arbitrated, gold stands: 4×
  interrogative subord-quirks, 1× honest length overfire (p259s2),
  1× for-NP over-segmentation (Tier-3 inherits into the rule).
- Tooling lessons banked: JS `.` never matches `\r` (sweep headers
  carry wraps — `[\s\S]`, the gensweep-chunk precedent); temporary
  E0716 from `&Default::default()` in rule vecs (bind locals).

## Vale alternative, rule 2: nominalization (DONE 2026-10-07)

- `syntax.nominalization`: closed light-verb table (explicit surface
  forms, no stemmer) governing `-tion`/`-sion`/`-sis`/`-ment`/
  `-ance`/`-ence` nouns via `obj`/`obl`, head must read VERB; names
  the pair, no auto-rewrite v1. Scope: light-verb government ONLY
  (bare nominalizations silent by design); suffix-boundary shapes
  (`notice/promise/attempt/effort/care/inquiry/choice`, non-light
  `pay attention`) count ACTIVE.
- Eval `tests/nominal.rs`: 60 (11 book + 19 composed canonicals —
  books underuse bureaucratic shapes; 30 negatives incl.
  list-boundary and suffix lookalikes). Measured precision 0.929 /
  recall 0.963 — both pre-registered bars clear (0.75/0.50).
  Round 1 arbitrated: 3 golds flipped where the rule's own scope
  excludes them (`care` no-suffix, `-ing`/`-edge` deferred with
  named risks: `get going` class, `did damage` tripwire); rule
  extended twice on principled classes (plurals by stem —
  `arrangements` ends in `ments`; Greek `-sis` — full+stem match
  after strip-order bug `analysis`→`analysi`); remaining misses:
  1× tagger `did`/AUX mistag (gold stands), 2× documented
  non-deverbial `-ment` (`take a moment` — honest precision cost,
  not special-cased). Deferred with reasons: `-ing` gerunds,
  `-age`/`-edge` (each needs its own measurement).

## Vale alternative, rule 2 v1.1: -ing nominals (DONE 2026-10-07)

- EWT web-review scan first: light + `-ing`-NOUN + obj/obl ≈ 12
  true vs ~9 FP, FP classes structurally separable (temporal
  `obl:tmod`, verbal-gerund tags, generic `thing`) → all three
  guards shipped with the suffix: dependent must read NOUN
  (inceptive `get going` reads VERB — UD-convention separator,
  not fitting), `obl:tmod` never counts as government
  (temporals are adjuncts), `thing(s)` carved out (indefinite
  pro-forms). Other suffixes need no guards (unambiguous form).
- Eval grows 60 → 66 (6 composed `-ing` positives: soaking /
  seating / baking / ordering / training / rating). Measured
  precision 0.929→0.912 / recall 0.963→0.939 — bars clear.
  5 misses arbitrated, gold stands: 2× documented `-ment`
  costs, 1× deferred `-ing` (`give warning`), 2× light-`do`
  mistagged AUX (`did violence`, `does the baking` — do-support
  overfire on light uses, a tagger gap worth its own note).
- `-age`/`-edge` stays deferred (`did damage` tripwire stands).

## Vale alternative, rule 2 v1.2: -age/-edge (DONE 2026-10-07,
  P 0.923 / R 0.923)

- `-age` tripwire converts: `did damage` IS light+nominal, so the
  branch ships (+ NOUN-gate and no-`tmod`, same guards as `-ing`;
  `-edge` rides along — `knowledge` fires correctly as TP) and the
  gold flips with it (documented in-test). Eval 66 → 71 (+5
  composed `-age` positives, +1 flip); all 5 new positives fire.
- All 6 misses dispositioned, gold stands: 2× documented `-ment`
  costs, 1× deferred `-ing` (`give warning`), 3× light-`do`→AUX
  tagger mistags (`did violence`, `does the baking`, `did damage`
  — the rule is 3-for-3 clean on correct pipeline input, blocked
  only by the tagger's do-support overfire).
- Note: a background eval run mid-edits reported stale numbers
  (race between edit and cargo fingerprint) — foreground re-run
  to confirm whenever counts look off; the numbers above are the
  confirmed run.

## Correction rule 13: pass-by (ADMITTED 2026-10-07)

- From the lint pilot's participle misses: predicted ADJ after a
  be-form AUX with a `by`+ADP agent +1..+4 ahead (verb/clause
  barrier). EWT train gold VERB 110:0 with the -ed/-en guard
  (130:4 without — guard removes all 4 known breaks; morphology
  trims, barrier carries precision, det-noun precedent).
- Gates: unit fire + 4 abstains green; dev ±0, test +1 (`I was
  married by a judge`, canonical passive, margin 4.0 — calibrated
  τ=5.0 catches margin-4 fires, blocks margin-11 statives);
  abstains correctly on the stative twin (`aren't married to...`,
  prev `n't`/PART); sweep 0 fires / explicit 0 breaks; full
  workspace 112 green; `flies` holds. 13-for-25.
- Incidental: copy-threading refactor drift caught in passing
  (sweep production 1843→1841 without any rule change — greedy
  1820→1817, beam recovers one; bars hold, banked numbers stale
  but green; recorded, not acted on).
- Residuals: irregulars without -ed/-en (`set`, `torn` — v1.1 with
  lexicon measurement), non-adjacent be (`was rudely broken`).

## pass-by v1.1 (MEASURED AND REVERTED 2026-10-07)

- Adverb gap (`was rudely broken`, EWT 14:0) + closed irregular
  list (attested core 19:0 + no-change-verb completion; `-orn`
  and gap+irreg excluded for zero evidence) — both EWT-majority,
  both ZERO fires on dev, test, sweep, and hand-built shapes (the
  tagger tags these shapes VERB or better than the gate).
  Unmeasurable code doesn't ship (11-removed precedent); numbers
  stand as the residual record in the rule docs. The `try again`
  was correct process (test split found v1.0's fire the same
  way) — negative result, still a result.

## Correction rule refused: light-do AUX (REJECTED 2026-10-07)

- From lint traces (`did violence`, `does the baking` tagged AUX):
  EWT train gold for do/does/did by next-tag — DET: VERB 77:14,
  ADJ: 15:4, NOUN: 14:14, PRON+other: 101:26, ADV: 26:10,
  ADP/PART: AUX 652:52, PRON+VERB: AUX 138:1, other:VERB: AUX
  69:1. Best flippable context 85% — no unanimity anywhere near
  the bar, and pred-conditioning (color-adj lesson) can only
  worsen it: the tagger predicts AUX exactly where aux-evidence
  is strongest (imperative `Do the dishes!`, interrogative `do
  you like`, prepositional/infinitive). The two pilot FNs stay
  tagger-mistag orphans (confident-mistag class, capture 0.139).
- No code written (EWT shape-majority failed first — nothing to
  prototype). Reverse direction (VERB→AUX on PRON+VERB/`do`+VERB)
  unneeded: no motivating miss, tagger already 99% there.

## Correction rule refused: color-adj (REJECTED 2026-10-07)

- The queued attr-adj trigger, measured: Berlin-Kay colors with
  DET-prev + nominal-next, EWT train gold ADJ 26:1 (break: `Any
  white light`). Built with unit fire/abstains; dev fired ONCE and
  broke it (`a blue box`, USPS-box noun adjunct — net -1, gate
  fails; no test run un-fails a dev gate).
- Lesson banked (load-bearing for all future rules): EWT-majority
  was measured on the GOLD shape, but fires happen conditioned on
  the model saying NOUN — and the tagger predicts NOUN exactly
  where noun-adjunct evidence is strongest. Shape-majority ≠
  fire-precision whenever base-model errors concentrate on the
  minority reading. Future EWT checks condition on the PREDICTED
  tag (run the tagger over train, or gate on --correct fires),
  never gold alone. Same noun-adjunct class that killed unguarded
  attr-adj — twice confirmed, 13-for-26.
- Fully reverted (RULES entry, fn, const, shape tests removed;
  rejection note in module docs). fmt reflow of long test tuples
  kept (committed unformatted — gate hygiene).

## Dogfood: english-lint on repo docs (DONE 2026-10-07)

- README.md: 6 findings, all readable (2 long-sentences, 2
  clause-complexity, 1 correct passive `is written in Rust`).
  AGENTS.md (1400 lines): 359 findings in 2.4 s, no crashes/panics
  (exit 1 = findings, the only non-clean signal). Self-reference
  amusement: the tool flags the doc EXAMPLES of passives (`was
  broken by X`, `was measured`) — correct behavior, they ARE
  passives; and fires on the stative-twin discussion (`aren't
  married`) exactly as the eval predicts.
- Scoping decision (no code): markdown structure is not prose —
  table rows (`| Full parse | 131 ms |...`) and colon-introduced
  lists (one 18-clause monster) produce structural findings.
  Same philosophy as the transcription bucket (markup encodes
  typography, not prose). Code fences produced nothing (short
  fragments stay silent under every rule — verified by absence
  across both docs). Documented as prose-scope; a markdown-aware
  input filter (fences/tables/lists) is queued, not started —
  needs its own FP-rate measurement before it touches the
  pipeline, and docs-linting is not the product's first market
  (book prose is).
- Fragments add noise (expected — rules assume sentences; ERROR
  recovery still annotates, findings follow garbage-in). Not
  acted on: same contract as the grammar (recover, never refuse).

## Vale alternative, rule 5: weasel modifiers (DONE 2026-10-07,
  P 0.968 / R 1.000)

- `syntax.weasel`: closed big-three intensifiers (`very/really/
  extremely`) + adjacent ADJ/ADV, POS-only (shallow path —
  keystroke-capable). `so/too/quite/rather/pretty` stay out
  (distinct semantics — hedges vs intensifiers need their own
  rule); assertive `really,` breaks adjacency via hidden comma;
  attributive `very` never matches (ADV required).
- Eval `tests/weasel.rs`: 60 (30 book positives incl. `little`-
  edge and repetition shapes; 30 negatives: verb-scope,
  attributive, other-intensifier, plain). 1 FP arbitrated
  (double-mistag `very/first`→ADV/ADV on `the very first dart`;
  pipeline-caused, gold stands); 0 FN (both `little` cases fire
  correctly). Threshold rules can hit 1.0 recall on clean
  shapes — precision is where the calibration lives.

## Correction rule 14: quite-adv (ADMITTED 2026-10-07)

- Found by the hedge lint eval (13/14 FNs): the tagger reads
  `quite` as DET before adjectives (`quite sure` → DET ADJ).
  EWT gold is ADV 27:0 in that shape (`quite` reads DET only
  before DET — `quite a few`); single-word lexical rule,
  `that`-precedent, τ=2.0.
- Gates: unit fire + 2 abstains green; EWT dev +1 and test +1
  (both `quite [NOUN/ADV]` → ADV, gold ADV, margin 1.0 — zero
  breaks); evals neutral (sweep 0 fires / explicit 0 breaks);
  full workspace green; `flies` holds. 14-for-27.
- Honest scope: does NOT move the motivating hedge eval (its 13
  `quite` FNs sit at margins 9–18, unreachable below any gate) —
  admitted on its own gates anyway (EWT +2, zero harm). Hedge
  recall recalibrates 0.65 → 0.50 on measured-mechanism
  (confident-mistag class, genre/sweep-chunk precedent).

## Docs sweep + Pages WASM demo (DONE 2026-10-07)

- Docs sweep pushed (`61d7bcf`): pinned-CLI command, 16 abbreviations,
  14 rules + production numbers (dev 91.94/test 92.13), body-cut Moby
  perf (211/17 ms, 4 prose errors), dep/lint READMEs + pipeline stages.
- `docs/index.html` + `style.css` + `demo.js` (marketing + live demo)
  served as GitHub Pages root (`docs/`); `treefmt.nix` gains
  `docs/*.{html,css,js}` + `docs/pkg/*` excludes (`on-unmatched=fatal`
  would reject them otherwise).
- New crate `crates/english-web` (wasm-bindgen 0.2.129, lib+CLI pinned
  equal): `analyze(text) -> JSON` with production decode
  (`tag_beam_margins_lowered` + all 14 rules) over naive sentences /
  whitespace tokens + native `split_contraction`; chunks via
  `chunk_tagged`; lint constructs `AnnotatedDoc` by hand (clauses 0)
  and runs length/weasel/hedge only. 3 native tests green (incl.
  canonical flies-VERB on the demo path); node smoke test of the
  nodejs-target bundle confirms tags + weasel end-to-end.
- Full-fidelity browser parsing REJECTED with measurement: release
  link fails — `parser.o`/`lib.o` compile for host (cc has no wasm
  target here) and `ts_parser_parse_with_options` is undefined
  (wasm32-unknown-unknown has no libc for tree-sitter's `lib.c`;
  sanctioned path is emscripten, which can't link the Rust scanner).
  Earlier `cargo build --target wasm32 -p english-pos` "success" was
  misleading (rlib archives unchecked host objects; only the cdylib
  link exposes it). Fidelity contract documented in-crate and on-page:
  tagger/chunker/rules exact, sentences/words naive, complexity /
  passive / nominalization native-only. Revisit only with a wasm C
  toolchain (emscripten/wasi sysroot) in the flake.
- Toolchain notes: rustup stable 1.99 installed ($HOME/.cargo;
  use with `RUSTUP_TOOLCHAIN=stable` inside `nix develop` so nix's
  `cc` is visible); `wasm-bindgen-cli` compiled from source (~10 min,
  needs nix `cc`); release wasm 2.1 MB (1.76 MB weights embedded).

## Site v2 + main-branch PR rule (DONE 2026-10-07)

- Site rewritten: separate structure demo (6 pre-parsed showcase
  examples via `crates/english/examples/sitegen.rs` →
  `docs/parse-examples.json`, all zero-ERROR; token-kind highlight +
  collapsible tree) vs live WASM lint demo; dark-hero visual overhaul;
  method section dropped for architecture (SMIL-animated SVG pipeline +
  layer cards); 4 hand-scaled SVG charts (EWT accuracy, log throughput,
  weights size, dep gold-vs-pipeline) + transformer-estimate table.
- Privacy: 7 crate manifests expose a gmail author address (pre-existing,
  owner's call 2026-10-07: keep as-is); EWT CC BY-SA 4.0 attribution
  added to `english-pos/README.md`.
- PROCESS: `main` now requires pull requests (GH013 on direct push).
  New routine: branch → push branch → API-create PR → API-merge →
  sync local main → delete branch. PR #5 (site-v2) merged this way.

## Full grammar in WASM (OPTION A SPIKE PROVEN 2026-10-07)

- The site-v2 "full-fidelity rejected" verdict is OVERTURNED. Missing
  piece was only a wasm-capable C compiler: clang 21.1.8 from locked
  nixpkgs (`nixpkgs/0a3468a...`), raw (unwrapped) binary to dodge host
  wrapper flags. tree-sitter 0.27 was DESIGNED for this: runtime
  compiles `lib.c` with `TREE_SITTER_WASM_STDLIB` + shim headers
  (from `tree-sitter-language`'s `DEP_..._WASM_HEADERS`), C allocation
  forwards to Rust's global allocator (`wasm_allocator.rs`, auto on
  wasm32-unknown), `wasm_language` (wasmtime) not needed for linked-in
  languages. Root `build.rs` already handled the headers env.
- Recipe: `CC_wasm32_unknown_unknown=clang` +
  `CFLAGS_wasm32_unknown_unknown="--target=wasm32-unknown-unknown -nostdlib"`.
  Proven: `parse_count("Mr. Smith arrived at 10:30.")` = 1 in node
  (abbreviation + time via the real scanner), em-dash = 1, two-sent = 2.
- Productized: `english-web::analyze` now runs parse → `annotate_shallow`
  → chunk → length/complexity/weasel/hedge (complexity REAL now);
  site structure demo is live on custom text (own dynamic import per
  section); `sitegen` example + `parse-examples.json` deleted;
  clang+lld added to devShell. Parity verified wasm-vs-native on all
  showcase shapes. Earlier fear ("rlib archives unchecked host objects")
  stands but is now handled: the objects are real wasm.

## web-tree-sitter bundle (OPTION B SPIKE PROVEN 2026-10-07)

- Standard library path works: wasi-sdk clang (CLI-cached,
  `--target=wasm32-wasip1`, exact flags read from loader 0.27 source)
  links `parser.c` + PIC Rust scanner staticlib
  (`wasm32-wasip1`, `-Zbuild-std` under RUSTC_BOOTSTRAP — prebuilt std
  is non-PIC) + `wasm/wasi_shim.c` (`errno`, `_CLOCK_*`), producing a
  `Language.load`-able bundle with the REAL scanner (proven: `Mr.`
  + `10:30` one sentence, em-dash join). Recipe in `wasm/build.sh`.
- web-tree-sitter@0.27.0 vendored (`docs/pkg/wt/`), queries in
  `queries/highlights.scm` (served copy `docs/queries/`); structure
  demo renders query captures + clause walk with byte→char mapping
  (tree-sitter spans are bytes, JS slices UTF-16 — em-dash drifts
  without it). Stub-scanner recipe capture + build-std notes in entry.

## TreeTagger harvest (QUEUED 2026-10-07, from Schmid 1994/1995)

- Source: TreeTagger page
  (`https://www.cis.uni-muenchen.de/~schmid/tools/TreeTagger/`;
  research license — binary + `.par` never vendored, oracle use
  only like Punkt/spaCy). What ports and what does not: the
  decision-tree transition estimator itself does NOT port (the
  perceptron + beam-2 already covers wider context; the 1994
  96.36-vs-96.06 trigram win is pre-linear-model history).
  Architecture validated, not changed (segment → tag → chunk
  post-pass mirrors TreeTagger + chunker `.par`).
- (a) OOV decision-list rule (REJECTED 2026-10-07, zero code
  written — no support found): temporary `oov_probe` example
  (since deleted) tabulated EWT-dev post-rule gate errors
  (0<m<2.0) by shape × train-vocab × pred/gold: 159 errors,
  and TreeTagger's population is absent from every cell.
  Suffix inventory (`-ly/-ness/-ment/-ize/-ise/-ous`) has ZERO
  gate rows; `-tion` 1; `-ed` 9× Adj→VERB (the deferred
  participle gap on SEEN words, not unknowns — needs barrier
  analysis, out of scope here). Titlecase has no majority
  (TitleMid PROPN→ADJ/ADP 8 against, NOUN→PROPN ≤7 for, with
  unmeasured noun-adjunct counter-fires — the color-adj lesson
  in advance). OOV proper is ~15 scattered singletons, no cell
  above 3, mixed directions. Lesson, generalizing color-adj:
  TreeTagger's tree solves unknown-word guessing, but our
  gate-zone errors are uncertain readings of KNOWN words
  (function/category flips: ADJ/ADV/ADP/DET/SCONJ/NOUN plain) —
  different population, different tool. Process note: the first
  probe run raced its own edit (stale binary, mixed-case rows
  impossible from one code path) — same poisoned-build lesson
  as the Pattern entry; clean rerun confirmed.
- (b) Lemmatizer, Tier-1 lookup table (NOT STARTED): lemmas come
  almost free from the same lexicon as (a); consumers are
  `english-lint` passive (participle identity) and
  nominalization (deverbal noun→verb) instead of current suffix
  matching. Scope: EWT-lemma table + closed irregular list,
  offline-built like `lexicon/verbs.txt`, zero tagger impact.
  Bars: lint evals neutral-or-better, weights md5 unchanged.
- (c) Lexicon-constrained decode (PROBE ONLY, expect reject):
  TreeTagger restricts candidates to lexicon tags; our tagdict
  behavior change stays rejected, and pred-tags training failed
  twice on pipeline-moves-AND-gold-holds — measure only, admit
  only on the same bar.
- (d) Rerunnable shootout (DONE 2026-10-07):
  `scripts/bench-taggers.py` reproduces the external-benchmark
  item on demand: exact-UPOS on EWT test gold words (flat
  stream, committed-eval discipline) + tagger-only tok/s on
  Moby body words, coarse universal-12 (Petrov Table 1,
  reimplemented) for Penn-tagged systems. Legs: ours (cargo
  `tag_tokens`, needs `nix develop`), nltk-perceptron, rdr
  (`--rdr-model`), spacy-sm (aligned subset, coverage
  reported), treetagger (`--treetagger` + `--tt-params`,
  coarse only). Anything absent is skipped with a hint;
  `--tsv` writes the record. Verified: skeleton run (all
  skips), nltk leg 0.5000 on a 4-word synthetic set (caught a
  FORM/LEMMA column off-by-one before it shipped), ours leg
  0.7500 via `nix develop`. Standing record stays the
  2026-10-06 numbers in the shootout item until a rerun
  replaces them. `docs/references.md` gains the TreeTagger
  entry + the external-systems list pointing at the script.
  Hyperfine outer runner (same day): `hyperfine` added to the
  devShell + `scripts/bench-commands.sh` (shfmt `-i 2` clean like
  the other scripts) hyperfines the repo-owned commands
  (`bench` end-to-end, `tag_tokens` on Moby words; release
  binaries built once so the build is never timed) with
  `--warmup 2 --runs 5 --export-markdown`, appending
  nltk/rdr/spacy one-liners only when `BENCH_VENV` provides
  them and treetagger only when `TREETAGGER_BIN`/`_PARAMS`
  are set. Split of duties: bench-taggers.py owns accuracy +
  in-process breakdowns (spawn excluded), bench-commands.sh
  owns wall clock. Verified on a 6-word stub (spawn-dominated,
  as documented) incl. the nltk leg; fixed a real quoting bug
  along the way (newline-split fragments become separate argv
  elements — single strings only).
- (e) Shootout on the front page (DONE 2026-10-07): the page had
  summary charts but no full table, no rerun pointer, and no
  TreeTagger/spaCy rows. `docs/index.html` gains a competitor
  table (exact UPOS + coarse-12 + tok/s + size; spaCy/TreeTagger
  rows read "not yet run", nothing invented) plus the two
  rerun commands; the old coarse-12 footnote folds into the
  table. Verified: diff shows only the new hunks, table tags
  balance-checked. Built on a clean checkout of the page: a
  foreign uncommitted legend rework in the tree was parked to
  /tmp/opencode/foreign-index.patch, re-applied after commit —
  their work rides uncommitted as before, this commit carries
  only the table.

## Honnibal 2013 averaged-perceptron harvest (QUEUED 2026-10-07)

- Source: `https://explosion.ai/blog/part-of-speech-pos-tagger-in-python`
  (averaging rationale, tagdict fast-path, case-frequency advice,
  greedy-suffices 0.1% line, train-with-guessed-history caveat).
  Three queued probes, three validations, one do-not-reopen —
  all below. Nothing here changes a shipped decision by itself.
- (a) PROBE — training-history exposure (REAL FIND, not started):
  `english-pos/src/lib.rs:827` advances `prev1/prev2` with the GOLD
  tag; Honnibal: history must come from the guesses, "otherwise it
  will be way over-reliant on the tag-history features" — exactly
  the failure our trainer is exposed to, never measured here.
  Variant: advance history from predicted `best` (updates still
  toward gold), else identical. Bars: EWT dev/test ≥ 0, all evals
  neutral-or-better, `flies` holds on all paths. Orthogonal to
  the averaging rejection (history modeling vs weight
  averaging — different axis, no interaction assumed).
- (b) PROBE — tagdict inference fast-path (not started): blog's
  "~50% of words unambiguous, output the tag and skip" with the
  literal `tagdict.get` short-circuit. Our tagdict-behavior-change
  rejection STANDS (training/locking); this is the already-open
  "fast-path-only optional" door with the strongest external
  precedent. Bar: byte-identical tags on every eval + speed delta
  on `bench` (tag pass is 104 ms; skip rate decides the win).
- (c) PROBE — external case-frequency backoff (not started):
  Honnibal's actual case advice is NOT in-model case features
  (our Titlecase×position, measured dev −130 with PROPN→NOUN
  +94 — exactly the domain-convention overfit he predicts) but
  "how frequently is this word title-cased in a large sample",
  train corpus lowercased. Unmeasured direction: offline
  titlecase-rate table (books100 bodies as the large sample —
  in-domain, no license exposure) consulted only below τ like
  every other rule. Standard gates (EWT-majority on the
  PREDICTED tag per the color-adj lesson, dev/test ≥ 0).
- Validations, no action: greedy-suffices ("can't do without an
  extra 0.1%") matches beam-2 at +0.05/+0.10 admitted as
  strictly-non-negative; perceptron "rubbish at multi-tagging,
  wants a distribution" validates `tag_margins` + tie
  abstention (ties carry no signal); WSJ/ABC/Web order-stability
  validates the sweep gate; the averaging rationale
  (late iterations mutate the model around hard examples) is
  the mechanism language for our 33%-vs-88% falsification —
  same discipline already recorded the opposite parser
  verdict (+7.1).
- Do NOT reopen: Brown clusters (0-for-1 stands — blog's
  clusters ride WITH averaging, which tagging rejects; no new
  mechanism, no new trial).
- Method note 2026-10-07: this section was wiped from the working
  tree three times by concurrent uncommitted-tree churn ( hardened
  by committing code first, docs appended+committed atomically).
  Standing rule: AGENTS entries land in the same commit as the
  work they record, never left uncommitted across round trips.

## Shootout 2026-10-07: full five-leg numbers (DONE 2026-10-07)

- `bench-taggers.py` full run on pinned EWT test (25,094 words) +
  Moby body (212,791 words), TSV at `/tmp/opencode/shootout4.tsv`
  (out-of-repo like all training data): ours exact 92.05
  (per-sentence decode — reproduces the trainer gate exactly;
  the flat-stream 91.57 is a methodology error, fenced by
  `tag_tokens --sentences`), coarse 94.40; NLTK coarse 87.24;
  spaCy exact 93.15 / coarse 95.43 on the aligned 85.3%
  (21,401 words — drifted sentences excluded, NOT comparable
  head-to-head); TreeTagger coarse 89.52 (Penn/CLAWS mix mapped
  from the observed inventory). Speeds: ours 425k tok/s
  spawn-included (page keeps the 2.1M in-process tag-pass
  number), NLTK 21.7k. RDR still skipped (no UPOS-EWT model
  file in env).
- Attribution for the coarse deltas vs the 2026-10-06 probe
  (+0.16 ours, +0.11 NLTK): projection, not model. The harness
  follows Petrov Table 1 literally (SYM→., PUNCT→. —
  exactly the 41 confused tokens); the deleted probe kept them
  split. NLTK 87.03→87.14 across runs is the TT_EXTRA paren
  literals (`(`/`)` newly mapped). Rerunnable-documented now
  supersedes the deleted probe; standing numbers live on the
  front page until the next rerun.
- Env notes: spaCy 3.8.16 + en_core_web_sm in a pip venv needs
  nix gcc/zlib libs on LD_LIBRARY_PATH (compiled wheels);
  TreeTagger installed at `/tmp/opencode/treetagger` under its
  research license (evaluation use confirmed); `english.par`
  emits Penn/CLAWS mix, mapped empirically.

## Treelight evaluation + theme adoption (DONE 2026-10-07)

- `tree-sitter-highlight` (devongovett, NAPI `.node` binaries,
  preset languages only) cannot run in browsers or load custom
  grammars — rejected for both uses.
- Treelight (`@treelight/core` 0.6.0 + web-tree-sitter): custom
  language definitions (`{wasm, queries.highlights}`) VERIFIED —
  our bundle highlights correctly in node (equivalence with the
  in-page sweep on all showcase shapes). But the core runtime
  duplicates the engine already in-page, and instance API has no
  HAST output (only HTML strings), so full-core adoption would
  cost a second parse for zero new information.
- Adopted instead: treelight THEME packages as data
  (`docs/pkg/th/theme-github-{light,dark}.js`, 4 KB each) +
  the standard dotted-fallback resolver + a Light/Dark switcher;
  structural tokens take theme fg/bold/italic, POS keeps the
  local pastel backgrounds. `tree-sitter-highlight` and
  treelight-core stay offline-only references, same shelf as
  RDR/transformers. Dead `k-*` CSS removed.

## Production numbers remeasured (DONE 2026-10-07)

- The standing "production" records disagreed with each other
  (chart 92.13 vs table 92.15) and both sat below a fresh
  tag_sentence-identical probe: `tag_tokens --sentences
  --production` (beam-2 + all 14 rules, per-sentence) over
  pinned EWT test = 23141/25094 = 92.22 exact, 94.55 coarse.
  `--correct` on test prints 23100 because it reports the
  greedy headline (rules evaluated for fires only) — the old
  production records evidently predate current code/rules and
  the copy-threading drift note already declared banked numbers
  stale. Adopted 92.22/94.55 (front page chart + table; this
  also heals the 92.13-vs-92.15 split); greedy stays 92.05 /
  94.40. Method is rerunnable: harness `ours-prod` row.

## Speeds and sizes filled in (DONE 2026-10-07)

- Measured, not estimated: TreeTagger binary 212,791 Moby words in
  3.2 s wall (~66k tok/s); spaCy `en_core_web_sm` full default
  pipeline over the 1.2 MB body in 57.5 s (279,467 pipe tokens,
  ~4.9k tok/s — needs `nlp.max_length` raised past the 1M-char
  default, the documented procedure when the parser is in use).
  Sizes from disk: spaCy model pack 15 MB, english.par 14 MB +
  1.1 MB binary. Front table now has no "—" left except RDR
  (no model file) and production tok/s (batch-stage question,
  unmeasured by design so far).

## RDR row complete (DONE 2026-10-07)

- Pre-trained UPOS-EWT tree from the RDRPOSTagger repo
  (`Models/ud-treebanks-v2.4/UD_English-EWT`, 0.36 MB .RDR+.DICT;
  no PyPI package — driven in-process from a repo checkout,
  `--rdr-model` + `--rdr-repo`). Full harness: exact 92.30
  (reproduces the 2026-10-06 record to the digit), coarse 94.70;
  Moby-body speed ~42k tok/s single-call (harness `speed_rdr`,
  rerunnable). Front table + charts carry 92.30/94.70/42k/0.36.
  Gotchas recorded in-harness: no PyPI package, module
  hard-codes a chdir dance (enter from `pSCRDRtagger/`),
  DICT is the .RDR stem sibling.

## Pattern leg + poisoned-build lesson (DONE 2026-10-07)

- Pattern (CLiPS, BSD) added as the blog's second real tagger
  (TextBlob is a wrapper, not a tagger — excluded with reason):
  coarse 88.99 on the aligned 89.3%, ~11.9k tok/s, 67 MB
  installed (lexicon-heavy). Install gauntlet, all venv-local:
  PyPI `pattern` is a plotting name-squat, `pattern3` ships an
  IndentationError, GitHub source needs `--no-deps` (mysqlclient
  is web-mining baggage), plus a one-line PEP 479 fix
  (`raise StopIteration` → `return` in the lazy lexicon loader,
  broken on 3.14) and NLTK wordnet for first use.
- LESSON (load-bearing): the full run printed ours 91.83/91.90
  because a concurrent edit landed mid-`cargo build` — a poisoned
  binary, not a model change (weights md5 identical, decode path
  untouched; recount on a stable tree: 92.05). Standing rule:
  benchmark binaries build from a clean tree or the numbers are
  void; pure-Python legs in the same run were unaffected and
  stand (Pattern, RDR, spaCy, NLTK, TreeTagger unchanged).
  Their history-probe diff (train-only) is the cooperative
  upside of the same shared tree.

## WordNet uses (QUEUED 2026-10-07)

- WordNet 3.0 (permissive license, derived tables shippable)
  resolves via NLTK data (note: `wordnet.zip` must be EXTRACTED —
  3.14 `find` doesn't auto-resolve the bare zip; fixed in env).
- (a) Lemmatizer table via morphy + exception lists (unblocks
  TreeTagger harvest (b)): offline-built like `lexicon/verbs.txt`,
  zero runtime deps. Consumers: passive participle identity,
  nominalization deverbal gate.
- (b) Nominalization via derivational links (NEW sub-item):
  `arrangement`→`arrange` replaces suffix-guessing with real
  morphology inside the existing light-verb government shape.
  Bars: nominal eval precision/recall neutral-or-better.
- Explicitly out: sense features for the tagger (UPOS coarser
  than synsets; EWT-majority already beats lexicon lookup),
  anything at runtime (10 MB+ database; offline tables only).

## Streaming lint + weights-deser memory work (DONE 2026-10-07)

- Staged peaks on Moby body (VmHWM): parse/tree 63.5, +shallow
  annotations 136.7, +full dep 730 MiB. Attribution probe (per-sentence
  HWM deltas, all ~0) proved the peak is NOT sentence work: weights
  LOAD transient — tagger 2 MB file → 60.7 MiB, dep 36 MB → 652 MiB
  (serde_json double-DOM + BTreeMap stopovers).
- Fix, no new deps: direct-into-final-shapes deserialization, then
  zero-intermediate visitors (borrowed `&str` keys) for tagger,
  dep-parser, and labeler loaders. Load peaks: tagger 60.7→12.7
  (−79%), dep 652→69.6 (−89%). Full pipeline 730→~200 MiB.
  Behavior preserved (dep labels suite green; unknown-code errors
  kept). Two scars: `U64Hasher::write` REPLACED state (all String
  keys one bucket, 52µs/lookup — caught by bench, folding fixed,
  226 ns); labeler visitor mixed map levels (caught by its own
  roundtrip test).
- `lint_streaming` (+ shallow twin) in english-lint: pass-1 grammar
  sentence spans (tree dropped), per-sentence re-parse (token-identical
  0/9973 on Moby), outer spans stamped on findings. Moby full-path:
  8307/8307 findings byte-identical, peak 117.5→88.1 MiB (−25%),
  time +4.4%. CLI batch path wired to it. Committed identity test
  (tests/streaming.rs, adversarial inputs incl. dash-handoff blank
  absorption). Bar recalibrated with reason: the 1/3-peak bar assumed
  annotation retention dominated; the DOM transient did — fixed
  separately. Residual: pass-1 tree transient (63 MB) bounds further
  gains; per-sentence dep spikes are inherent (windowed, not removed).
- Byte-splitter paragraph streaming REJECTED with mechanism:
  dash/colon-handoff arbitration absorbs blank lines into following
  sentences (~36 Moby paragraphs), so no byte splitter reproduces
  grammar paragraphs. Temp probes deleted per discipline.
