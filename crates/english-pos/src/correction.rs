//! Brill-style correction post-pass over greedy perceptron output.
//!
//! The perceptron decodes left-to-right with local features, so it has
//! systematic blind spots the training data cannot fix at this model
//! size: 3sg `-s` verbs read as plural nouns (`wears`, `glitters`),
//! and subjectless sentence-initial verbs read as nouns
//! (`Better sleep ...`). Correction rules rewrite tags where lexical
//! shape plus neighbor tags outweigh the perceptron — gated on the
//! decode margin so confident predictions are never touched.
//!
//! Admission discipline (CANONICAL.md): a rule ships only with
//! EWT-majority and oracle-set support measured on EWT dev plus the
//! Moby evals. Gated rules cannot move confident tokens by
//! construction; the gate threshold is calibrated per rule in the
//! trainer (`--correct` reporting) and recorded here.
//!
//! REJECTED (2026-09-27, kept out of `RULES`): an `s-verb` rule
//! (NOUN→VERB for `[a-z]+s$` non-`ss`/-`ness` words after nominals).
//! Measured net-negative on EWT at every threshold (τ=2: dev ±0 /
//! test −2 on margin-0.0 ties `theories`, `status`; τ=8: dev −4 /
//! test −2 across 19 fires) and zero fires on the Moby eval —
//! plural `-ies` nouns and `-us` words share the shape, and only a
//! lexicon (rejected: tagdicts lock the wrong tag early) tells them
//! apart. The engine below stays for rules that pass admission; its
//! tests use a local toy rule.
//!
//! Background: `docs/references.md` (Brill 1992/1995 for the
//! post-pass shape; Collins 2002 for the perceptron).
//!
//! Deliberately NOT here: tagdicts (lock the wrong tag early),
//! stemming backoff (destroys the `-s` signal), more `w+t-1`
//! conjunctions (hurt dev), Collins averaging, HMM EM.
//!
//! Lexicon backoffs (ADMITTED 2026-10-06, 3 of 14): the roadmap's
//! no-weight-change step. Unlike the rejected per-form tagdict
//! (consult-always, locks early), these consult wordlists only below
//! the margin gate, as post-pass predicates through this same
//! engine. Eleven candidates measured and removed (see `RULES`);
//! survivors: `to-prep`, `have-verb`, `to-verb`. Three more were
//! admitted later the same day from margin probes and the
//! external-rules survey: `that-det` (eval-margin probe),
//! `that-rel` + `det-noun` (Brill/fnTBL/RDR/CG convergence),
//! `that-sconj` (RDR tree-mining), `that-ccomp` + `subconj-adp` +
//! `apos-part` (gate-zone autopsy), `to-part` (gate-zone autopsy) —
//! 12-for-24 total. A thirteenth joined later from the lint pilot's
//! participle misses: `pass-by` (be-participle + by-agent barrier,
//! EWT 110:0) — 13-for-25, then 13-for-26 after `color-adj` below
//! was measured and refused. A fourteenth joined from the hedge
//! lint eval's 13/14 FN wall: `quite-adv` (determiner `quite` before
//! ADJ/ADV → ADV, EWT 27:0) — 14-for-27.
//!
//! REJECTED (2026-10-07, removed from `RULES`): `color-adj`
//! (NOUN→ADJ on Berlin-Kay colors with DET-prev + nominal-next).
//! EWT train gold 26:1 justified building; dev fired once and broke
//! it (`a blue box`, USPS-box noun adjunct — net -1, gate fails, no
//! test run un-fails it). Deeper lesson, bank it: EWT-majority was
//! measured on the GOLD shape, but fires happen conditioned on the
//! model saying NOUN — and the tagger predicts NOUN in this shape
//! exactly where noun-adjunct evidence is strongest (`blue box`,
//! `white light`). Shape-majority ≠ fire-precision whenever the
//! base model's errors concentrate on the minority reading; future
//! EWT checks must condition on the predicted tag, not just gold.
//! Same class that killed unguarded attr-adj, twice confirmed.
//!
//! REJECTED (2026-10-07, zero code written): case-frequency titlecase
//! rules from a 6.6M-token book-body rate table (Honnibal (c) probe).
//! Pred-conditioned EWT-train tabulation (89 gated titlecase tokens
//! past position 0): NOUN→PROPN fires 8:2 at rate ≥ 0.1 but 6:5 below
//! it (55% — the tagger predicts NOUN exactly where noun evidence is
//! strongest, color-adj all over again); PROPN→NOUN goes 8:17 against;
//! PROPN→ADJ 11:17 against. Nothing near the 89:1-class bar the
//! shipped shapes hold, and the book misses scatter across buckets
//! (`sabbath` 0.95 vs `midnight` 0.03 — no threshold captures them
//! without the breaks). Deeper lesson, generalizing OOV-list: book
//! titlecase errors are domain-shift (unseen names), not EWT-shaped
//! ambiguity — the rate table describes books well but the rule
//! needs web-gated precision that isn't there. Probe deleted.

