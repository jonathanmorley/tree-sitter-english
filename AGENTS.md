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
  work now stands: distillation 0-for-5, char 0-for-2 variants,
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
  (2) correction layer 3-for-14 — engine plus first shipped rules
  (`have-verb`, `to-prep`, `to-verb`) and the beam decoder; the
  load-bearing unproven piece left is distillation scale-up
  (0-for-5: joint 03, finetune 03, joint 04, joint 05, micro-06
  all rejected — 05 is the mirror case, evals up / EWT down, and
  still rejected per EWT-gates discipline; 06 proves composition
  dominates mass, killing the drift-floor hypothesis);
  (3) chunker has no accuracy number — CLOSED 2026-10-05 by the
  genre end-to-end chunk eval (rule 20/20, cascade 9/20 sent /
  0.873 token, bar 0.43).
