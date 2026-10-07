# english-dep

Greedy arc-eager dependency parsing over tagged English prose
(unlabeled heads + UD relation labels), as a post-tagging pass —
never grammar (argument structure belongs in Tier-3 exile with
word classes and phrases).

`Model::parse_beam` (width 4) decodes head indices with a static
oracle-trained averaged perceptron over parser-configuration
features; `LabelModel::predict` classifies relations over decoded
arcs. Margins (best minus runner-up legal action) feed the same
gated-correction discipline as tag margins. Measured, EWT with gold
tags: UAS 83.9% / LAS 80.2% (+tagger pipeline 77.8% / 71.3% —
the cascade is structural, see `docs/joint-tag-parse.md` for the
stopped program). Batch/save-pass stage (~4 s/book at beam 4),
never keystroke.

Weights (`weights/dep.json` + `weights/labels.json`, 32 + 3.8 MB)
are Tier-1 lazy assets — gitignored, regenerated offline by
`crates/english-dep-train` (`--beam-train 4`, `--labels`) from
out-of-repo UD data. Do not vendor them.