use crate::Tag;
use crate::lexicon::{is_oov, known_verb_form, nn_tag};

/// A correction rule: a name plus a predicate over the token stream.
/// Predicates see current predicted tags, lowercased surface pieces,
/// and margins; they return the replacement tag (or `None` to leave
/// the token). Conditions use *predicted* tags (what the decoder
/// actually saw, including its mistakes) — never gold.
#[derive(Clone, Copy)]
pub struct Rule {
    /// Short identifier (`s-verb`, `imperative-0`, …).
    pub name: &'static str,
    /// Margin gate: applies only when the token decoded with margin
    /// strictly inside `(0, threshold)`. Calibrated per rule.
    pub threshold: f32,
    /// The rewrite test. `i` indexes `tags`/`low` (and margins, via
    /// the gate in [`apply_rules`]).
    pub test: fn(&[Tag], &[String], usize) -> Option<Tag>,
}

/// Margin gates are double-bounded `0 < margin < threshold`: exact
/// ties (margin 0) carry no model signal, and shape-only guesses there
/// lose on base rates (measured: `theories`, `status` flip wrong) —
/// a rule may only override a weak-but-present signal, never invent
/// one from silence. Thresholds calibrate per rule in the trainer
/// (`--correct` reporting); Moby evals move only deliberately.
///
/// No rules ship yet: the `s-verb` candidate (NOUN→VERB for 3sg-shaped
/// words after nominals) measured net-negative on EWT at every
/// threshold (τ=2: dev ±0 / test −2; τ=8: dev −4 / test −2) with zero
/// Moby fires — plural `-ies` nouns and `-us` words share the shape.
/// See the rejection note at the top of this module.
///
/// REJECTED (2026-09-27): an `imperative-0` rule (sentence-initial
/// NOUN→VERB for morphology-plausible base verbs with complement
/// next) measured 0% precision — EWT dev fires (`Lifts`, `Dentist`,
/// `someplace`) all gold NOUN/ADV, test net ±0 only because its one
/// fire was already wrong, and threshold ∞ flips ordinary nouns
/// (`Thanks`, `Hundreds`, `Things`) en masse. Morphology without a
/// lexicon cannot beat NOUN base rates at pos-0; the EWT-majority
/// per-form variant is a tagdict and stays rejected. Removed, like
/// `s-verb` above; the engine + gate stay for rules that pass.
///
/// All shipped rules, in application order. A rule ships only with
/// EWT dev ≥ 0 and test ≥ 0 measured plus Moby/genre deltas recorded;
/// rejects get removed (see below).
///
/// ADMITTED 2026-10-06 (τ=2.0 throughout): `have-verb` (EWT dev +2,
/// test +1 — possessive-`have` fixes), `to-prep` (dev +3, test ±0 —
/// prepositional-`to` before nominals), `to-verb` (dev +1, test ±0 —
/// infinitive heads). Combined: dev +6, test +1; Moby/genre/chunk Δ
/// 0 — the evals' remaining misses are confident (margin ≥ τ) or
/// exact ties (margin 0, blocked by design), so the gate cannot
/// reach them; recorded, not assumed.
///
/// ADMITTED 2026-10-06 (`that-det`, τ=2.0): determiner-`that`
/// after ADP/sentence-start before ADJ/NOUN (EWT gold DET 89:1;
/// VERB/ADV/DET/NUM/PROPN-next stay out per measured splits).
/// EWT ±0 (zero fires both splits — the shipped three carry the
/// +6/+1); production-path evals fix 2 (moby-sample `That`,
/// hard `that`+NOUN) with zero new breaks (244 confident / 8 ties
/// unchanged). Fourth rule; the gate plus EWT measurement did
/// their job again.
///
/// REJECTED 2026-10-06 (measured, removed): `s-verb-lex` (dev ±0:
/// `steps` fixed, `structures` broken — the lexicon did not save it
/// from its predecessor's fate), `imperative-lex` (dev −1 on `Lifts`,
/// the documented killer), `proper-name`, `directional-adv`,
/// `a-predicative` (test −1 on `aground`, gold ADP — `run aground`
/// is particle use), `ness/ous/tion/ment-noun/adj`, `ward-adv`,
/// `more-adj` (zero fires anywhere: sound shapes, no support).
/// Eleven of fourteen candidates removed; the gate plus EWT
/// measurement did their job.
///
/// DUAL-PATH DISCIPLINE (2026-10-10): every rule here was calibrated
/// on perceptron margins over EWT. The neural paths (`english-pos-neural`,
/// `english-joint` examples/tests) replay these rules on neural
/// margins — a new rule must re-gate there (sweep + EWT prod numbers)
/// because margin scales and vocab premises need not transfer
/// (`oov-nn`'s OOV gate is EWT-vocab: stale under an EWT+GUM tagger,
/// so neural production excludes it by name — see the neural sweep
/// test). Never assume transfer; measure both.
pub const RULES: &[Rule] = &[
    Rule {
        name: "to-prep",
        threshold: 2.0,
        test: to_prep,
    },
    Rule {
        name: "have-verb",
        threshold: 2.0,
        test: have_verb,
    },
    Rule {
        name: "to-verb",
        threshold: 2.0,
        test: to_verb,
    },
    Rule {
        name: "that-det",
        threshold: 2.0,
        test: that_det,
    },
    Rule {
        name: "that-rel",
        threshold: 2.0,
        test: that_rel,
    },
    Rule {
        name: "det-noun",
        threshold: 2.0,
        test: det_noun,
    },
    Rule {
        name: "that-sconj",
        threshold: 2.0,
        test: that_sconj,
    },
    Rule {
        name: "that-ccomp",
        threshold: 2.0,
        test: that_ccomp,
    },
    Rule {
        name: "subconj-adp",
        threshold: 2.0,
        test: subconj_adp,
    },
    Rule {
        name: "apos-part",
        threshold: 2.0,
        test: apos_part,
    },
    Rule {
        name: "to-part",
        threshold: 2.0,
        test: to_part,
    },
    Rule {
        name: "that-vcomp",
        threshold: 2.0,
        test: that_vcomp,
    },
    Rule {
        name: "pass-by",
        threshold: 5.0,
        test: pass_by,
    },
    Rule {
        name: "quite-adv",
        threshold: 20.0,
        test: quite_adv,
    },
    Rule {
        name: "those-pron",
        threshold: 2.0,
        test: those_pron,
    },
    Rule {
        name: "there-adv",
        threshold: 2.0,
        test: there_adv,
    },
    Rule {
        name: "be-aux",
        threshold: 2.0,
        test: be_aux,
    },
    Rule {
        name: "oov-nn",
        threshold: 2.0,
        test: oov_nn,
    },
];

