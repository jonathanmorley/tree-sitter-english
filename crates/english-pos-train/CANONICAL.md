# Canonical UD decisions

Goal: a harmonized English training set ("real canonical dataset") for
`english-pos`, built from UD EWT + GUM + LinES with documented,
evidence-backed calls wherever the treebanks contradict each other.

Decision model: the human operator declined the oracle role (domain
expertise), so the agent decides from primary sources — UD guideline
text (universal + en-specific), treebank changelogs, 2-of-3 majority —
and records rationale + confidence per item. Every call is vetoable;
nothing here claims linguistic authority beyond its cited evidence.

Provenance notes that shape the calls:

- GUM UPOS tags are **converted from manual XPOS**, not hand-annotated
  as UPOS: systematic divergences (participles→VERB, `United`/VERB)
  read as conversion-rule artifacts. GUM's changelog shows active
  harmonization toward EWT (2022-10 "updates to UPOS consistency with
  EWT") and introduced `nmod:desc` for titles (2025-03) — the same
  relation EWT/LinES already use for honorifics.
- LinES stands alone against EWT+GUM on honorifics, `ago`, `else`,
  attributive `another`, and `of course`: project-specific conventions
  (or older conversion), not guideline readings.
- EWT is the guideline-home treebank and the test gate, but is itself
  stale where guidelines moved on (temporal `when` + `advmod`).

Method: word-form UPOS distributions compared across the three trains
(miner: total ≥ 20, present in ≥ 2 treebanks, ranked by count × max
pairwise L1 distance), then examples pulled for every candidate and
checked against the guidelines above. Naive concatenation was tried
first and rejected (EWT+GUM −0.5, EWT+LinES −2.1 on EWT test).

Status values: `accepted` (agent decision, vetoable), `rejected`
(keep the split), `deferred` (no safe rule). All calls below are
agent decisions from cited evidence, not linguistic authority.

Provenance (from treebank docs): LinES UPOS is **automatically
converted** from manual XPOS with only partial train review ("there
may still be some errors remaining", "occasional deviations from the
general guidelines" — README.txt) — its divergences read as mapping
artifacts. GUM UPOS is likewise converted-from-manual, but GUM
actively harmonizes toward EWT per its changelog.

## Mechanical conversions (syntax- or word-determined)

| ID | Item | Evidence | Canonical | Status |
|---|---|---|---|---|
| M1 | Honorifics/titles in LinES: `Mr` 50×, `Mrs` 23×, `President` 13×, `Jews` 14×, `Lord` 6× NOUN; EWT/GUM PROPN | Same forms, opposite tags; syntax agrees (`nmod:desc` 79 vs 73) | PROPN via rule: Titlecase + `nmod:desc` + honorific/title form → PROPN (standalone titles like `the President`/nsubj stay NOUN, matching EWT's own mix) | accepted |
| M2 | `ago`: LinES ADP 18× vs EWT/GUM ADV 87×, unanimous otherwise | Word-determined | ADV | accepted |
| M3 | `else`: LinES ADJ 18× vs EWT/GUM ADV 98× | Word-determined | ADV | accepted |
| M4 | Attributive `another`: LinES ADJ (`another XML document`) vs EWT/GUM DET; en/pos/DET lists `another` as DET lexeme | Guideline-explicit | DET via rule: `another` + nominal next → DET, else PRON | accepted |
| M5 | `of course`: LinES `course`/ADV 24× vs EWT/GUM NOUN | Bigram-determined | NOUN via rule on `of course` | accepted |
| M6 | GUM `United`/VERB/amod 91× (`United Kingdom`, affiliations) vs EWT ADJ 81× | `amod` VERB indefensible in a name | ADJ via rule: `United` + `amod` + {Kingdom, States, Airlines, Nations} → ADJ | accepted |

## Judgment calls (agent decisions, vetoable)

| ID | Item | Evidence | Decision |
|---|---|---|---|
| J1 | Name parts (`New` in `New York`, `North`/`South`): EWT ADJ (73×) vs GUM PROPN (33×); universal PROPN page tolerates both (PTB practice vs Czech-style) | 1v1 + guideline ambiguity; weakest call here | ADJ (EWT primacy; keeps EWT-test; consistent either way for Moby's `New Bedford`) |
| J2 | Predicative participles (`interested`, `concerned`…): EWT 138 ADJ/22 VERB vs GUM 24 ADJ/134 VERB | Both defensible; conversion needs copula-context syntax, risky wholesale | DEFER (no harmonization; revisit with syntactic rules) |
| J3 | Temporal `when`: EWT+GUM ADV+`advmod` (578×) vs LinES SCONJ+`mark` (106×, guideline-correct) | Majority + test gate vs guideline text; rewriting EWT's 304 temporal-whens out of scope | ADV (status quo), tension noted |
| J4 | Discourse `like`: EWT ADP vs GUM INTJ 219×; en/pos/INTJ (v1) = PTB UH, no discourse-marker mandate | 2-of-3 + en-guideline favor ADP | ADP (status quo) |
| J5 | `others`: EWT/GUM NOUN (94×) vs LinES PRON (17×); DET guideline lists `all`/`both` but is silent on `other` | Majority, no guideline text | NOUN (status quo) |

## Accepted as unresolvable by conversion

- `had` AUX/VERB priors (EWT 68% VERB possessive-`had` vs LinES 81%
  AUX pluperfect narration): both treebanks correct, purely
  distributional. Context features (`w+1` participle vs determiner)
  should disambiguate; no conversion possible.
- GUM `'s`/VERB + `'s`/PRON: `let's` → PRON (correct) and disfluency
  noise (`There's a- in there`); small counts, no rule.
- `5m` unit: EWT `m`/NUM vs GUM `m`/NOUN; small counts, no rule.

## Validation protocol

Harmonizer: `scripts/harmonize-ud.mjs` (node; runs over the
`fetch-ud.sh` cache, reports per-rule conversion counts). It implements
exactly the accepted M1–M6 rules plus the J3 `when`→ADV conversion.
Deliberately unconverted: `Uncle` (no EWT PROPN precedent — EWT's own
single `Uncle` is NOUN), standalone-title and common-noun-position
ethnonyms (`President` vocative/nsubj, `Jews` nsubj/obj — EWT itself
mixes these), GUM name parts and participles (J1/J2 deferred),
discourse-`like` (J4 status quo).

Measured (iters=20/min-count=1, EWT dev/test gates):

| train | EWT dev | EWT test |
|---|---|---|
| ewt (committed) | 91.70% | 91.79% |
| ewt + harmonized gum | 91.28% | 91.03% |
| ewt + harmonized lines (M1–M6 only) | 89.83% | 89.79% |
| ewt + harmonized lines (+J3 when, 106×) | 90.03% | 89.97% |

Conclusion 2026-09-26: mechanical harmonization recovers ~0.4 of ~1.9
points. The remainder is domain divergence (fiction vs web priors:
pluperfect `had`, reporting verbs, dialogue) plus the deferred J-items
(participles, name parts, discourse markers) — none safely
rule-convertible. EWT-only stands; this record plus the scripts
preserve the work for a future literary-tuning pass with a real Moby
eval set. Harmonization lives as a documented transform, never
hand-edited data.
