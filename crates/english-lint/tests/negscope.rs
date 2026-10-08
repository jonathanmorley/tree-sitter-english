//! Negation-scope eval: 60 sentences (30 composed positives —
//! edited book prose avoids the shape, same justification as the
//! nominal eval's composed canonicals; 1 verbatim book + 29
//! composed negatives). Quantifier-before-negation is genuinely
//! rare in books (one corpus hit for Q…n't shapes), so the eval
//! leans composed by necessity; its weight rests on the
//! near-miss negatives (reversed order, excluded `no`,
//! quantifier-/negation-less plains).
//!
//! Gold discipline: positive = a careful reader cannot tell
//! whether the negation scopes over all of the quantified set
//! or only part (`everybody didn't come` — nobody, or not
//! everybody?). Reversed scope (`did not see all`), bare
//! negation, bare quantification, and `no`-negation count
//! NEGATIVE. Shallow path (grammar + vendored tagger).
//!
//! Bars (pre-registered): precision >= 0.75, recall >= 0.50.

use english_lint::{NegScope, Rule, annotate_shallow};

// (sentence, quantifier-negation scope is unclear).
#[rustfmt::skip]
const SENTENCES: &[(&str, bool)] = &[
    // Positives, composed: universal-before-negation shapes.
    ("Everybody didn't come to the meeting.", true),
    ("All clocks do not keep time.", true),
    ("Each student did not pass the exam.", true),
    ("All answers were not correct.", true),
    ("Everyone doesn't like rain.", true),
    ("Both men did not survive the winter.", true),
    ("Everybody was not invited.", true),
    ("All is not lost.", true),
    ("Each child did not finish dinner.", true),
    ("Everything he said was not true.", true),
    ("All guests had not arrived by eight.", true),
    ("Every door was not locked.", true),
    ("Both teams did not score.", true),
    ("Each applicant has not replied.", true),
    ("All lights were not out.", true),
    ("Everyone could not hear the speech.", true),
    ("Every window was not shuttered.", true),
    ("All rivers do not run to the sea.", true),
    ("Both sisters never wrote back.", true),
    ("Each soldier did not return.", true),
    ("All debts have not been paid.", true),
    ("Everybody cannot be satisfied.", true),
    ("All members were never told.", true),
    ("Each parcel did not arrive.", true),
    ("Both doors were never locked.", true),
    ("Everyone did not agree.", true),
    ("All hope was not gone.", true),
    ("Each witness never spoke again.", true),
    ("Every bell did not ring.", true),
    ("Both friends have not called.", true),
    // Negatives: one verbatim book (different-clause `never`).
    ("All that Silver said was a riddle to him, but you would never have guessed it.", false),
    // Negatives, composed: reversed scope (negation first — clear).
    ("I did not see all the films.", false),
    ("She has not read every book.", false),
    ("They don't want each seat.", false),
    ("He had not met both brothers.", false),
    ("We could not hear everyone.", false),
    ("She did not invite everybody.", false),
    // Negatives, composed: bare negation, no quantifier.
    ("He did not come.", false),
    ("She never lies.", false),
    ("They don't know the answer.", false),
    ("It was not raining.", false),
    ("We have never met.", false),
    // Negatives, composed: bare quantification, no negation.
    ("All men arrived on time.", false),
    ("Every dog barked at midnight.", false),
    ("Each child smiled.", false),
    ("Both teams played well.", false),
    ("Everyone cheered.", false),
    // Negatives, composed: excluded `no` (pure negation).
    ("No men came.", false),
    ("No one answered.", false),
    ("There is no time.", false),
    // Negatives, composed: adverbial `all` (tag-gated out).
    ("It was all too late to turn back.", false),
    ("She was all alone in the house.", false),
    // Negatives, composed: plains.
    ("The ship sailed at dawn.", false),
    ("He walked to the village.", false),
    ("Birds sang in the trees.", false),
    ("The meeting lasted an hour.", false),
    ("Rain fell all afternoon.", false),
    ("He invited not only friends but family.", false),
    ("Not every cloud brings rain.", false),
    ("They seldom visit all the museums.", false),
];

#[test]
fn negscope_precision_recall() {
    assert_eq!(SENTENCES.len(), 60);
    let tagger =
        english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap();
    let rule = NegScope;
    let (mut tp, mut fp, mut tn, mut fn_) = (0usize, 0usize, 0usize, 0usize);
    let mut misses = Vec::new();
    for (si, (text, gold)) in SENTENCES.iter().enumerate() {
        let doc = annotate_shallow(&tagger, text);
        let fired = !rule.check(&doc).is_empty();
        match (fired, gold) {
            (true, true) => tp += 1,
            (true, false) => {
                fp += 1;
                misses.push(format!("FP#{si}: {text}"));
            }
            (false, false) => tn += 1,
            (false, true) => {
                fn_ += 1;
                misses.push(format!("FN#{si}: {text}"));
            }
        }
    }
    let precision = tp as f64 / (tp + fp).max(1) as f64;
    let recall = tp as f64 / (tp + fn_).max(1) as f64;
    eprintln!(
        "negscope eval: tp={tp} fp={fp} tn={tn} fn={fn_} precision={precision:.3} recall={recall:.3}\nmisses: {misses:?}"
    );
    assert!(
        precision >= 0.75,
        "precision bar 0.75 missed: {precision:.3}"
    );
    assert!(recall >= 0.50, "recall bar 0.50 missed: {recall:.3}");
}