/// True when a VERB/AUX tag appears strictly ahead of `i`
/// (positions i+2..=i+6) before any clause boundary
/// (PUNCT/CCONJ/SCONJ/sentence end): the Constraint Grammar
/// barrier scan, bounded. Boundary stops keep it from firing
/// cross-clausally; the i+1 position is deliberately skipped
/// (the rules using this already condition on it).
fn fin_ahead(tags: &[Tag], i: usize) -> bool {
    for j in (i + 2)..((i + 7).min(tags.len())) {
        match tags[j] {
            Tag::Punct | Tag::Cconj | Tag::Sconj => return false,
            Tag::Verb | Tag::Aux => return true,
            _ => {}
        }
    }
    false
}

/// Complementizer `that` after a verb (`said that men rejoice`):
/// pred-PRON `that` with VERB prev, nominal next, and a finite
/// verb ahead before the boundary is SCONJ 395:16 in EWT — the
/// finite-verb-ahead test separates it from determiner `that`
/// (`saw that man`, no verb ahead: 59:36 mixed, correctly out of
/// reach). The CG/RDR barrier shape for the direction `that-det`
/// cannot cover. Candidate from the 2026-10-06 gate-zone autopsy;
/// admit only with EWT dev/test ≥ 0 measured.
fn that_ccomp(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Pron || low[i] != "that" {
        return None;
    }
    if i == 0 || tags.get(i - 1) != Some(&Tag::Verb) {
        return None;
    }
    if !matches!(
        tags.get(i + 1),
        Some(Tag::Det | Tag::Adj | Tag::Noun | Tag::Propn | Tag::Pron | Tag::Num)
    ) {
        return None;
    }
    fin_ahead(tags, i).then_some(Tag::Sconj)
}

