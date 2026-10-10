# R3-1 neural port scope: `english-pos-neural` (staged, STOP rules apply)

Batch/save-pass BiLSTM tagger. The perceptron keeps the keystroke
path unconditionally — this crate never touches it.

## Product shape

- New crate `crates/english-pos-neural`, path-deps on `english`
  (segmentation) + `english-pos` (`Tag`, wire pieces, contraction
  splitting, correction rules — never its weights).
- Zero-dep core: hand-rolled f32 LSTM/matmul backend first
  (preserves the zero-dep property); `Backend` trait keeps a
  candle backend optional behind a feature flag, never default.
- Weights are Tier-1 lazy assets (gitignored when admitted);
  trainer stays /tmp Python (`round3/bilstm_screen.py`) — no
  `-train` crate unless Rust-side training earns its keep.
- Score-gated, never md5-gated (torch-CPU retrains wobble ±0.2).

## Bars (set before work, all required for admission)

- Accuracy: greedy dev/test ≥ committed greedy (93.58/94.23);
  production (neural + reused correction rules where margins
  allow) ≥ committed production (93.67/94.33); PUD/GUM/sweep/
  genre/hard/moby neutral-or-better; `flies` holds on all paths.
- Parity: Rust f32 argmax-identical to torch on a pinned dev
  sample (tolerance: 0 diffs; the export already verifies 50/50
  at the JSON level).
- Speed: batch tok/s measured against the dep batch precedent
  (~44k tok/s); bar set from the f32 measurement before quant
  work, never assumed.
- Veto: no flies-VERB pin on the neural path (decided Stage 1:
  the veto guards perceptron suffix-memorization over EWT's single
  VERB "flies"; the BiLSTM reads NOUN 6.96 vs VERB 5.17 — pinning
  one ambiguous token would be post-hoc fitting; dev/test gates
  are the change-detection).
- Size: quantized artifact single-digit MB (f32 export is
  28.5 MB — the quant stage must be accuracy-neutral).
- Determinism: single-thread inference, fixed reduction order;
  document the residual wobble band.

## Stages (stop at first FAIL, probes /tmp-only until admission)

- Stage 0 — export + JSON parity: DONE 2026-10-10 (28.5 MB,
  50/50 argmax-identical reload). Backend choice stays open.
- Stage 1 — f32 Rust parity + accuracy gates: scaffold crate,
  hand-rolled forward (embed + char-BiLSTM + word-BiLSTM +
  linear + softmax/margins), pinned-sample parity, then full
  dev/test + book evals + `flies`. Parity miss → STOP.
  DONE 2026-10-10 (PRs #118/#121): parity 0/50; Rust dev
  93.83 / test 94.11 (torch run-2 to the token); sweep 0.9110
  (tied with perceptron greedy/production); flies-veto
  deliberately not ported (see Veto below).
- Stage 2 — speed: MEASURED 2026-10-10 at ~2k tok/s release naive;
  bar ≥20k (dep-batch class) via the inference-optimization playbook
  (buffer reuse, batched char encode, f32 sums re-gated by parity,
  sentence batching, then quant). Bar set from the measurement,
  never assumed.
- Stage 3 — size: int8 quant of matmuls (embeddings stay
  higher precision first), accuracy-neutral re-gate. Damage →
  narrower quant, never accuracy spend.
- Stage 4 — admission: production comparison (neural greedy +
  portable correction rules + beam-equivalent if earned),
  front table + chart + READMEs, weights vendored as Tier-1
  pair. Any bar miss → weights stay /tmp, crate stays
  experimental.

## Explicitly out

- Keystroke path, Rust training, silver data for the perceptron
  (0-for-6 stands), changing `english-pos` APIs (additive only),
  candle-by-default, WASM until native passes.
