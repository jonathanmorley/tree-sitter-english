# english-pos-neural

Batch/save-pass neural POS tagging (BiLSTM over words + char-BiLSTM)
— never the keystroke path (the averaged perceptron in `english-pos`
keeps that unconditionally). Nothing in the workspace depends on this
crate; consumers opt in explicitly at Stage-4 admission.

Hand-rolled zero-dep forward behind the `RecurrentBackend` trait (a
candle backend may implement it later — as a default-off cargo
feature, never a default dep), greedy `tag` / `tag_cached` (caller-kept
`WordCache`, `Model: Sync` for scoped-thread batch tagging) plus
`tag_margins` for the same gated correction discipline. Weights are
Tier-1: the int8 artifact (`weights/upos-i8.json`, 8.2 MB,
single-digit rule via compact separators) is vendored and admitted
(EWT+GUM+LinES training; test 94.52/96.46 +18 rules; dev
94.58/96.17; sweep 0.9312; score-gated never md5-gated); the f32
export stays a /tmp speed reference. See `docs/neural-scope.md`.

```rust
let model = english_pos_neural::Model::from_json(&weights_json)?;
let tags = model.tag(&["Time", "flies", "like", "an", "arrow", "."]);
// [Noun, Noun, Adp, Det, Noun, Punct] (noun-noun reading, deliberately
// unpinned — see the flies note in docs/neural-scope.md)
```

## Tradeoff: pick the artifact, not a feature flag

No `f32`/`i8` cargo features — both paths are zero-dep, so one
binary serves both and the weights file selects the tradeoff at
runtime (`Model` reads `"params"`, `QModel` reads `"qparams"`; the
examples auto-detect). A compile-time split would buy nothing and
cost a shared-code third crate.

| artifact | size | speed, 2-core (single / scoped×2) | accuracy vs f32 |
|---|---|---|---|
| f32 (`Model`) — speed path | 43.4 MB JSON | 11.3k / 19.3k tok/s | reference |
| int8 (`QModel`) — size path | 8.2 MB JSON | 6.9k / 11.5k tok/s | dev +3 / test ±0 |

Per-row symmetric int8 (offline absmax; biases/states f32;
per-vector activation quant). Single-digit MB is the size bar;
batch-viability is the speed bar: the shipped i8 runs 11.5k tok/s
scoped×2 on 2-core silicon (realistic docs <1s; books ~20s worst
case); the f32 reference holds 19.5k vs the retired ≥20k line.
