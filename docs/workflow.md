# How quality work happens here

The repeated processes that move quality (parse, tags, chunks,
dependencies, lint findings) in this
repo. Each names the failure it prevents and the measured artifact it
leaves behind. `AGENTS.md` is the *log* of every run; this file is the
*method* — the thing to consult before starting the next round.

## 1. Measure before you change (the standing reflex)

Every change lands against numbers that existed *before* the change,
and ships only when the deltas are recorded beside it.

- **Bars first.** Set the accuracy floor before touching code
  (tagger `moby.rs` 0.84, `genre.rs` 0.86, `hard.rs` 0.84,
  `sweep.rs` 0.86/0.87, chunk bars 0.43/0.87 and sweep-chunk
  1.0/0.80/0.05, dep UAS 87 and LAS 77/71, lint bars per rule:
  passive 0.85/0.60, nominal 0.75/0.50, complexity 0.80/0.65,
  weasel 0.80/0.65, hedge 0.80/0.50). A bar moves deliberately,
  with a recorded reason, or the change is reverted.
- **One weight file, two gates.** `english-pos/weights/upos.json` is
  committed reading only. The dev/test gates (`--eval-only`) run on
  *gold gold* — UD EWT test is never both a dev proxy and a report.
- **md5 hygiene.** Weights are md5-tracked at every step; echoed
  intent is never a substitute for verified bytes; retrains are
  deterministic — suspect suspect inputs, not the algorithm.
- **Rejections are results.** Every 0-for-N (distillation 0-for-5,
  mechanics 0-for-2, char 0-for-2, clusters 0-for-1, beam-select,
  attr-adj and color-adj, joint program at Stage 0, light-`do`)
  is committed with its forensics — the register of things
  measured and refused. It is what keeps the roadmap honest when it
  says "every direction has a measurement." New standing rule from
  the color-adj refusal: EWT-majority conditions on the PREDICTED
  tag (shape-majority ≠ fire-precision).

## 2. Margins as the gate (the cheap-but-load-bearing idea)

The perceptron's score gap (best minus runner-up) drives every
post-pass decision. This is the whole architecture of "safe by
construction":

- **Beam re-decode** (width 2) re-decides only runs below margin 2.0
  with the history pool — the expensive generality never touches
  confident tokens. Beam cannot be run "rules first" because it
  optimizes the scorer; rules encode what the scorer lacks.
- **Correction rules** fire only inside `0 < margin < threshold`.
  Margin *zero* ties abstain by design (ties carry no signal; see
  `flies` veto and the tie-lottery lesson). Rules that measured
  net-negative at every threshold are removed, names preserved in the
  rejection log.
- **Margin lists feed the harvest.** `candidates` ranks sentences;
  the `verify` example buckets incoherences (error / no-predicate /
  joiner / transcription); the eval-margin probe mined the shipped
  `that-det`. Probe examples are throwaway and deleted after use.

## 3. Negative knowledge wins (the reward system)

- **Eval before model change.** Weights move only with greedy/prod
  /-correct deltas printed and the canonical-`flies` veto held on all
  decode paths. Recent admitters: participle repair `pass-by`
  (test +1) and `quite`-adverb `quite-adv` (dev/test +1 each);
  everything refused since carries forensics (color-adj dev −1,
  light-do best-context 85%, v1.1 extensions with zero fires).
- **EWT-majority oracle discipline.** No heuristic is admitted on
  template shape; it ships only with EWT majority + measured dev/test
  ≥ 0 + Moby/genre deltas. EWT is the arbiter of ambiguity;
  invented classes do not exist.
- **No misparse is enshrined.** A test that pins a known-wrong
  reading is a placeholder. The corpus TDD below exists to edit that
  sentence.
- **The bar for "done" is a standing cross-book record with no new
  actionable class**, not a to-do list. (`No.`-removal, `that-sconj`,
  `?";` etc. are the wins that *created* the record.)

## 4. Deterministic artifacts (trust by hash)

- **Weights** serialize deterministically (sorted keys, integer
  weights, stripped zeros); same code+data ⇒ same bytes, md5
  `56082361`. Determinism is the test that the pipeline is closed.