/// Subordinating word read as complementizer (`after the war`):
/// pred-SCONJ closed-class prep words (`as/after/before/since/
/// without/upon/with/like/by/on/of`) with nominal next and NO
/// finite verb ahead are ADP 5356:165 in EWT (97%) — a plain
/// prepositional phrase, not a clause. The mirror of `that-ccomp`
/// through the same barrier test. Candidate from the 2026-10-06
/// gate-zone autopsy (10 instances, largest single mass); admit
/// only with EWT dev/test ≥ 0 measured.
fn subconj_adp(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Sconj {
        return None;
    }
    const WORDS: &[&str] = &[
        "as", "after", "before", "since", "without", "upon", "with", "like", "by", "on", "of",
    ];
    if !WORDS.contains(&low[i].as_str()) {
        return None;
    }
    if !matches!(
        tags.get(i + 1),
        Some(Tag::Det | Tag::Adj | Tag::Noun | Tag::Propn | Tag::Pron | Tag::Num)
    ) {
        return None;
    }
    (!fin_ahead(tags, i)).then_some(Tag::Adp)
}

/// Possessive `'s` read as auxiliary (`Daggoo's hat`):
/// pred-AUX/ADP `'s` after NOUN/PROPN and before a nominal head is
/// PART 571:8 in EWT (copula contractions need PRON prev — `it's`
/// — which stays out, and predicative `Ahab's above` / `man's a`
/// human` stay out via the nominal-next guard). Candidate from
/// the 2026-10-06 gate-zone autopsy; admit only with EWT dev/test
/// ≥ 0 measured.
fn apos_part(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if !matches!(tags[i], Tag::Aux | Tag::Adp) {
        return None;
    }
    if low[i] != "'s" {
        return None;
    }
    if i == 0 || !matches!(tags.get(i - 1), Some(Tag::Noun) | Some(Tag::Propn)) {
        return None;
    }
    if !matches!(
        tags.get(i + 1),
        Some(Tag::Noun | Tag::Propn | Tag::Adj | Tag::Num | Tag::Pron)
    ) {
        return None;
    }
    Some(Tag::Part)
}

