# Soft-target distillation scope (item 7, STAGE 0 PASS 2026-10-09)

Five hard-label distillations failed 0-for-N (joint 03/finetune 03/
joint 04/joint 05/micro-06, frozen-prior, counterweight — all
shared-prior drift). This item is the one surviving mechanism:
distilling DBERT *distributions* preserves teacher uncertainty
instead of yanking toward argmax noise.

## Stage 0 — mechanism check (DONE, offline, no code)

DistilBERT opt-fp32 posteriors (first-subword, UPOS head direct)
over the 112 gold eval sentences (sweep/genre/hard/moby) vs our
averaged-production errors there (355):
enrichment 2.62× in the top-entropy quartile (bar 2.0), capture
0.656 (bar 0.2); mean teacher entropy 0.349 on our errors vs
0.098 on correct tokens; teacher right 80% in-zone; only 81/355
errors have teacher agreeing with us (the hard-label-covered
confident-drift class). PASS on both pre-registered bars
(parse-validation shape). Conclusion: our errors sit where the
teacher is uncertain AND mostly right — exactly where soft
labels hedge noise that hard labels would bake in.

## Stage 1a — soft trainer on EWT (MEASURED AND REJECTED
2026-10-09): soft-averaged perceptron (fire iff model-argmax ≠
teacher-argmax, delta = q − onehot, η=1.0, timestamp averaging,
gold history; posteriors extracted once over EWT train, 12,544
lines word-verified (extraction PROCEDURE kept as
`scripts/dbert-posteriors.py`, verified argmax-identical against
the ORT record at 6dp; bytes stay /tmp-ephemeral per the
no-training-data rule); deterministic md5-verified
retrain). Result: production dev 23581 (+25) / test 23603 (−68
vs 23671) — STOP, no book text touched. Mechanism: smoothing
absorbs teacher habits that disagree with EWT-test conventions
(same drift signature as the five hard failures, now
in-domain); the Stage-0 in-zone hedging is outweighed by
confident-drift teaching everywhere the model and teacher
differ. Softness dials drift, nothing resolves it — as scoped.
Code fully reverted (train_soft + --soft + prodscore deleted),
suite green, weights untouched.

## Stage 1b — soft book batches (NEVER STARTED: 1a failed its
bars — correctly, per the pre-registration).

Small batches (≤1k sentences, micro-06 scale) of book-domain
text with teacher posteriors, EWT-gates discipline (dev/test
neutral-or-better or STOP — the frozen-prior/counterweight
guardrails). Only if 1a proves the machinery. The drift tension
remains (gains need shared-row movement); softness dials it,
nothing resolves it — 1b can still fail after 1a passes.

## Non-goals

- No model posteriors (forward-backward stays unbuilt — hard
  model-argmax + soft teacher needs none).
- No hard-label silver of any size (closed permanently, item
  silver-close).
- No runtime change (distilled weights only, same shapes).
- If any gate fails: full revert, probes stay /tmp-ephemeral.
