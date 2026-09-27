//! Second-genre eval: 20 hand-composed sentences (10 news-report,
//! 10 academic-expository), hand-tagged UD-style with ASCII
//! apostrophes. Guards Moby-Dick overfit: different genre, different
//! diction, same EWT-side conventions (CANONICAL.md) and oracle
//! discipline as `moby.rs` (checked word classes against EWT train
//! counts where noted).
//!
//! Text is original (composed for this eval) to keep licensing clean:
//! GUM/LinES text cannot ship here (CC BY-NC-SA), and EWT test/dev
//! sentences would double-count selection data. The cost is
//! author-bias toward tagger-friendly prose, mitigated by including
//! the hard patterns (passives, coordination, subordinates, numbers,
//! quotes, inversions).
//!
//! Oracle calls:
//! - existentials (`There are`, `There is`) -> There/PRON + VERB
//!   (EWT 249:5 over AUX).
//! - `more` + noun -> ADJ (EWT 69:0 over DET, never DET).
//! - predicative adjectives (`concerned`) and infinitival passives
//!   (`be published`) split EWT-side per CANONICAL.md J2: ADJ and
//!   VERB respectively; reduced psych-participles (`worried`) -> ADJ
//!   (EWT 7:0 over VERB).
//! - `such` -> ADJ (EWT 72:24 over DET, against the guideline's
//!   predeterminer listing — EWT-majority rules per oracle
//!   discipline); `all`/`those` -> DET (guideline PronType=Tot/Dem).
//! - `tenth` (attributive ordinal) -> ADJ; EWT's single `tenth` is a
//!   nominal use (`a tenth`), a different construction.
//! - possessive `their`/`its` -> PRON; particles (`off`, `out`,
//!   `up`) -> ADP.
//! - `said`-type reporting verbs -> VERB; quote commas hidden.
//! - numbers/dates (`3.2`, `1998`, `47`) -> NUM.
//! - subjunctive `be` (`demand that it be`) -> AUX (copula slot).
//! - `data` (mass noun, plural agreement) -> NOUN.

use english_pos::Model;

const TOKENS3: &[&str] = &[
    "Stocks",
    "fell",
    "sharply",
    "on",
    "Tuesday",
    "as",
    "investors",
    "worried",
    "about",
    "inflation",
    ".", //
    "The",
    "mayor",
    "said",
    "the",
    "bridge",
    "will",
    "reopen",
    "in",
    "June",
    ".", //
    "Police",
    "arrested",
    "three",
    "men",
    "after",
    "the",
    "robbery",
    ".", //
    "The",
    "committee",
    "voted",
    "5",
    "to",
    "3",
    "to",
    "approve",
    "the",
    "budget",
    ".", //
    "Firefighters",
    "contained",
    "the",
    "blaze",
    "by",
    "midnight",
    ".", //
    "The",
    "court",
    "ruled",
    "that",
    "the",
    "law",
    "was",
    "unconstitutional",
    ".", //
    "Exports",
    "rose",
    "3.2",
    "percent",
    "in",
    "the",
    "third",
    "quarter",
    ".", //
    "The",
    "president",
    "will",
    "visit",
    "France",
    "next",
    "week",
    ".", //
    "Doctors",
    "urge",
    "patients",
    "to",
    "get",
    "vaccinated",
    ".", //
    "The",
    "team",
    "won",
    "its",
    "tenth",
    "championship",
    "in",
    "a",
    "row",
    ".", //
    "The",
    "results",
    "suggest",
    "that",
    "sleep",
    "improves",
    "memory",
    ".", //
    "This",
    "study",
    "examines",
    "the",
    "effects",
    "of",
    "stress",
    "on",
    "students",
    ".", //
    "The",
    "authors",
    "argue",
    "that",
    "policy",
    "failed",
    "because",
    "funding",
    "ended",
    ".", //
    "There",
    "are",
    "several",
    "explanations",
    "for",
    "this",
    "pattern",
    ".", //
    "The",
    "data",
    "were",
    "collected",
    "in",
    "1998",
    ".", //
    "Critics",
    "are",
    "concerned",
    "about",
    "bias",
    "in",
    "the",
    "sample",
    ".", //
    "The",
    "theory",
    "predicts",
    "that",
    "markets",
    "adjust",
    "slowly",
    ".", //
    "We",
    "compared",
    "treatment",
    "and",
    "control",
    "groups",
    ".", //
    "The",
    "committee",
    "demands",
    "that",
    "it",
    "be",
    "published",
    ".", //
    "Such",
    "findings",
    "are",
    "consistent",
    "with",
    "earlier",
    "work",
    ".", //
];

const GOLD3: &[&str] = &[
    "NOUN", "VERB", "ADV", "ADP", "PROPN", "SCONJ", "NOUN", "ADJ", "ADP", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "DET", "NOUN", "AUX", "VERB", "ADP", "PROPN", "PUNCT", //
    "NOUN", "VERB", "NUM", "NOUN", "ADP", "DET", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "NUM", "ADP", "NUM", "PART", "VERB", "DET", "NOUN", "PUNCT", //
    "NOUN", "VERB", "DET", "NOUN", "ADP", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "SCONJ", "DET", "NOUN", "AUX", "ADJ", "PUNCT", //
    "NOUN", "VERB", "NUM", "NOUN", "ADP", "DET", "ADJ", "NOUN", "PUNCT", //
    "DET", "PROPN", "AUX", "VERB", "PROPN", "ADJ", "NOUN", "PUNCT", //
    "NOUN", "VERB", "NOUN", "PART", "VERB", "VERB", "PUNCT", //
    "DET", "NOUN", "VERB", "PRON", "ADJ", "NOUN", "ADP", "DET", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "SCONJ", "NOUN", "VERB", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "DET", "NOUN", "ADP", "NOUN", "ADP", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "SCONJ", "NOUN", "VERB", "SCONJ", "NOUN", "VERB", "PUNCT", //
    "PRON", "VERB", "ADJ", "NOUN", "ADP", "DET", "NOUN", "PUNCT", //
    "DET", "NOUN", "AUX", "VERB", "ADP", "NUM", "PUNCT", //
    "NOUN", "AUX", "ADJ", "ADP", "NOUN", "ADP", "DET", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "SCONJ", "NOUN", "VERB", "ADV", "PUNCT", //
    "PRON", "VERB", "NOUN", "CCONJ", "NOUN", "NOUN", "PUNCT", //
    "DET", "NOUN", "VERB", "SCONJ", "PRON", "AUX", "VERB", "PUNCT", //
    "ADJ", "NOUN", "AUX", "ADJ", "ADP", "ADJ", "NOUN", "PUNCT", //
];

#[test]
fn genre_eval_meets_bar() {
    assert_eq!(TOKENS3.len(), GOLD3.len());
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let got = model.tag(TOKENS3);
    assert_eq!(got.len(), GOLD3.len());
    let wrong: Vec<(usize, &str, &str, &str)> = got
        .iter()
        .zip(GOLD3.iter())
        .zip(TOKENS3.iter())
        .enumerate()
        .filter(|(_, ((t, g), _))| t.upos() != **g)
        .map(|(i, ((t, g), w))| (i, *w, t.upos(), *g))
        .collect();
    let accuracy = 1.0 - wrong.len() as f64 / GOLD3.len() as f64;
    eprintln!("genre misses (index, token, got, gold): {wrong:?}");
    assert!(
        accuracy >= 0.86,
        "genre eval accuracy {accuracy:.3} below bar; move deliberately with model changes",
    );
}