/// Infinitive `to` read as preposition (`want to go`):
/// pred-ADP `to` before a VERB is PART 2893:48 in EWT (98.4%) —
/// a preposition never takes a bare-verb complement. The mirror
/// of `to-prep` (which handles the nominal-next direction); the
/// SCONJ/ADP residue is EWT annotation noise.
///
/// ADMITTED 2026-10-06 (τ=2.0): EWT dev +5 / test +1 net, all
/// sampled fires gold-correct; 20 Moby fires, ~18 infinitive
/// markers with 2 suspect gerund-complement residuals
/// (`preliminary to scalping`, `used to impenitent` — ADJ-prev
/// `to` + participle reads prepositional, but UPOS doesn't split
/// VBG from VB so no cheap guard exists; documented trigger).
/// `flies` holds.
fn to_part(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Adp || low[i] != "to" {
        return None;
    }
    matches!(tags.get(i + 1), Some(Tag::Verb)).then_some(Tag::Part)
}

/// Verb-complement `that` (`said that the answer`): pred-PRON
/// `that` with VERB prev and DET next is SCONJ 129:1 in EWT —
/// the complement-taking verb selects the reading, so unlike
/// `that-ccomp` no finite-verb-ahead barrier is needed. NOUN-prev
/// (`the way that the group`) stays out: reduced relatives live
/// there (36:21 mixed).
///
/// ADMITTED 2026-10-06 (τ=2.0): EWT ±0 (zero fires both splits);
/// 2 Moby hand-verified fixes (`happened that those boats`,
/// `saw that this ship`), zero known breaks, `flies` holds.
fn that_vcomp(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Pron || low[i] != "that" {
        return None;
    }
    if i == 0 || tags.get(i - 1) != Some(&Tag::Verb) {
        return None;
    }
    matches!(tags.get(i + 1), Some(Tag::Det)).then_some(Tag::Sconj)
}

/// Eventive participle read as predicative adjective (`was broken by
/// X`): predicted ADJ after a be-form AUX with a `by`+ADP agent
/// within +1..+4 ahead (verb/clause barrier). EWT gold is VERB 110:0
/// with the -ed/-en guard (130:4 without — the guard removes all 4
/// known breaks; det-noun precedent: the barrier carries precision,
/// morphology trims). Fixes the participle-ADJ overfire the passive
/// pilot rediscovered (`unfolded/set/overpowered/fastened/stranded/
/// `broken` class).
///
/// v1.1 MEASURED AND REVERTED 2026-10-07: adverb gap (`was rudely
/// broken`, EWT 14:0) and closed irregular list (EWT-attested core
/// 19:0 + no-change-verb completion; `-orn` and gap+irreg excluded
/// for zero evidence) — both with EWT-majority, both with ZERO fires
/// on dev, test, sweep, and hand-built shapes (the tagger either
/// tags these shapes VERB or better than the gate). Unmeasurable
/// code doesn't ship (same standard that removed 11 candidates);
/// the numbers stand as the residual record. Residuals: `-orn`
/// participles, non-ADV gaps (no EWT evidence either way).
///
/// ADMITTED 2026-10-07 (τ=5.0): EWT dev ±0, test +1 (`I was married
/// by a judge`, canonical passive, margin 4.0 — calibrated τ catches
/// margin-4 fires, blocks margin-11 statives like `tired by`);
/// abstains correctly on the stative twin (`aren't married to...`,
/// prev is `n't`/PART); sweep 0 fires / 0 breaks; full workspace
/// green; `flies` holds.
fn pass_by(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Adj {
        return None;
    }
    if !(low[i].ends_with("ed") || low[i].ends_with("en")) {
        return None;
    }
    if i == 0 || tags.get(i - 1) != Some(&Tag::Aux) {
        return None;
    }
    if !matches!(
        low[i - 1].as_str(),
        "be" | "am" | "is" | "are" | "was" | "were" | "been" | "being"
    ) {
        return None;
    }
    // Ahead barrier scan for the agent (same stop set the EWT count
    // was measured with — PUNCT blocks cross-sentence agents).
    let mut j = i + 1;
    while j <= i + 4 && j < tags.len() {
        match tags[j] {
            Tag::Verb
            | Tag::Aux
            | Tag::Sconj
            | Tag::Cconj
            | Tag::Part
            | Tag::Punct
            | Tag::Intj
            | Tag::X
            | Tag::Sym => return None,
            _ => {}
        }
        if low[j] == "by" && tags[j] == Tag::Adp {
            return Some(Tag::Verb);
        }
        j += 1;
    }
    None
}

