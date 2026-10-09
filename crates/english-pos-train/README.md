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
not a mechanics artifact. Standing tally: distillation 0-for-5,
mechanics 1-for-4 (guessed-history rejected, tagdict admitted),
char 0-for-2, clusters 0-for-1. The tagdict fast path (below) is the
first weight change since the joint protocol to pass every gate —
it wins by restoring memorization rather than reshaping shared
priors, which is why it escapes the structural trade.

Guessed-history training (MECHANICS REJECTED 2026-10-07, third
mechanics — the Honnibal REAL FIND): `Model::train_guessed_history`
(history advances from the predicted tag, updates still toward
gold) + trainer `--guessed-history`, canonical joint protocol
(EWT+01+02, iters=20/min-count=1). The discipline's most
interesting negative result in a while, because the theory is
sound and the measurement goes the other way:

| path | dev | test |
|---|---|---|
| committed greedy | 23096 (91.84%) | 23100 (92.05%) |
| guessed greedy | 23063 (91.71%, −33) | 23043 (91.83%, −57) |
| committed production | 91.94% | 92.22% |
| guessed production | 23081 (91.78%, −0.16) | 23050 (91.85%, −0.37) |

Gate fails at greedy on both splits; production (beam-2 + all 14
rules) halves the dev gap to −0.16pp but test stays −0.37pp
(−91 tokens). Book evals
not run — the EWT gate failed first, and no book gain can un-fail
a −91-token test split. Forensics, two parts. First, the mismatch
math: inference-time history matches gold 92% of the time (the
tagger is 92% accurate), so gold-trained history features fire
correctly in the overwhelmingly common case; the 8% mismatch
costs less than training on noise. Second, perceptron dynamics:
updates fire only on mispredicts, and early-iteration guesses are
the noisiest — history-induced mispredicts drag LEXICAL rows
around (top guessed confusions shift toward noun sinks:
PROPN→NOUN 479, VERB→NOUN down only to 95 while NOUN→VERB stays
61), while gold history gives clean tag-bigram signal from
iteration 0. Honnibal's warning assumes the history features are
the fragile part; here the lexical rows pay for history noise
instead. The beam span counts corroborate: guessed production
spans 427/2001 dev sentences vs the mechanism needing MORE
re-decode, not less. Code kept (`train_guessed_history`,
`--guessed-history`) as measured infrastructure; weights
restored (`56082361`, md5-verified; rejected artifact
`17c37c3d` in /tmp only).

Tagdict inference fast path (ADMITTED 2026-10-07 — Honnibal (b),
and the largest single EWT jump since the joint protocol):
`Model.tagdict` (lowercased words seen under exactly one tag in
training: 14,563 entries) + `decode_lower` skip with a gate-inert
placeholder margin (`FAST_PATH_MARGIN`, finite for the
`tag_margins` contract) + `tagdict` key in the weights JSON
(1.76→1.98 MB). Retrained under the canonical protocol; weight
values bit-identical to a tagdict-less retrain (0/30,360 rows
differ), so every delta below is the fast path at inference:

| path | dev | test |
|---|---|---|
| committed greedy | 23096 (91.84%) | 23100 (92.05%) |
| tagdict greedy | 23171 (92.14%, +75) | 23250 (92.65%, +150) |
| committed production | 91.94% | 92.22% |
| tagdict production | 23187 (92.20%, +~67) | 23290 (92.81%, +~149) |

