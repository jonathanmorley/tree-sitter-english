# english-joint

Joint neural tagger-parser (R3-5 Stage 1): shared word+char BiLSTM
encoder (R3-1 screen dims) with UPOS + biaffine arc/label heads and
Chu-Liu-Edmonds MST at decode. Batch/save-pass only — never
keystroke, never a replacement for the committed decoders until the
staged bars pass (`docs/joint-neural-scope.md`).

Encoder math reuses `english-pos-neural` (`HandRolled` backend,
`LstmParams`, `dot`, `WordCache`); orchestration, heads, and MST
live here. Weights are Tier-1 (/tmp until admission), score-gated
never md5-gated.

```rust
let model = english_joint::JointModel::from_json(&weights_json)?;
let p = model.parse(&["Time", "flies", "like", "an", "arrow", "."]);
// p.tags / p.heads (-1 = root) / p.rels
```
