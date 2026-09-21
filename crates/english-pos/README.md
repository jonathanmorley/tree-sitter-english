# english-pos

Statistical part-of-speech tagging for English prose (17 Universal POS
tags), as a post-parse pass over segmented text — never grammar (word
classes in `grammar.js` were tried upstream and dropped; see Tier 3).

A greedy perceptron: `Model::tag` decodes surface tokens left to right
(lowercased internally; shape features read the raw forms). Weights
live in `weights/upos.json`, trained by `crates/english-pos-train` on
UD English-EWT: dev 90.35%, test 90.53%, 1.29 MB.

```rust
let model = english_pos::Model::from_json(include_str!("weights/upos.json"))?;
let tags = model.tag(&["Time", "flies", "like", "an", "arrow", "."]);
// [Noun, Verb, Adp, Det, Noun, Punct]
```

Caveat: UD tokenization splits contractions (`do` + `n't`) while the
`english` crate keeps them whole (`don't`). Tagging whole words works
but loses the contraction-internal signal; aligning the two
tokenizations is future work.