/// Determiner `quite` before an adjective/adverb (`quite sure`):
/// predicted DET with ADJ/ADV next → ADV. EWT gold is ADV 27:0 in
/// this shape (`quite` reads DET only before DET — `quite a few`);
/// the model overfires the determiner reading everywhere else
/// (found by the hedge lint eval, which went 13/14 FN on it).
/// Single-word lexical rule, `that`-precedent (function word with a
/// fixed EWT majority, gated below margin like everything else).
///
/// ADMITTED 2026-10-07 (τ=2.0): EWT dev +1, test +1 (both correct,
/// zero breaks); unit fire + 2 abstains green; evals neutral
/// (sweep 0 fires / explicit 0 breaks); full workspace green;
/// `flies` holds. Does NOT move the motivating hedge eval (its 13
/// `quite` FNs sit at margins 9–18, unreachable below any gate —
/// the hedge-recall bar recalibrates to measured-mechanism on that
/// account); this rule's gates stand on their own.
/// WIDENED 2026-10-08 (τ=2.0→20.0): the τ=99 sweep shows dev/test
/// never present this shape above margin 2.0 (one standing fire,
/// correct) — widening moves EWT ±0 by measurement, not by hope.
/// Train in-shape breaks: zero (DET-gold `quite` is DET-next-only,
/// excluded by the guard; the lone NOUN-next DET abstains the same
/// way). Hedge eval recall 0.567→0.800 (+7 TP: bashful, certain,
/// essential, impossible, happy, nervous, lame; FP unchanged at 1;
/// remaining FNs are neighbor-mistags, `quite-a`-guard abstains,
/// or margins > 20 — other mechanisms). Full workspace green.
/// Sentence-final locative `there`/`here` (`water-gazers
/// there`): pred-PRON at the last two positions is gold ADV 22:0
/// in EWT train (existentials lead sentences; trailers locate).
/// Single closed pair, `quite`-precedent, τ=2.0.
fn there_adv(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Pron || (low[i] != "there" && low[i] != "here") {
        return None;
    }
    (i + 2 >= tags.len()).then_some(Tag::Adv)
}

/// Infinitive `be` read as main verb (`seems to be a duchess`):
/// pred-VERB `be` right after `to` is gold AUX 38:5 in EWT train
/// (the 5 breaks are raising/existential annotation noise —
/// `there seems to be a problem`, same surface shape, no guard
/// available). Modal-prev (`can/will be`) stays out: thin and mixed
/// there (5–8 instances either way), so the guard is prev-word
/// `to` only. Candidate from disagreement mining (tagger-vs-96.5
/// on Austen/Doyle/Stevenson: 67 `to be` instances, 3 in-gate at
/// margin 1.0); admit only with EWT dev/test ≥ 0 measured.
fn be_aux(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Verb || low[i] != "be" {
        return None;
    }
    (i > 0 && low[i - 1] == "to").then_some(Tag::Aux)
}

/// Out-of-vocabulary neighbor tag: pre-lowered `low[i]` never seen in
/// EWT train takes the EWT-majority tag of its nearest GloVe-50
/// neighbor among train-vocab targets (R3-3: dev +21 / test +14 net
/// at τ=2.0, both splits positive — the first consistently-positive
/// backoff; fires only when the neighbor tag differs, so agreement
/// is a no-op). Last in RULES: the 17 shipped rules keep priority.
/// `-ing` words stay out: participles/gerunds need clausal barrier
/// analysis this vote cannot see (the deferred participle gap —
/// `pass-by` covers be-prev only), and the damage is measured
/// (sweep `plodding` VERB→ADJ plus 2 EWT breaks against 6 forgone
/// fixes, documented cost).
fn oov_nn(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if !is_oov(&low[i]) || low[i].ends_with("ing") {
        return None;
    }
    match nn_tag(&low[i]) {
        Some(t) if t != tags[i] => Some(t),
        _ => None,
    }
}

