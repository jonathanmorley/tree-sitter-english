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
dev 91.70%, test 91.79%, 1.98 MB). Re-evaluate committed weights any
time with `--eval-only test` (or `dev`). Hyperparams are dev-selected:
a sweep over iters {5,10,15,20,30} × min-count {1,2} peaked at
iters=15 / min-count=1 (dev 92.09%) but flipped canonical "flies"
VERB→NOUN (suffix memorization beats its single VERB observation in
EWT), so iters=20 / min-count=1 was adopted instead (dev 91.70%,
canonical intact). See AGENTS.md.

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