- **Snapshot per slice.** `examples/*.parse.txt` diff on every
  grammar/scanner edit; audit histograms bucket ERROR/MISSING into
  prose vs transcription by *character class* (never counted by
  output). A histogram rise is a stop.
- **Full workspace stays green.** `cargo test --workspace`, fmt,
  clippy are gates, not reports.

## 5. Harvest → TDD → measure (the loop)

For every error class:

1. **Harvest** from book baselines (Moby-Dick + the `/tmp/opencode`
   book cache) with `audit` (prose) + `candidates` (uncertainty) —
   *never* from error counts alone; transcription stays excluded by
   bucket, not by judgment.
2. **Write the corpus test first** (`test/corpus/*.txt`) in the
   exact grammar input shape (lines between header and `---`,
   blanks matter).
3. **Fix minimally** — scanner arbitrates the duality, grammar
   inherits at aliases; zero `generate` conflicts is a hard stop.
   Scanner refusal rewinds fully; handoff vs join reading is decided
   by the scanner, not LR precedence.
4. **Measure**: corpus + snapshots + full gates + histogram deltas
   on Moby/books. Fix rules admitted only with EWT dev/test ≥ 0;
   grammar fixes need snapshot identity + no histogram rise.

## 6. Gold-label and hand-tag discipline (the gold standard)

- **WORD+TAG lines, arrays generated.** Hand tagging is *one* token
  per line, `word TAG`; Rust arrays are generated from those lines
  (`tests/*.rs` — `sweep.rs` is built from a verified gold file,
  see commit `Cross-book sweep eval`). Never hand-align a parallel
  array > ~200 tokens; hand-aligned arrays drifted (3 dropped tags
  in review).
- **Every contested call carries its EWT count** in-file
  (`touching` VERB 2:0; `than` ADP:156 SCONJ:41; `whole` ADJ:46;
  `as` SCONJ+verb-ahead 325:76). Where EWT is zero, the UD
  guideline decides (`in want of` → constituent noun, bare singular)
  and the decision is recorded.
- **Gold files move through edits, not rewrites.** Sweep-gold is the
  hand-verified source; the Rust test is generated once from it and
  then pinned. When two hand-fix batches were reviewed but never
  applied, the production-vs-greedy break diff caught *gold*, not
  rules.

## 7. The rejection log (negative results are permanent)

Kept in `AGENTS.md` "Queued"/"DONE" blocks and the train README's
batch table. The standing list to cite when a proposal recurs:

- POS: Collins averaging (falsified 33% vs 88% — while averaging
  *wins* for the parser, +7.1: same discipline, opposite verdicts),
  Collins EM, tagdict, stemming-as-backoff, more `w+t-1`
  (sparse overfit), small `-s`/imperative Brill rules (both rejected:
  `theories` and `Lifts`), char 4-5 suffixes (title-fragmentation /
  NOUN→PROPN), Brown clusters (0-for-1), micro-batch 06, finetune 03,
  light-`do` AUX (best context 85%, needs ~99%), v1.1 gap/irregular
  extensions (EWT-majority, zero fires anywhere).
- Grammar: em-dash joins in subordinate interiors (stranded `but`),
  markup tolerance (strip test: 0 improvements, transcription by
  bucket won), leading-dash dialogue (priced out), `{2,}` ellipsis
  bound (silent miscompile — *know your generator's silent failures*).
- Runtime: neural tagging (BERT 1000× budget), Punkt port as
  runtime (offline only), Plato/statistical NP chunking in-grammar
  (Tier 3).

## 8. What to reach for next

A standing order of operations for the next round, by expected value:

1. **Harvest from `candidates` again** (fresh cuts of the book
   cache) — the pipeline for every win since.
2. **Margin-probe any new miss class** before writing a rule: gate
   must reach it, EWT majority must support it, and it must not
   reopen a `s-verb`/`Lifts` rejection.
3. **Corpus TDD** for the fix, then the bar.
4. **Record** in AGENTS.md and `references.md`: what was measured,
   what was rejected, what was admitted, with numbers.

When the standing bar is "no large auto-imported list, no dual-use
entry without a digit guard, no new class without EWT majority" —
quality has ratcheted: each round adds one verified sentence of
pull-request-grade evidence, and the number of *known safe*
constructions grows.