fn quite_adv(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Det || low[i] != "quite" {
        return None;
    }
    matches!(tags.get(i + 1), Some(Tag::Adj) | Some(Tag::Adv)).then_some(Tag::Adv)
}

/// Elliptical `those` (`those in power`, `those of you`): pred-DET
/// `those` before ADP is PRON 7:0 in EWT train (the head noun is
/// elided; ADJ-next stays out — DET 14:5 there — as do NOUN/NUM
/// (44/5:0 DET) and VERB (2:3 noise)). Single-word lexical rule,
/// `quite`-precedent, τ=2.0.
fn those_pron(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Det || low[i] != "those" {
        return None;
    }
    matches!(tags.get(i + 1), Some(Tag::Adp)).then_some(Tag::Pron)
}

/// Prepositional `to` read as infinitive marker (`to Coenties
/// Slip`). Infinitive `to` is followed by VERB/AUX/ADV/PART — never
/// a nominal — so nominal-next plus a verb-stem guard (protects
/// `to approve`, `to test`, `to mention`) makes the flip EWT-safe
/// by shape. Guards, not just gates.
fn to_prep(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Part || low[i] != "to" {
        return None;
    }
    let next_tag = match tags.get(i + 1) {
        Some(t) => *t,
        None => return None,
    };
    if !matches!(
        next_tag,
        Tag::Det | Tag::Noun | Tag::Propn | Tag::Pron | Tag::Num | Tag::Adj
    ) {
        return None;
    }
    (!known_verb_form(&low[i + 1])).then_some(Tag::Adp)
}

/// Possessive `have` read as auxiliary (`I have of driving`):
/// AUX-`have` is followed by participle/`to`/negation, never a
/// nominal — nominal-next flips to VERB.
fn have_verb(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Aux || low[i] != "have" {
        return None;
    }
    match tags.get(i + 1) {
        Some(Tag::Det | Tag::Pron | Tag::Adj | Tag::Noun | Tag::Num | Tag::Adp) => Some(Tag::Verb),
        _ => None,
    }
}

/// Infinitive head read as noun (`to approve`): previous token is
/// (predicted) infinitive `to`. The `to-prep` guard above keeps
/// genuine `to` intact; this fixes the head.
fn to_verb(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Noun {
        return None;
    }
    let prev = i.checked_sub(1)?;
    let prev_tag = match tags.get(prev) {
        Some(t) => *t,
        None => return None,
    };
    if prev_tag != Tag::Part || low[prev] != "to" {
        return None;
    }
    known_verb_form(&low[i]).then_some(Tag::Verb)
}

/// Determiner `that` read as relative pronoun (`of that slouching
/// snow`, sentence-initial `That ...`): after a preposition (or at
/// sentence start) and before a nominal (ADJ/NOUN), EWT gold is DET
/// 89:1 — relatives take clauses (VERB/AUX next, PRON gold), never
/// bare nominals. Guards, not just gates: VERB/ADV/DET/NUM/PROPN
/// next all stay out (EWT splits there favor PRON or are tiny).
/// Candidate from the 2026-10-06 eval-margin probe (4 actionable
/// fires); admit only with EWT dev/test ≥ 0 measured.
fn that_det(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Pron || low[i] != "that" {
        return None;
    }
    let prev_ok = i == 0 || tags.get(i - 1) == Some(&Tag::Adp);
    let next_ok = matches!(tags.get(i + 1), Some(Tag::Adj) | Some(Tag::Noun));
    (prev_ok && next_ok).then_some(Tag::Det)
}

