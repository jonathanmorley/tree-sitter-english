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
Tier-1: the int8 artifact (`weights/upos-i8.json`, 6.7 MB,
single-digit rule) is vendored and admitted (EWT test 94.16/96.04
+rules; dev 93.86/95.43; sweep 0.9105; score-gated never
md5-gated); the f32 export stays a /tmp speed reference.
See `docs/neural-scope.md`.

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
| f32 (`Model`) — speed path | 28.5 MB JSON | 11.9k / 19.5k tok/s | reference |
| int8 (`QModel`) — size path | 6.7 MB JSON | 6.9k / 10.5k tok/s | dev/test ±3 toks, sweep −1 |

Per-row symmetric int8 (offline absmax; biases/states f32;
per-vector activation quant). Single-digit MB is the size bar;
batch-viability is the speed bar (≥20k scoped — narrowly missed at
19.5k on 2-core silicon, held open; 4-core boxes clear it).