Book evals: moby 27→27, genre 20→20 (both neutral, miss lists
entry-identical in count), hard 0.8644→0.8741 (+~15 tok), sweep
greedy 1827→1839 (+12) / production 1843→1853 (+10) with zero
rule breaks, `flies` holds on all three decode paths, full
workspace 123 green. Train-corpus diff (banked vs new weights,
204,932 tok): 1,179 greedy / 1,109 production flips (0.6%), every
sampled flip correct-direction (initialisms→PROPN, gerunds→VERB,
`dominant`→ADJ, `effect`→NOUN). Speed: tag pass 101.8→87.2 ms
(−14%, 25.8% skip rate on Moby body).
Why it helps (against the old "locks the wrong tag early"
rejection, which concerned AMBIGUOUS words in beam spans):
single-tag-in-train words are memorization, and full decode was
overriding memorized readings with context noise — the dict
restores them. The old rejection stands for ambiguous words
(`this`/`that` never enter the table). Flip census on the train
corpus (banked vs new weights, 204,932 tok; probe since deleted):
1,179 flips, 1,151 correct (97.6%). Mechanism, measured three ways.
(1) Rare words: flip mass by train count [1, 2–3, 4–10, 11–50,
51+] = [326, 295, 269, 203, 86] — weak word-identity rows lose to
dense context features that outvote them (context steamrolling).
(2) Confident errors: mean baseline margin at flips 5.61 — far
above every gate (τ ≤ 5.0) and beam threshold, so no correction
rule and no re-decode span could ever reach them; the tagdict is
the only layer that touches the confident-mistag class (the same
class the joint and margin probes measured at capture 0.139 from
below). (3) Directions concentrate on the known hard classes:
NOUN→PROPN 205, PROPN→NOUN 174, NOUN→ADJ 146, NOUN→VERB 143
(gerunds), PROPN→ADJ 78, VERB→NOUN 71. The 28 wrong flips (2.4%)
are the price, bounded and sampled. This also reconciles the
rejected Honnibal probe: training on guessed history hurt
because early-iteration noise drags lexical rows, while
constraining inference to memorized tags helps because these
errors are systematic context bias, not noise — opposite
interventions for opposite failure modes.
- Beam re-measured under dict (DONE 2026-10-07, keep): greedy+rules
  vs beam+rules = dev 23199→23187 (−12), test 23271→23290 (+19).
  Pre-dict the marginal was +25/+17; the dict shrank beam's role
  (fewer/weaker spans) but split the splits, so removal fails the
  change-gate on test (−19) and beam stays. NOT deleted.
- Queued: (1) the 51+ count flips (86 — frequent words the dict
  still overrides; distinct phenomenon, possibly model bug or EWT
  quirk); (2) the 28 wrong flips (titlecase-forcing suspect — a
  guard needs its own EWT-majority measurement, color-adj
  discipline); (3) beam-dict consistency (DONE 2026-10-07,
  REJECTED, code fully reverted): forcing dict tags inside
  re-decoded spans measured dev 23187→23197 (+10) but test
  23290→23284 (−6) — a wash trading test for dev, failing the
  change-gate; beam stays as-is. Fixes outnumber breaks in both
  splits (32/26 diffs), but the breaks concentrate exactly where
  suspected (titlecase PROPN forcing: `War`/`PHB`/`Margin`/`Call`
  →NOUN, `laudatory`→VERB), while the fixes (`'s`→PART,
  `heard`→VERB, `political`→ADJ) don't generalize into a rule.
  Probe plumbing reverted, hot decoder untouched; (4) per-class
  weighting for rare-word identity rows (the census hands the
  queued distillation idea a concrete target).
- Follow-up closures (DONE 2026-10-07, probes since deleted):
  (1) 51+ flips are direct dict hits, all correct — no model bug,
  no EWT quirk (spot-check confusion resolved: `different` is
  58× ADJ in train, so NOUN→ADJ flips are memorization restored,
  not damage). (2) Wrong flips (~2 dozen on 205k train tokens)
  are cascade damage on ambiguous function words (`that`/`have`/
  `of`/`in`/`'s`/`with`/`to`, 2–4 each, mixed with correct
  cascades like `review`/`american` 3/3 and 2/2) — no guard
  shape has EWT-majority support, so no code; the price stays
  bounded and measured. Direct-vs-cascade split was the load-
  bearing distinction (a first cut conflated them). Two implementation scars
worth recording: (1) the first table was a sorted vec — binary
search cost MORE than scoring on misses (tag pass 102→148 ms);
a hash map fixed it; (2) the `U64Hasher::write` fallback REPLACED
state instead of folding, collapsing all String keys into one
bucket (52µs/lookup); folding fixed it (226 ns). Both caught by
bench, not review. `finetune`/`finetune_frozen` clear the table
(weight updates can move a word off its dict tag); smoke tests
pin the placeholder-margin contract deliberately.

## Ensemble-averaged weights (committed 2026-10-09, K=15 since)

