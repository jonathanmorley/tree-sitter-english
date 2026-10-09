# Joint arc+label: scope (no code)

MaltParser's DEPREL ldep/rdep features are worth +3.1 UAS with
identical inputs (ablation 2026-10-08) — the precise shape of the
table loss. Our split pipeline (arcs, then labels on frozen
heads) cannot see that signal. This doc scopes the coupling.
Status: design only. No code lands without clearing §4 in order.

Numbers below use the v5 banked baseline (dev UAS 85.90, test
84.99 +gold; 79.75/79.56 +tagger; LAS 82.07/81.27,
73.18/73.31). The error-overlap study repriced the prize to
≤ +1.5 net (realistic +0.5–1.0): both systems fail in the same
places, so no lever captures the full +3.1.

## 1. The mismatch lesson applies

Two pred-tags treatments (labeler +0.5/−0.7, parser +1.1/−0.8)
were rejected under pipeline-moves-AND-gold-holds: training on
predicted tags trades clean-input quality ~1.5:1 against
pipeline gains. Label features trained on gold labels and
decoded with predicted ones are the same tradeoff wearing a
different coat — presumed guilty. Any proposal trains clean
(gold labels, gold heads) and proves the mismatch tax
affordable at inference. Bars (dev, beam4): gold UAS ≥ 85.7,
gold LAS ≥ 81.9; pipeline UAS must move ≥ +0.5 with LAS
neutral-or-better (current 85.90/82.07, 79.75/73.18).

## 2. Options

### A. Two-pass feedback (recommended first, reversible)

Pass 1: today's pipeline (greedy tags → beam4 arcs → v1
labels). Pass 2: re-parse with label features (ldep/rdep
DEPREL + tag conjunctions, Malt's shapes ported to 0x60+
templates) reading pass-1 predicted labels, then relabel
pass-2 heads. Falls back to pass 1 on any regression —
reversible by construction, no decoder surgery (reuse
`parse_beam` with a labels slice), ships incrementally.

Risk: pass-1 label noise poisons pass-2 arcs (the mismatch
tradeoff). The Stage-0 mismatch probe (§4) prices it before
any LaSO run. Cost ~2.5× batch decode (parse+label twice);
keystroke path untouched (POS/chunk only); dep stays a
batch/save-pass stage per the tiering.

### B. Joint beam (queued behind A)

Expand beam states with label choice per arc; LaSO retrain
with joint oracle. Most principled, most expensive (decoder
surgery + new oracle + full retrain). Starts only if A hits
the mismatch wall AND the oracle bound (§4) says headroom
remains — i.e. evidence, not impatience.

### C. Unified single model (rejected upfront)

One perceptron over tags+arcs+labels. Scope explosion:
re-opens the STOPPED tag-parse coupling (confident-mistag
class, capture 0.139) with none of its Stage-0 gates cleared.
No.

## 3. Stage 0 (greedy trains only — no LaSO without passing)

Templates land first (0x60+ label features, additive). Then:

- **0a — oracle bound**: greedy train with label features on
  gold labels; decode with GOLD labels. Unshippable by design;
  measures the feature-value ceiling for OUR learner.
- **0b — mismatch tax**: same weights, decode with PREDICTED
  labels (v1 labeler on greedy-best heads). Measures what the
  tradeoff eats.
- **Gate**: 0b beats greedy baseline (v5-greedy dev 84.29)
  → LaSO two-pass training earns its run (train-clean,
  decode-predicted, same bar as §1). 0a >> 0b with 0b flat →
  mismatch dominates; do NOT LaSO-train around it (the
  twice-measured 1.5:1 trade says that path loses) — record
  and stop. 0a flat too → features carry nothing here;
  reject the stage.

## 4. Evals (all must hold at every step)

EWT dev/test UAS+LAS (gold + greedy-pred tags), PUD UAS/LAS,
lint passive/nominal evals (they read labels — better heads
can move them; neutral-or-better), full workspace green,
`flies` holds (POS untouched, but the pipeline feeds the
linters — verify, don't assume). Weights stay gitignored
Tier-1; md5 hygiene per retrain.

## 5. Revisit 2026-10-09: uncertainty-gated coupling (measured,
   program stays stopped)

The one untested coupling idea since the stop: gate label
features on labeler margin — couple only confident labels
(margin ≥ T), back off to arc-only features below. Unlike A/B/C
it concedes the mismatch instead of fighting it: confident
labels are mostly right, so the tax applies where it costs
least. Pre-registered bars: capture P(margin<T | label error)
≥ 0.5 AND precision P(correct | margin≥T) ≥ 0.97 — else the
gate cannot reach enough errors cleanly enough to matter.

Measured (temp `labelmarg` probe, since deleted; pipeline
regime throughout — our tags, beam4 heads, correct heads
only so head errors don't pollute label separability; EWT
dev, 20,495 tokens, 1,476 label errors): capture 0.035 at
T<1, 0.094 at T<2, 0.236 at T<5; precision 0.930–0.943
against a 0.928 base rate. Both bars miss by miles — label
errors are confident too (76% sit above margin 5), the fourth
member of the confident-mistag class (tags 0.139, chunk 22/22,
eval-margin 244, now labels 0.236). Even perfect gating
captures a quarter of the errors to fix a subset that is
barely above base rate: expected value ≈ +0.1 for decoder
surgery plus a retrain, below any admission bar ever set
here. No stage, no code, probe deleted.

Program stays stopped. The full menu is now exhausted — 0a/0b
(greedy joint), B (joint beam, parked behind evidence that
never arrived), C (unified, rejected upfront), uncertainty
gating (this section) — all measured or refused with reason.
Reopen triggers, and only these: (1) a label-quality
breakthrough that moves the confident mass (nothing queued —
pred-tags labeler went the wrong way); (2) a consumer that
funds a decoder stage regardless of expected value (the
UAS-87 lesson: numbers, not hunches, and the number here
rounds to zero). Do not relitigate the mismatch without one
of the two.