/// Relativizer `that` read as complementizer (`the book that sells`):
/// with a VERB/AUX immediately next, EWT gold is PRON 534:4 over
/// all `that` uses — relatives take clauses, and the clause's verb
/// is the tell a left-to-right decoder cannot see (all three rule
/// traditions — Brill, fnTBL/RDR, Constraint Grammar barriers —
/// converge on right-verb evidence for this reading).
///
/// ADMITTED 2026-10-06 (τ=2.0): EWT ±0 (zero fires both splits);
/// 2 Moby hand-verified fixes (`the lines that using all their
/// dexterous...`, `that found in the secret...` — participles
/// can't head SCONJ clauses, so both are relative PRON), zero
/// known breaks, `flies` holds.
fn that_rel(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Sconj || low[i] != "that" {
        return None;
    }
    matches!(tags.get(i + 1), Some(Tag::Verb) | Some(Tag::Aux)).then_some(Tag::Pron)
}

/// Plural `-s` noun read as 3sg verb (`the glitters`, `the cells`):
/// with a determiner 1–4 back and only ADJ/ADV between (the
/// Constraint Grammar determiner+barrier shape), EWT gold is NOUN
/// 1902:3. Syntax-gated, not suffix-gated — the barrier (not the
/// `-s`) carries the precision, dodging the `-ies`/`-us` plural
/// collision that killed the morphology-only `s-verb` rule.
///
/// ADMITTED 2026-10-06 (τ=2.0): EWT ±0 (zero fires both splits);
/// 1 Moby hand-verified fix (`the front of the try-works` —
/// lexicalized equipment noun), zero known breaks, `flies` holds.
///
/// Complementizer `that` read as relative pronoun (`I know that
/// big dogs bark`): with DET+ADJ two ahead, EWT gold is SCONJ
/// 45:5 — a complement clause is coming, not a bare nominal
/// (the RDR-mined two-wide extension of `that-det`, which covers
/// only the PRON→DET direction).
///
/// ADMITTED 2026-10-06 (τ=2.0): EWT ±0 (zero fires both splits);
/// 3 Moby hand-verified fixes (`that the Greenland whale...`,
/// `so that the whole rope will bear`, `so that the precious
/// gold seems...` — all full clauses with NP subjects), zero
/// known breaks, `flies` holds.
fn that_sconj(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Pron || low[i] != "that" {
        return None;
    }
    match (tags.get(i + 1), tags.get(i + 2)) {
        (Some(Tag::Det), Some(Tag::Adj)) => Some(Tag::Sconj),
        _ => None,
    }
}
fn det_noun(tags: &[Tag], low: &[String], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Verb {
        return None;
    }
    let w = low[i].as_str();
    if !w.ends_with('s') || w.ends_with("ss") {
        return None;
    }
    let mut j = i;
    for _ in 0..4 {
        j = match j.checked_sub(1) {
            Some(p) => p,
            None => return None,
        };
        match tags.get(j) {
            Some(Tag::Det) => return Some(Tag::Noun),
            Some(Tag::Adj) | Some(Tag::Adv) => continue,
            _ => return None,
        }
    }
    None
}

/// Apply `rules` to decoded `(tag, margin)` pairs in place.
/// Each token takes the first matching rule whose gate opens
/// (`0 < margin < threshold`). `low` is the lowercased surface
/// pieces for shape tests — production callers reuse the decoder's
/// copy (see [`Model::tag_beam_margins_lowered`]) instead of
/// lowercasing the sentence again. Predicates all see the pre-pass
/// tag sequence (snapshotted once), not mid-rewrite state — later
/// rules never observe earlier rewrites within a pass.
pub fn apply_rules(tagged: &mut [(Tag, f32)], rules: &[Rule], low: &[String]) {
    let snapshot = tags_snapshot(tagged);
    for (i, (tag, margin)) in tagged.iter_mut().enumerate() {
        for rule in rules {
            if *margin > 0.0
                && *margin < rule.threshold
                && let Some(fix) = (rule.test)(&snapshot, low, i)
            {
                *tag = fix;
                break;
            }
        }
    }
}

/// Snapshot current tags for predicate reads (predicates must see a
/// stable tag sequence, not mid-rewrite state).
fn tags_snapshot(tagged: &[(Tag, f32)]) -> Vec<Tag> {
    tagged.iter().map(|(t, _)| *t).collect()
}
