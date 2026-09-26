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

use english_pos::Model;

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

// Prose eval: 19 sentences from Moby-Dick chapters 1-2 (opening
// paragraphs), hand-tagged token by token. UD-style tokens, ASCII
// apostrophes. Supplements the hard-sample test above with ordinary
// prose - including imperatives, inversion, subordinate clauses, and
// archaic/OOV diction (the model's weak spots per the `verify`
// example: 3sg `-s` verbs, imperatives, titlecase OOV).
//
// Oracle calls (checked against EWT train where possible):
// - existential `is` -> VERB (EWT 249:5 over AUX).
// - `little` (adverbial `a little`) -> ADJ (EWT 110:7 over ADV).
// - `about` before `a/DET` -> ADP particle (EWT 12:3 over ADV).
// - `more` + noun -> ADJ (EWT 69:0 over DET, never DET).
// - predicative participles (`gone`, `bound`) -> ADJ (EWT side;
//   GUM dissents - see CANONICAL.md J2).
// - `all`/`those` -> DET (guideline PronType=Tot/Dem); possessive
//   `my` -> PRON; particles (`off`) -> ADP.
// - OOV doctrine: `Sabbath`/`Corlears`/`Hook`/`Coenties`/`Slip`/
//   `Whitehall`/`Ishmael` -> PROPN (names); `thither`/`thence`/
//   `waterward`/`northward` -> ADV (directional adverbs;
//   `northward` is 1x ADV in EWT, the rest by analogy).
// - `Right and left` (bare directionals) -> ADV; `yet` joining
//   clauses -> CCONJ; exclamative `How` -> ADV.

const TOKENS2: &[&str] = &[
    "Call",
    "me",
    "Ishmael",
    ".", //
    "I",
    "thought",
    "I",
    "would",
    "sail",
    "about",
    "a",
    "little",
    "and",
    "see",
    "the",
    "watery",
    "part",
    "of",
    "the",
    "world",
    ".", //
    "It",
    "is",
    "a",
    "way",
    "I",
    "have",
    "of",
    "driving",
    "off",
    "the",
    "spleen",
    "and",
    "regulating",
    "the",
    "circulation",
    ".", //
    "This",
    "is",
    "my",
    "substitute",
    "for",
    "pistol",
    "and",
    "ball",
    ".", //
    "There",
    "is",
    "nothing",
    "surprising",
    "in",
    "this",
    ".", //
    "Right",
    "and",
    "left",
    ",",
    "the",
    "streets",
    "take",
    "you",
    "waterward",
    ".", //
    "Look",
    "at",
    "the",
    "crowds",
    "there",
    ".", //
    "Circumambulate",
    "the",
    "city",
    "of",
    "a",
    "dreamy",
    "Sabbath",
    "afternoon",
    ".", //
    "Go",
    "from",
    "Corlears",
    "Hook",
    "to",
    "Coenties",
    "Slip",
    ",",
    "and",
    "from",
    "thence",
    ",",
    "by",
    "Whitehall",
    ",",
    "northward",
    ".", //
    "What",
    "do",
    "you",
    "see",
    "?", //
    "But",
    "these",
    "are",
    "all",
    "landsmen",
    ".", //
    "How",
    "then",
    "is",
    "this",
    "?", //
    "Are",
    "the",
    "green",
    "fields",
    "gone",
    "?", //
    "Yet",
    "here",
    "they",
    "all",
    "unite",
    ".", //
    "Tell",
    "me",
    ",",
    "does",
    "the",
    "magnetic",
    "virtue",
    "of",
    "the",
    "needles",
    "of",
    "the",
    "compasses",
    "of",
    "all",
    "those",
    "ships",
    "attract",
    "them",
    "thither",
    "?", //
    "There",
    "is",
    "magic",
    "in",
    "it",
    ".", //
    "Should",
    "you",
    "ever",
    "be",
    "athirst",
    "in",
    "the",
    "great",
    "American",
    "desert",
    ",",
    "try",
    "this",
    "experiment",
    ".", //
    "But",
    "look",
    "!", //
    "here",
    "come",
    "more",
    "crowds",
    ",",
    "pacing",
    "straight",
    "for",
    "the",
    "water",
    ",",
    "and",
    "seemingly",
    "bound",
    "for",
    "a",
    "dive",
    ".", //
];

