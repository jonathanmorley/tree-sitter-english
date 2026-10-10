# R3-5 joint neural tag-parse: scope (no code yet)

The cascade is the pipeline's largest structural loss (UAS gold 85.9
→ pipeline 80.7 test; LAS 82.1 → 75.0; −6.1 preserved on PUD) and the
only one twice confirmed untrainable-around (pred-tags parser/labeler
both rejected ~1.5:1). R3-1 changes the available machinery: the
program that stopped in `docs/joint-tag-parse.md` coupled at
inference with clean linear weights because joint *training* was
presumed guilty. A shared neural encoder reopens the training side —
multi-task supervision is not noise adaptation (gold tags AND gold
heads supervise one encoder; nothing trains on its own predictions).
This doc scopes that program. Status: design only.

## 1. What unblocks now (and what stays closed)

- Unblocked: a shared word+char BiLSTM encoder (the admitted R3-1
  screen architecture) with two heads — UPOS tagger + dependency
  parser — trained multi-task on gold EWT. The encoder sees both
  signals; neither head trains on the other's predictions.
- Stays closed: the stopped program's verdicts on margin coupling
  (+0.83 headroom, capture 0.139 — the addressable remainder for
  uncertainty features, not for shared representation), joint
  arc+label decode over linear features (pred-noise tradeoff ×3),
  runtime backfeed, and the UAS-87 bar (retired).
- Malt lesson carried in: label-informed arcs are +3.1 (the biggest
  measured lever) — the parser head should predict arcs AND labels
  jointly (biaffine, Dozat & Manning shape), not arc-eager + post
  labeler.

## 2. Design under test (Stage 0 picks)

- Encoder: R3-1 word-embed + char-BiLSTM + word-BiLSTM (dims from
  the screen, tunable in-screen).
- Heads: (a) 17-way UPOS softmax (same as screen); (b) biaffine arc
  scorer + biaffine label classifier over encoder states
  (Dozat & Manning 2017 shape; Eisner/Chu-Liu-Edmonds well-formedness
  at inference — projective-first like the static oracle).
- Training: multi-task (tag loss + arc loss + label loss) on gold
  EWT train, /tmp torch-CPU (the R3-1 screen recipe extends).
- Inference: Rust port later (same hand-rolled playbook, Tier-1
  weights); keystroke path untouched (batch/save-pass only).

## 3. Bars (all required to scope the port; EWT-first, books second)

- Tag: greedy dev/test neutral-or-better vs committed greedy
  (93.58/94.23) — joint training must not cost the tagger (the
  pred-noise tradeoff in reverse).
- Parse: pipeline UAS/LAS strictly above banked beam4+tagger
  (dev 80.89/74.85, test 80.72/75.00 UAS/LAS — averaged-tagger
  refresh) with gold cells neutral-or-better per split (dev gold
  UAS ≥ 85.7 / LAS ≥ 81.9, test gold UAS ≥ 84.8 / LAS ≥ 81.1:
  the pipeline-moves-AND-gold-holds bar at ±0.2 on banked
  85.90/82.07 dev and 84.99/81.27 test).
- Generalization: PUD pipeline neutral-or-better; sweep/genre/hard
  book evals neutral-or-better; `flies` holds.
- Budgets: Tier-1 size (single-digit MB target via the int8
  playbook); batch speed measured against dep batch (~42k tok/s
  parse-only — joint does tags+parse in one pass, so parity is
  against the SUM of tag+parse batch times); determinism bands
  (score-gating, never md5 — the R3-1 lesson).

## 4. Stages (stop at first FAIL; /tmp-only until admission)

- Stage 0 — offline screen: multi-task joint in /tmp torch on EWT;
  report tag exact + UAS/LAS (gold and pipeline regimes) + PUD.
  Tag-costs-anything or pipeline-misses-bars → STOP (no port).
- Stage 1 — Rust port parity + accuracy gates (same discipline as
  R3-1: argmax-identical heads on pinned samples, then full gates).
  DONE 2026-10-10 (PR #132): `english-joint` (factored biaffine,
  raw roots, Rust MST brute-force-verified); parity 0/0/0; Rust
  dev/test/PUD reproduce torch-avg; sweep 0.9086; `flies` VERB.
- Stage 2/3 — speed/size (int8 playbook; biaffine matrices quantize
  like the tagger's).
- Stage 4 — admission (front parser table + charts + READMEs,
  Tier-1 weights; keystroke path untouched).

## 5. Explicitly out

- Keystroke-path changes, Rust training, arc-eager revival (the
  biaffine choice is the Malt-lesson consequence — no second
  decoder horse race without a measured reason), pseudo-projective
  lifting (premise failed: stranding ≈ 0), WASM until native
  passes, silver data for any head (0-for-N stands).
