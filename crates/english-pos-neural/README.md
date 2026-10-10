# english-pos-neural

Batch/save-pass neural POS tagging (BiLSTM over words + char-BiLSTM)
— never the keystroke path (the averaged perceptron in `english-pos`
keeps that unconditionally).

Stage 1 (this crate): hand-rolled zero-dep f32 forward behind the
`RecurrentBackend` trait (a candle backend may implement it later),
`Model::from_json` over the export JSON, greedy `tag` plus
`tag_margins` (best minus runner-up) for the same gated correction
discipline. Weights are Tier-1 lazy assets: `/tmp` until the
neural-scope bars pass (parity → accuracy → speed → size →
admission), score-gated never md5-gated. See `docs/neural-scope.md`.

```rust
let model = english_pos_neural::Model::from_json(&weights_json)?;
let tags = model.tag(&["Time", "flies", "like", "an", "arrow", "."]);
// [Noun, Verb, Adp, Det, Noun, Punct]
```