Training is fully deterministic (zero init, fixed data order, no
RNG) — so a single run bakes in its file order's recency bias.
`examples/ensemble.rs` (kept as the committed recipe) trains one
baseline plus K members on LCG-shuffled orders (Fisher-Yates,
`6364136223846793005`/`1442695040888963407`, seeds fixed) and
averages the member maps entrywise at the JSON level into ONE map
(same decode shapes; kept recipe steps: members via `ensemble`,
mean via `scripts/average-members.py`, prune via
`scripts/prune-weights.py`; every step re-derived md5-identical).
Tagdicts are order-independent by construction
(first-seen tag only sticks when unanimous) and asserted equal —
an assert that once caught a real contamination (oracle-pool
members mixed into an EWT averaging set: same-pool tagdicts
cannot differ, so the failure proved mislabeled inputs, and
mtime forensics confirmed the overwrite order).
Committed (2026-10-09 evening): entrywise MEDIAN of K=15
(seeds 1–15), iters=20, min-count=1, EWT train only — no
oracle data, no prune step (medians cancel symmetric
disagreement exactly, so the map is naturally sparse at
1.87 MB). Recipe: members via `ensemble`, combination via
`scripts/average-members.py --median` (both md5-verified
end to end). Median ignores outlier members the way a vote
does: per-token-majority vote over the 15 greedy decodes
measured +115/+69 over the mean with identical test score
to the median (23558/23616 vs 23545/23616) — the map ships,
the 30 MB vote does not.
Yield curve by K (greedy; members alone +59…+195): K=1 91.99 /
K=3 93.22 / K=9 93.63 / K=15-mean 93.70 / K=15-median 93.63
dev (test 92.48 / 93.74 / 94.14 / 94.10 / 94.11) — the median
trades a hair of dev for the mean (-19) at identical test.
Superseded same day: mean+prune-0.34 (shipped that morning:
93.77/94.16 prod, md5 `eeb87c81…`; prune tolerance shrinks
with K — 0.67 costs −15/−14 at K=15, 0.50 fails too).
Member pairwise dev disagreement ~5.4% (the variance being
combined out is real, not identity).

| setup (greedy) | EWT dev | EWT test |
|---|---|---|
| fixed order (EWT-only) | 23133 (91.99%) | 23206 (92.48%) |
| shuffled members (15) | +59…+195 | +30…+191 |
| entrywise mean of 3 | 23443 (93.22%, +310) | 23522 (93.74%, +316) |
| entrywise mean of 15 | 23563 (93.70%, +430) | 23622 (94.13%, +416) |
| entrywise median of 15 | 23545 (93.63%, +412) | 23616 (94.11%, +410) |

Production (beam-2 + 17 rules) for committed median: dev
23557 (93.67%), test 23622 (94.13%, coarse 95.99); PUD test
19759/21180 (93.29%). Books: sweep greedy 1896 / production
1902 (+8/+15 over mean-shipped), hard 0.9064, Moby/genre miss
counts 21/14, chunk sent/token mixed within noise, lint bars
all hold with five TP/FP improvements over mean-shipped
(coordscope +1TP/−1FP, passive +1TP, vague −1FP, weasel +1TP,
nominal +1TP; hard +6toks too — nothing moves backwards). Flip census vs committed:
624 fixes / 292 breaks, no concentration, X at background
rate. Median weights md5
`ca29d68ca32d2febdc8257c9bcfc0668` — the committed file
(1.87 MB); Moby batch/stream parity 8881/8881; site
numbers/table rebuilt from them (94.13/95.99, wasm bundle
2.4 MB). Queued, not started: oracle-joint on top of the
ensemble protocol (one variable at a time).

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

## Cross-treebank standing (PUD test + GUM refresh, gold)

PUD (UD_English-PUD r2.18, CC BY-SA, news+wiki, 1,000 sents /
21,180 words, fetched like EWT — see `scripts/fetch-ud.sh`):
the fourth UD English treebank and the first non-EWT gold check
on the dependency stages. GUM test re-measured with current
weights beside it (28,397 words):

| split | tag exact | tag coarse | UAS gold | UAS pipe | LAS gold | LAS pipe |
|---|---|---|---|---|---|---|
| EWT test | 92.81 | 94.99 | 83.9 | 77.8 | 80.2 | 71.3 |
| PUD test | 91.56 | 93.74 | 79.3 | 73.2 | 75.2 | 65.7 |
| GUM test | 91.77 | 93.74 | — | — | — | — |

PUD is uniformly ~1–5 points harder (longer news sentences), and
the cascade tax is EXACTLY preserved: gold→pipeline UAS −6.1 on
both EWT (83.9→77.8) and PUD (79.3→73.2) — independent
confirmation that the cascade is structural, not EWT-shaped.
Labeler ceilings (gold 93.31 / pipeline 85.59) show the same
compression. No gates change: bars were set on EWT; PUD is a
standing second opinion, recorded here. Competitor rows on PUD
(NLTK coarse 91.06, Pattern 92.12, spaCy 95.27/96.61 @96.9%
aligned) live in the harness TSV, not the table — same-domain
caveats as EWT.

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
