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
dev 91.84%, test 92.05%, 1.76 MB — EWT plus in-domain oracle data
below, at iters=20/min-count=1; md5 `56082361`, integer-encoded
since 2026-10-06). Re-evaluate committed weights any
time with `--eval-only test` (or `dev`). Base hyperparams are
dev-selected: a sweep over iters {5,10,15,20,30} × min-count {1,2}
peaked at iters=15 / min-count=1 (dev 92.09%) but flipped canonical
"flies" VERB→NOUN (suffix memorization beats its single VERB
observation in EWT), so iters=20 / min-count=1 was adopted instead
(dev 91.70%, canonical intact). See AGENTS.md.

## In-domain oracle data (committed)

Oracle labels live in-repo under `data/` (Moby-Dick is public domain;
unlike the NC-licensed UD treebanks it can ship here):
`moby-oracle-01.conllu` (14 sentences, ch.4–6),
`moby-oracle-02.conllu` (10 sentences, ch.5–6), and
`moby-oracle-03/-04.conllu` (24/20 sentences, ch.7–26/ch.3+27–30;
03 and 04 REJECTED as training data, KEPT as documented data),
hand-tagged EWT-side per CANONICAL.md and disjoint from the ch.1–2
prose eval. Reproduce:

```sh
scripts/fetch-ud.sh --dir /tmp/ud ewt
cat /tmp/ud/ewt/en_ewt-ud-train.conllu data/moby-oracle-01.conllu data/moby-oracle-02.conllu \
  > /tmp/ud-oracle/en_ewt-ud-train.conllu
cp /tmp/ud/ewt/en_ewt-ud-{dev,test}.conllu /tmp/ud-oracle/
cargo run --release -p english-pos-train -- --corpus /tmp/ud-oracle \
  --iters 20 --min-count 1
```
(Batches 03/04/05 are spelled out — never the `moby-oracle-*` glob:
all three were measured, rejected, and must stay out of training.)

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
bounded top-ups.

Distillation 0-for-3 (all rejected, weights restored every time):
batch 03 joint (751 tok) dev −225 / test −257 — targeted
confusions fixed, shared priors dragged (VERB→AUX +114 from
AUX-heavy be-forms, VERB→ADJ +107 from ADJ-participles);
batch 03 finetune ×3: dev −60 / test −188 (bounded drift still
drifts, test-heavy book-prior overfit); batch 04 joint (532 tok,
boundary-balanced: passive be+AUX with VERB participles,
reduced relatives, noun-noun compounds, fresh 3sg — zero new
ADJ-participles, won boundaries untouched): dev −218 /
test −257, the same collapse. Lesson: balance didn't save it —
oracle mass past ~350 tokens shakes shared priors no matter the
composition (EWT-majority discipline held throughout; even the
suspect `For`-ADP calls check out 40:1). The joint protocol that
worked at 354 tokens does not scale by adding more; next attempt,
if any, must change the training mechanics (per-class weighting?
frozen shared priors?), not the batch. Next oracle batches should target the stable miss
classes (titlecase OOV, preposition chains, `-s`/imperative verbs)
with more examples per class. Oracle discipline learned the hard
way: never contradict EWT-majority on frequent words (checked
sentence-initial `So`→ADV 88:0 and `open`→ADJ 31:17 before keeping
those labels).

Batch 03 (REJECTED 2026-10-05): 24 sentences / 751 tokens
(`data/moby-oracle-03.conllu`, ch.7–26, disjoint from eval and
batches 01/02), per-class-targeted at the stable miss classes (3sg
`-s` verbs, `that` in all three constructions, imperatives,
titlecase PROPN, participial ADJ vs reduced-relative VERB) with an
EWT-count check behind every contested call (ambiguous items like
`All dressed`, `round/yonder`, `to`+gerund dropped, not guessed).
Joint retrain (EWT + 01/02/03, iters=20/min-count=1): dev
91.84→90.95 (−225), test 92.05→91.03 (−257). Weights restored
(`712e0fc7`), data kept. Every targeted confusion improved
(SCONJ→PRON 57→3, ADJ→NOUN 106→58, NOUN→PROPN 158→109,
ADP→SCONJ 40→29) but the batch's auxiliary observations dragged
shared priors the other way: VERB→AUX 52→166 (all 12 batch
be-forms tagged AUX — the batch starves EWT's infinitival-passive
be-VERB per J2), VERB→ADJ 48→155 (12 ADJ-participles vs EWT's 22
VERB predicatives), NOUN→ADJ 52→121, PROPN→NOUN 263→308,
PART→ADP 17→61. Same shared-prior-drift signature as the ch.36
rejection, different classes: targeting works at the confusion
level, but joint training lets the batch's majority classes pull
untargeted boundaries. Next: counter-observe starved classes
(be-VERB, VERB participles) for boundary balance, or try
`--finetune` bounded top-ups; lexicon backoffs (no weight change)
stay the fallback.

