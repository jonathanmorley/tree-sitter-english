# english-pos-train

Offline trainer for the `english-pos` perceptron. Not published.

Fetch the data with `scripts/fetch-ud.sh` (pinned revisions; the
corpora stay out of the repo — GUM and LinES are CC BY-NC-SA and
cannot ship here, EWT is CC BY-SA; see the script header):

```sh
scripts/fetch-ud.sh --dir /tmp/ud ewt
cargo run --release -p english-pos-train -- \
  --corpus /tmp/ud/ewt --iters 20 --min-count 1
```

Trains on the preset's `en_ewt-ud-train.conllu`, reports dev/test
accuracy, and writes `../english-pos/weights/upos.json` (committed:
dev 91.84%, test 92.05%, 2.01 MB — EWT plus in-domain oracle data
below, at iters=20/min-count=1). Re-evaluate committed weights any
time with `--eval-only test` (or `dev`). Base hyperparams are
dev-selected: a sweep over iters {5,10,15,20,30} × min-count {1,2}
peaked at iters=15 / min-count=1 (dev 92.09%) but flipped canonical
"flies" VERB→NOUN (suffix memorization beats its single VERB
observation in EWT), so iters=20 / min-count=1 was adopted instead
(dev 91.70%, canonical intact). See AGENTS.md.

## In-domain oracle data (committed)

Oracle labels live in-repo under `data/` (Moby-Dick is public domain;
unlike the NC-licensed UD treebanks it can ship here):
`moby-oracle-01.conllu` (14 sentences, ch.4–6) and
`moby-oracle-02.conllu` (10 sentences, ch.5–6), hand-tagged EWT-side
per CANONICAL.md and disjoint from the ch.1–2 prose eval. Reproduce:

```sh
scripts/fetch-ud.sh --dir /tmp/ud ewt
cat /tmp/ud/ewt/en_ewt-ud-train.conllu data/moby-oracle-*.conllu \
  > /tmp/ud-oracle/en_ewt-ud-train.conllu
cp /tmp/ud/ewt/en_ewt-ud-{dev,test}.conllu /tmp/ud-oracle/
cargo run --release -p english-pos-train -- --corpus /tmp/ud-oracle \
  --iters 20 --min-count 1
```

`--correct` reports accuracy with the correction rules applied
(`tag_margins` + `apply_rules`), including fire counts and a
first-N fire printer on stderr — the measurement harness for rule
admission (no rules ship yet, so it currently reports 0 fires).

Measured (EWT dev/test gates, Moby prose eval at bar 0.84):

| setup | EWT dev | EWT test | Moby prose |
|---|---|---|---|
| EWT base (20,1) | 91.70% | 91.79% | 87.3% (24 miss) |
| + joint oracle (354 tok, committed) | 91.84% | 92.05% | 85.7% (27 miss) |
| + finetune oracle, 3 iters | 91.88% | 91.99% | 85.7% (27 miss) |

Joint training was adopted over fine-tuning: dev differs by 10
tokens (noise), test favors joint, and the protocol is a single
reproducible train run rather than an accidental two-stage. Moby
dips 24→27 misses in both variants — boundary churn on a 189-token
eval (possessive-`have`, long-distance `does`, `the`-PRON wobbles),
while EWT gains on 50k tokens; the eval is too small to resolve ±3
tokens, so EWT rules. `--finetune` mode stays available for future
bounded top-ups. Next oracle batches should target the stable miss
classes (titlecase OOV, preposition chains, `-s`/imperative verbs)
with more examples per class. Oracle discipline learned the hard
way: never contradict EWT-majority on frequent words (checked
sentence-initial `So`→ADV 88:0 and `open`→ADJ 31:17 before keeping
those labels).

## Cross-genre standing (GUM test, gold)

Committed weights measured per GUM genre (split its test file by
`# meta::genre`; EWT test 92.05% for reference). GUM-side conventions
(predicative participles →VERB, discourse-`like`→INTJ — see
CANONICAL.md J2/J4) count as errors here, so this table mixes real
gaps with known disagreements:

| genre | acc | | genre | acc |
|---|---|---|---|---|
| conversation | 93.36% | | essay | 92.92% |
| podcast | 92.17% | | whow | 91.96% |
| news | 91.80% | | academic | 91.75% |
| vlog | 91.73% | | speech | 91.38% |
| interview | 91.11% | | letter | 90.97% |
| court | 90.65% | | fiction | 90.64% |
| voyage | 89.84% | | textbook | 89.14% |
| bio | 88.92% | | | |

Weakest (bio/textbook/voyage) is OOV-heavy specialist vocabulary;
fiction trails EWT by 1.4 points. This is the baseline any
domain-data or Brill-rule work moves deliberately.

## More data (tried, rejected)

`fetch-ud.sh` also builds `ewt+gum`, `ewt+lines`, and `all` presets,
but naive concatenation hurts (all at iters=20/min-count=1, EWT
dev/test as gates):

| train | EWT dev | EWT test |
|---|---|---|
| ewt (committed) | 91.70% | 91.79% |
| ewt+gum | 91.18% | 91.10% |
| ewt+lines | 89.63% | 89.86% |
| all | 90.27% | 90.19% |

The treebanks contradict each other on specific conventions, so the
perceptron averages them into worse-than-either. Confirmed at word
level (same forms, opposite tags): LinES tags honorifics/titles as
NOUN (`Mr` 50×, `Mrs` 23×, `President` 13×, `Jews` 14×, `Lord` 6×)
where EWT/GUM tag them PROPN — LinES is the outlier, EWT and GUM
agree. Mid-sentence Titlecase PROPN:NOUN ratios tell the same story
(EWT 3.7:1, LinES 2.2:1, GUM 13.8:1 the other way). Largest
EWT+LinES top-confusion deltas on EWT dev: PROPN→NOUN +237, AUX→VERB
+60 (LinES main-verb be/have uses check out clean, so this one is
still untraced — likely subtler auxiliary contexts), plus DET/PRON
and ADP/ADV drift. GUM is milder
(-0.5) but still negative, including on its own test (90.64% vs
91.37% for the EWT-only model). Keep EWT-only until a
per-construction harmonization exists; the fetch script preserves
reproducibility for that work.

Notes:

- Plain (unaveraged) perceptron, deliberately: on fast-converging
  data, Collins averaging shrinks settled weights into noise while
  rarely-updated rare patterns dominate decoding (measured 33% vs
  88% dev on a 300-sentence pilot). Final-iteration weights win here.
- Feature templates live in `english-pos` and are shared verbatim
  with inference; changing them invalidates the committed weights.
- Tokenization follows UD (contractions split); see the `english-pos`
  README for the mismatch with `english`-crate words.
