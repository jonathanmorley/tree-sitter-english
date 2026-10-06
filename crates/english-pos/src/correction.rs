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
//! survivors: `to-prep`, `have-verb`, `to-verb`. A fourth,
//! `that-det`, was admitted 2026-10-06 from the eval-margin probe
//! (determiner-`that`, EWT 89:1 — see the rule).

use crate::Tag;
use crate::lexicon::known_verb_form;

/// A correction rule: a name plus a predicate over the token stream.
/// Predicates see surface pieces, current predicted tags, and margins;
/// they return the replacement tag (or `None` to leave the token).
/// Conditions use *predicted* tags (what the decoder actually saw,
/// including its mistakes) — never gold.
#[derive(Clone, Copy)]
pub struct Rule {
    /// Short identifier (`s-verb`, `imperative-0`, …).
    pub name: &'static str,
    /// Margin gate: applies only when the token decoded with margin
    /// strictly inside `(0, threshold)`. Calibrated per rule.
    pub threshold: f32,
    /// The rewrite test. `i` indexes `pieces`/`tags`/`margins`.
    pub test: fn(&[String], &[Tag], usize) -> Option<Tag>,
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
];

/// Prepositional `to` read as infinitive marker (`to Coenties
/// Slip`). Infinitive `to` is followed by VERB/AUX/ADV/PART — never
/// a nominal — so nominal-next plus a verb-stem guard (protects
/// `to approve`, `to test`, `to mention`) makes the flip EWT-safe
/// by shape. Guards, not just gates.
fn to_prep(pieces: &[String], tags: &[Tag], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Part || pieces[i].to_lowercase() != "to" {
        return None;
    }
    let (next_tag, next_word) = match (tags.get(i + 1), pieces.get(i + 1)) {
        (Some(t), Some(w)) => (*t, w),
        _ => return None,
    };
    if !matches!(
        next_tag,
        Tag::Det | Tag::Noun | Tag::Propn | Tag::Pron | Tag::Num | Tag::Adj
    ) {
        return None;
    }
    (!known_verb_form(next_word)).then_some(Tag::Adp)
}

/// Possessive `have` read as auxiliary (`I have of driving`):
/// AUX-`have` is followed by participle/`to`/negation, never a
/// nominal — nominal-next flips to VERB.
fn have_verb(pieces: &[String], tags: &[Tag], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Aux || pieces[i].to_lowercase() != "have" {
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
fn to_verb(pieces: &[String], tags: &[Tag], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Noun {
        return None;
    }
    let prev = i.checked_sub(1)?;
    let (prev_tag, prev_word) = (tags.get(prev)?, pieces.get(prev)?);
    if *prev_tag != Tag::Part || prev_word.to_lowercase() != "to" {
        return None;
    }
    known_verb_form(&pieces[i]).then_some(Tag::Verb)
}

/// Determiner `that` read as relative pronoun (`of that slouching
/// snow`, sentence-initial `That ...`): after a preposition (or at
/// sentence start) and before a nominal (ADJ/NOUN), EWT gold is DET
/// 89:1 — relatives take clauses (VERB/AUX next, PRON gold), never
/// bare nominals. Guards, not just gates: VERB/ADV/DET/NUM/PROPN
/// next all stay out (EWT splits there favor PRON or are tiny).
/// Candidate from the 2026-10-06 eval-margin probe (4 actionable
/// fires); admit only with EWT dev/test ≥ 0 measured.
fn that_det(pieces: &[String], tags: &[Tag], i: usize) -> Option<Tag> {
    if tags[i] != Tag::Pron || pieces[i].to_lowercase() != "that" {
        return None;
    }
    let prev_ok = i == 0 || tags.get(i - 1) == Some(&Tag::Adp);
    let next_ok = matches!(tags.get(i + 1), Some(Tag::Adj) | Some(Tag::Noun));
    (prev_ok && next_ok).then_some(Tag::Det)
}

/// Apply `rules` to decoded `(tag, margin)` pairs in place.
/// Each token takes the first matching rule whose gate opens
/// (`0 < margin < threshold`). Pieces are needed for shape tests.
/// Predicates all see the pre-pass tag sequence (snapshotted once),
/// not mid-rewrite state — later rules never observe earlier rewrites
/// within a pass.
pub fn apply_rules(pieces: &[String], tagged: &mut [(Tag, f32)], rules: &[Rule]) {
    let snapshot = tags_snapshot(tagged);
    for (i, (tag, margin)) in tagged.iter_mut().enumerate() {
        for rule in rules {
            if *margin > 0.0
                && *margin < rule.threshold
                && let Some(fix) = (rule.test)(pieces, &snapshot, i)
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