const GOLD2: &[&str] = &[
    "VERB", "PRON", "PROPN", "PUNCT", //
    "PRON", "VERB", "PRON", "AUX", "VERB", "ADP", "DET", "ADJ", "CCONJ", "VERB", "DET", "ADJ",
    "NOUN", "ADP", "DET", "NOUN", "PUNCT", //
    "PRON", "AUX", "DET", "NOUN", "PRON", "VERB", "ADP", "VERB", "ADP", "DET", "NOUN", "CCONJ",
    "VERB", "DET", "NOUN", "PUNCT", //
    "PRON", "AUX", "PRON", "NOUN", "ADP", "NOUN", "CCONJ", "NOUN", "PUNCT", //
    "PRON", "VERB", "PRON", "ADJ", "ADP", "PRON", "PUNCT", //
    "ADV", "CCONJ", "ADV", "PUNCT", "DET", "NOUN", "VERB", "PRON", "ADV", "PUNCT", //
    "VERB", "ADP", "DET", "NOUN", "ADV", "PUNCT", //
    "VERB", "DET", "NOUN", "ADP", "DET", "ADJ", "PROPN", "NOUN", "PUNCT", //
    "VERB", "ADP", "PROPN", "PROPN", "ADP", "PROPN", "PROPN", "PUNCT", "CCONJ", "ADP", "ADV",
    "PUNCT", "ADP", "PROPN", "PUNCT", "ADV", "PUNCT", //
    "PRON", "AUX", "PRON", "VERB", "PUNCT", //
    "CCONJ", "PRON", "AUX", "DET", "NOUN", "PUNCT", //
    "ADV", "ADV", "AUX", "PRON", "PUNCT", //
    "AUX", "DET", "ADJ", "NOUN", "ADJ", "PUNCT", //
    "CCONJ", "ADV", "PRON", "DET", "VERB", "PUNCT", //
    "VERB", "PRON", "PUNCT", "AUX", "DET", "ADJ", "NOUN", "ADP", "DET", "NOUN", "ADP", "DET",
    "NOUN", "ADP", "DET", "DET", "NOUN", "VERB", "PRON", "ADV", "PUNCT", //
    "PRON", "VERB", "NOUN", "ADP", "PRON", "PUNCT", //
    "AUX", "PRON", "ADV", "AUX", "ADJ", "ADP", "DET", "ADJ", "ADJ", "NOUN", "PUNCT", "VERB", "DET",
    "NOUN", "PUNCT", //
    "CCONJ", "VERB", "PUNCT", //
    "ADV", "VERB", "ADJ", "NOUN", "PUNCT", "VERB", "ADV", "ADP", "DET", "NOUN", "PUNCT", "CCONJ",
    "ADV", "ADJ", "ADP", "DET", "NOUN", "PUNCT", //
];

#[test]
fn moby_prose_meets_bar() {
    assert_eq!(TOKENS2.len(), GOLD2.len());
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let got = model.tag(TOKENS2);
    assert_eq!(got.len(), GOLD2.len());
    let wrong: Vec<(usize, &str, &str, &str)> = got
        .iter()
        .zip(GOLD2.iter())
        .zip(TOKENS2.iter())
        .enumerate()
        .filter(|(_, ((t, g), _))| t.upos() != **g)
        .map(|(i, ((t, g), w))| (i, *w, t.upos(), *g))
        .collect();
    let accuracy = 1.0 - wrong.len() as f64 / GOLD2.len() as f64;
    eprintln!("moby prose misses (index, token, got, gold): {wrong:?}");
    assert!(
        accuracy >= 0.84,
        "moby prose accuracy {accuracy:.3} below bar; move deliberately with model changes",
    );
}
