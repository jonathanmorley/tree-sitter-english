//! Gold POS tags for two hard Moby-Dick sentences, hand-reviewed token
//! by token (see commit history for the review discussion). UD-style
//! tokens: punctuation split off, `orator` + `'s` split.
//!
//! Model disagreements below are deliberate gold calls, not typos:
//! - `as` (equative): ADP — EWT "as + DET" is ADP 182 vs SCONJ 44.
//! - `Arrayed`: VERB — sentence-initial VBN is 70-0 in EWT, and the only
//!   attire-verb precedents are VERB; the predicative-absolute reading
//!   (cf. `intent`, certainly ADJ) keeps this the weakest verdict here.
//! - `what` (exclamative, 2x): DET — EWT "what + a/an" is DET 6 vs PRON 3
//!   with matching examples; overall `what` still favors PRON, so this
//!   stays the second-weakest call.
//! - `minced`/`wooden`: prenominal participles and derived adjectives
//!   are ADJ under amod; `bible`/`black`: nominal uses are NOUN.
//! - `beneath`/`in`/`for`: plain prepositions are ADP, never NOUN/ADV.
//! - `drop`/`consists`: finite verbs; `the` (article), `Pope` (PROPN).
//!
//! The bar below documents the committed model on this sample (~80%:
//! archaic whaling vocabulary); move it deliberately with model changes.

use english_pos::{Model, Tag};

const TOKENS: &[&str] = &[
    "That",
    "office",
    "consists",
    "in",
    "mincing",
    "the",
    "horse-pieces",
    "of",
    "blubber",
    "for",
    "the",
    "pots",
    ";",
    "an",
    "operation",
    "which",
    "is",
    "conducted",
    "at",
    "a",
    "curious",
    "wooden",
    "horse",
    ",",
    "planted",
    "endwise",
    "against",
    "the",
    "bulwarks",
    ",",
    "and",
    "with",
    "a",
    "capacious",
    "tub",
    "beneath",
    "it",
    ",",
    "into",
    "which",
    "the",
    "minced",
    "pieces",
    "drop",
    ",",
    "fast",
    "as",
    "the",
    "sheets",
    "from",
    "a",
    "rapt",
    "orator",
    "'s",
    "desk",
    ".",
    "Arrayed",
    "in",
    "decent",
    "black",
    ";",
    "occupying",
    "a",
    "conspicuous",
    "pulpit",
    ";",
    "intent",
    "on",
    "bible",
    "leaves",
    ";",
    "what",
    "a",
    "candidate",
    "for",
    "an",
    "archbishopric",
    ",",
    "what",
    "a",
    "lad",
    "for",
    "a",
    "Pope",
    "were",
    "this",
    "mincer",
    "!",
];

const GOLD: &[&str] = &[
    "DET", "NOUN", "VERB", "ADP", "VERB", "DET", "NOUN", "ADP", "NOUN", "ADP", "DET", "NOUN",
    "PUNCT", "DET", "NOUN", "PRON", "AUX", "VERB", "ADP", "DET", "ADJ", "ADJ", "NOUN", "PUNCT",
    "VERB", "ADV", "ADP", "DET", "NOUN", "PUNCT", "CCONJ", "ADP", "DET", "ADJ", "NOUN", "ADP",
    "PRON", "PUNCT", "ADP", "PRON", "DET", "ADJ", "NOUN", "VERB", "PUNCT", "ADV", "ADP", "DET",
    "NOUN", "ADP", "DET", "ADJ", "NOUN", "PART", "NOUN", "PUNCT", "VERB", "ADP", "ADJ", "NOUN",
    "PUNCT", "VERB", "DET", "ADJ", "NOUN", "PUNCT", "ADJ", "ADP", "NOUN", "NOUN", "PUNCT", "DET",
    "DET", "NOUN", "ADP", "DET", "NOUN", "PUNCT", "DET", "DET", "NOUN", "ADP", "DET", "PROPN",
    "AUX", "PRON", "NOUN", "PUNCT",
];

#[test]
fn moby_sample_meets_bar() {
    assert_eq!(TOKENS.len(), GOLD.len());
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let got = model.tag(TOKENS);
    assert_eq!(got.len(), GOLD.len());
    let correct = got
        .iter()
        .zip(GOLD.iter())
        .filter(|(t, g)| t.upos() == **g)
        .count();
    let accuracy = correct as f64 / GOLD.len() as f64;
    assert!(
        accuracy >= 0.78,
        "moby sample accuracy {accuracy:.3} ({correct}/{}) below bar; move deliberately with model changes",
        GOLD.len()
    );
}