Finetune variant (REJECTED 2026-10-05): committed weights +
batch 03 only, 3 passes: dev 91.60% (−60), test 91.30% (−188);
weights restored. Bounded drift still drifts — and test-heavy
(−188 vs dev −60), the book-prior overfit signature. Distillation
at this scale is now 0-for-2 (joint + finetune), the same shape
as the correction layer's 0-for-2 with morphology rules. Next
distillation attempt needs a different kind, not a different
size: boundary-balanced batches, or per-class targeting with the
starved classes counter-observed in the same batch. Otherwise the
roadmap's no-weight-change steps (lexicon backoffs, beam-2
re-decode of low-margin spans) go first.

Batch 05 (REJECTED 2026-10-06): 5 sentences / 226 tokens
(`data/moby-oracle-05.conllu`, ch.32–37 — cetology narrative +
Ahab's Quarter-Deck speeches, disjoint from evals and batches
01–04), per-class-targeted at verb morphology only (five 3sg
`-s` verbs in one sentence, four imperatives, bare/base
contrasts, passive vs attributive participles), EWT-count checks
in-file (`touching`→VERB 2:0 kept over the prepositional
reading, `-able`→ADJ 308:8, `yonder`→DET by guideline,
relative-`that`→PRON, generic `Emperors/Kings`→NOUN per M1,
`Captain`→NOUN by in-repo lawyer-precedent; S3 dropped as
lowest verb value to hold ~200 tokens). Joint retrain (EWT +
01/02/05, iters=20/min-count=1): dev 91.84→91.42 (−106), test
92.05→91.66 (−98). Weights restored (`712e0fc7`), data kept.
The first distillation run where EVERY eval improved — Moby
27→23, genre 20→13, hard 0.8644→0.8883, genre-chunk cascade
9/20→13/20 sent (token 0.873→0.931), `flies` veto held — while
EWT dropped 10×-noise on both splits: the exact mirror of batch
01/02 (there EWT +tiny / Moby −3; here EWT −100 / evals +48).
Same shared-prior-drift signature at smaller magnitude —
targets won (dev VERB→NOUN 133→118, NOUN→PROPN 158→102,
SCONJ→PRON 57→14, ADJ→NOUN 106→82) while priors dragged
(PROPN→NOUN 263→313, NOUN→VERB 85→116). Composition finding:
verb-heavy 226 drags NOUN/VERB/AUX priors both ways where the
mixed 354 of 01/02 held — the drift floor depends on batch
shape, not just mass. Per the 01/02 precedent (EWT rules on
conflicts) and the dev/test gates, REJECTED. Distillation now
0-for-4; the roadmap stands — accuracy work needs new training
mechanics (per-class weighting, frozen priors), not new
batches. One live hypothesis for a cheap probe: a sub-100-token
micro-batch of the densest verb material only (S1+S4+S5 ≈ 96
tok), testing whether drift has a floor the 226 exceeded.

Micro-batch 06 (REJECTED 2026-10-06): exactly that probe — S1,
S4, S5 of batch 05 only (96 tok, no new labels). Joint retrain:
dev 91.84→91.14 (−202), test 92.05→91.28 (−193). SMALLER was
WORSE: composition dominates mass entirely (the five-3sg S1
poisons NOUN↔VERB both ways: NOUN→VERB 85→130; ADP→ADV
90→171 from infinitival as/to/for contexts; NOUN→ADJ 52→108;
targets still won: VERB→NOUN 133→106, SCONJ→PRON 57→43).
Weights restored (`712e0fc7`). Distillation now 0-for-5, and
the floor hypothesis is dead with it — no batch shape is safe
under joint training; only new mechanics (per-class weighting,
frozen priors) remain on the roadmap.

