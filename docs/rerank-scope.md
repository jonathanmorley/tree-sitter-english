# Reranking scope (item 5, SCOPED 2026-10-09 — unbuilt)

N-best tag paths rescored with global features. The only clean
vehicle for `that`-gap detection (relativizers with pronominal
subjects go SCONJ only 111:50 in EWT — unseparable locally, per
the sweep-margin probe) plus parallelism symmetry and
prep-chain shapes. Local machinery is exhausted here: 17
correction rules cover predicted-tag local shapes, beam-2 spans
cover local uncertainty, and the mirror experiment (item 4)
proved disagreement alone is not a selector (breaks 34 dev /
8 sweep). What remains needs features over the whole path.

## Design

- **N-best decoder**: k-best extension of second-order Viterbi
  (features depend only on t-2/t-1/words, so prefix states are
  exact; keep k best prefixes per state). Deterministic
  lowest-index tie-breaks throughout (decoder lesson record).
- **Rescoring features** (all path-global, none available to the
  local model): base model path score (as one feature),
  forward/mirror agreement count (subsumes item 4 — the mirror
  becomes a signal, never an override), relativizer-gap
  indicators (`that` + nominal-subject + finite-verb-ahead with
  no local gap filler), coordination parallelism symmetry
  (conjunct tag/span symmetry), prep-chain run features.
  Dozens of weights (bytes), not megabytes.
- **Training**: Collins-style perceptron rerank over train
  n-best (oracle = min-loss path), offline in
  `english-pos-train`, deterministic (fixed order, md5-pinned
  like every weights change). Single-model features first;
  mirror-agreement joins only if the single-model reranker
  admits (one variable at a time).
- **Integration**: in-crate rescoring after beam decode;
  keystroke measured (k-best + rescore must hold the ~40 ms
  budget — tag pass is 9 µs/sent today, so k ≤ 10 likely fits,
  but measured not assumed).

## Stages and gates (STOP rules pre-registered)

- **Stage 0 — prize census, no code**: production dev errors in
  global-only shapes: (a) relativizer-gap (`that` + nominal
  subject + finite verb, gold flips the relativizer reading);
  (b) parallelism (CCONJ-adjacent mistag with asymmetric
  conjunct readings); (c) prep-chain (ADP/SCONJ/ADV flip with
  ≥ 2 same-class neighbors within ±3); (d) mirror-agreement
  fixes (the 64 dev takes item 4 found). Proceed to Stage 1
  iff (a)+(b)+(c)+(d) ≥ 250 dev tokens (1 pt — medium-high
  cost needs a visible prize); else STOP, item closes.
- **Stage 1 — k-best + oracle reachability**: k=10 decoder,
  gold-in-10 rate on dev. Proceed iff ≥ 98% (below that the
  base scores can't even rank the truth reachable — the
  Viterbi lesson repeating); else STOP (revisit only via item
  13 LaSO, which recalibrates the base scores).
- **Stage 2 — train reranker**: perceptron over train n-best.
  Proceed iff beats production (beam2+rules+17 over median)
  on dev AND test.
- **Stage 3 — integration**: full bars — all book/chunk/lint
  evals neutral-or-better (sweep zero-breaks holds), `flies`
  holds on all paths, keystroke measured, determinism
  (md5-identical retrains), size (bytes — Tier-1 trivially).

## Non-goals and relations

- Not a replacement for the correction layer (local shapes stay
  local) or beam spans (local uncertainty stays spanned).
- Item 13 (LaSO) is independent and compatible: rerank sits on
  whatever base scores exist; if LaSO lands first, rerank
  retrains on LaSO scores (same recipe).
- No new runtime deps, no model-file growth beyond bytes,
  no grammar involvement (Tier-3 rule holds).
- If admitted, the rerank weights + recipe join the kept set
  (`ensemble`/`average-members` precedent); if any gate fails,
  full revert (item-4 precedent) with the census kept as the
  residual record.