Frozen-prior finetune (MECHANICS REJECTED 2026-10-06, first new
mechanics tried): `Model::finetune_frozen` (per-feature masking —
features with base-corpus count ≥ K never update; novel/rare rows
still learn) + trainer `--freeze-at K`, run on oracle batch 05
(226 tok, 20 passes, base = committed EWT+01+02 corpus):

| K | dev | test | moby | genre | hard |
|---|---|---|---|---|---|
| committed | 23096 | 23100 | 27 | 20 | 0.8644 |
| 2 | 23096 | 23100 | — | — | — |
| 5 | 23096 | 23100 | — | — | — |
| 20 | 23096 | 23100 | 27¹ | 20 | 0.8644 |
| 100 | 23092 | 23102 | 27 | 20 | — |
| 500 | 23082 | 23105 | 27 | 20 | 0.8683 |
| ∞ (plain joint) | 22990 | 23001 | 23 | 13 | 0.8883 |

¹moby miss list entry-identical to committed, not just count-equal.

Zero drift at K≤20 (bit-identical EWT) — the mechanism works —
but zero gain everywhere it matters: every frozen K leaves all
book evals untouched, while plain joint moves them ±. K=500 buys
hard +0.004 (+6 tok) for dev −14: pure cost, rejected by the
gates like everything else. Flip-forensics cross-tab (plain-05
fixes × EWT word counts) explains why: ~half the gains ride
ultra-shared rows (`that`×3, `have`, `of`, `this`, `the`, `does`,
`take`, `look`, `after`, `next` — EWT counts 100–9,075, frozen
at every K), and the rare-word half (`watery`, `pistol`,
`landsmen`, `approve`, `Exports`, `improves`, `predicts`,
`markets`, `demands` — counts 0–10) never materializes under
freezing either, because perceptron updates fire on mispredicts:
frozen shared rows change mid-training predictions, so rare rows
learn different weights than in plain joint. Gains and damage are
*dynamically* entangled, not just statically shared — no masking
threshold can separate them. Per-class weighting is the only
mechanics left untried, and this result predicts its shape: it
must apply EWT counter-pressure on the same shared rows, not
avoid them. Code kept (`finetune_frozen`, `--freeze-at`) as
measured infrastructure; weights restored (`56082361`).

Counterweighted joint (MECHANICS REJECTED 2026-10-06, second
and last untried mechanics): data-only, zero code changes —
12 contested words where oracle-05 contradicts the EWT majority
(`that` SCONJ→PRON, `to` PART→ADP, `as`, `more`, `before`,
`what`, `do`, `judge`, `living`, `sinking`, `thought`, plus `'`
PUNCT→PART), and the 33% of EWT-train sentences containing one
under its majority tag duplicated k× in the joint corpus, so
shared rows feel EWT counter-pressure proportional to the
oracle pull:

| k | dev | test | moby | genre | hard |
|---|---|---|---|---|---|
| committed | 23096 | 23100 | 27 | 20 | 0.8644 |
| 2 | 23137 | 23049 | 15 | 16 | 0.8838 |
| 3 | 23049 | 22984 | — | — | — |

k=2 is the mechanism's high-water mark — moby 27→15 beats even
plain joint (23), hard +0.02 — but test −51 fails the gates by
25× precedent, and k=3 collapses both splits. The dev/test
split (dev +41 yet test −51 at k=2) is the tell: the duplicated
function-word priors fit dev-adjacent contexts while hurting
web-text broadly — exactly what the EWT gates exist to catch.
Across all operating points (frozen K≤500, counterweight k=2/3,
plain joint) book gains scale with web-test damage and no point
passes both splits: the trade is structural at this capacity,
not a mechanics artifact. Accuracy roadmap now stands:
distillation 0-for-5, mechanics 0-for-2, char 0-for-2, clusters
0-for-1 — the linear ceiling is holding on every axis with a
measurement behind each. Remaining accuracy work is the
no-weight-change column only (lexicon backoffs beyond the 3
shipped, beam refinements). Weights restored (`56082361`).

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
