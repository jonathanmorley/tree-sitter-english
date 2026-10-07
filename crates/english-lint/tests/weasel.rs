//! Weasel-modifier eval: 60 sentences (30 intensifier + ADJ/ADV,
//! 30 negatives incl. verb-scope, attributive-`very`, other-
//! intensifier and plain lookalikes). Book-domain positives (trimmed
//! attributions noted); 4 composed boundary negatives (marked).
//!
//! Gold discipline: ADV-tagged `very`/`really`/`extremely` with
//! immediately following ADJ/ADV = positive. Verb-scope (`really
//! got`, `really abandoned`), attributive `very` (`the very
//! first/day`), other intensifiers (`so/too/quite/rather`), and
//! bare/plain sentences count NEGATIVE — the rule's stated scope.
//! Shallow path (grammar + vendored tagger): runs everywhere.
//!
//! Bars (pre-registered): precision >= 0.80, recall >= 0.65.
//!
//! Miss arbitration 2026-10-07 (1 FP, 0 FN — gold stands): `Leviathan
//! was slain at the very first dart` double-mistagged (`very`→ADV +
//! `first`→ADV; attributive `very` should read ADJ). Pipeline-caused;
//! the `little`-edge positives (`very little notice/company`) both
//! fired correctly (little reads ADJ).

use english_lint::{Rule, Weasel, annotate_shallow};

// (sentence, has_weasel).
#[rustfmt::skip]
const SENTENCES: &[(&str, bool)] = &[
    ("Mary wished to say something very sensible, but knew not how.", true),
    ("I was very much flattered by his asking me to dance a second time.", true),
    ("You are a very strange creature by way of a friend!", true),
    ("Yes; but as it happens, they are all of them very clever.", true),
    ("I shall be very fit to see Jane--which is all I want.", true),
    ("Her inquiries after her sister were not very favourably answered.", true),
    ("She had very little notice from any but him.", true),
    ("She seems a very pleasant young woman.", true),
    ("Oh dear, yes; but you must own she is very plain.", true),
    ("My style of writing is very different from yours.", true),
    ("That is very bad!", true),
    // Trimmed attribution ("said I; ...").
    ("This is a very unexpected turn of affairs.", true),
    ("Well, I found my plans very seriously menaced.", true),
    // Trimmed attribution ("he remarked").
    ("You did it very nicely.", true),
    ("You really did it very well.", true),
    ("I have the honour to wish you a very good morning.", true),
    ("I did not gain very much, however, by my inspection.", true),
    ("Pray continue your very interesting statement.", true),
    // Trimmed attribution ("said I").
    ("That would suit me very well.", true),
    // Trimmed attribution ("said Holmes").
    ("And you did very wisely.", true),
    ("My father told him no, very little company, the more was the pity.", true),
    ("He was a very silent man by custom.", true),
    ("And he seemed very much relieved.", true),
    ("I am not very sure whether he's sane.", true),
    ("Such eye-wrinkles are very effectual in a scowl.", true),
    // Trimmed attribution ("said Elijah"); repetition shape.
    ("Very dim, very dim.", true),
    ("Why this book of whales is not denominated the Quarto is very plain.", true),
    ("The old monks of Dunfermline were very fond of them.", true),
    ("Queequeg did not look very brisk.", true),
    ("I asked with a very tremulous voice.", true),
    ("You have really got it!", false),
    ("But had Stubb really abandoned the poor little negro to his fate?", false),
    ("Leviathan was slain at the very first dart.", false),
    // Composed: assertive `really` (comma breaks adjacency).
    ("Really, I don't know.", false),
    ("Do you really think so?", false),
    // Composed: other intensifiers (documented boundary).
    ("She is so kind to animals.", false),
    ("It was too late to turn back.", false),
    ("He seemed quite happy today.", false),
    ("They were rather tired afterwards.", false),
    // Composed: attributive `very`.
    ("It was the very thing she wanted.", false),
    // Plain actives, no intensifiers (specificity baseline).
    ("Their manners are not equal to his.", false),
    ("My dear Miss Eliza, why are not you dancing?", false),
    ("Well, Jane, who is it from?", false),
    ("His manner was not effusive.", false),
    ("I was certain that it was indeed he.", false),
    ("Oh, the cause is excellent!", false),
    ("I told him he was out walking.", false),
    ("I thought; and besides, it was difficult to know what to do.", false),
    ("I'll tell you if they get that.", false),
    ("Here, you below there, is it on Bill?", false),
    ("So far there was not a hitch.", false),
    ("We take the risk, but we are not so ignorant as you believe us.", false),
    ("Doctor, said the captain, you are smart.", false),
    ("All these things are not without their meanings.", false),
    ("O young ambition, all mortal greatness is but disease.", false),
    ("A day or two passed, and there was great activity aboard the Pequod.", false),
    ("Yes, we are, but what business is that of yours?", false),
    ("But, to this, Bishop Jebb's anticipative answer is ready.", false),
    ("The astonishment of the ladies was just what he wished.", false),
    ("Next moment we had turned the corner and my home was out of sight.", false),
];

#[test]
fn weasel_precision_recall() {
    assert_eq!(SENTENCES.len(), 60);
    let tagger =
        english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap();
    let rule = Weasel;
    let (mut tp, mut fp, mut tn, mut fn_) = (0usize, 0usize, 0usize, 0usize);
    let mut misses = Vec::new();
    for (si, (text, gold)) in SENTENCES.iter().enumerate() {
        let doc = english_lint::annotate_shallow(&tagger, text);
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
        "weasel eval: tp={tp} fp={fp} tn={tn} fn={fn_} precision={precision:.3} recall={recall:.3}\nmisses: {misses:?}"
    );
    assert!(
        precision >= 0.80,
        "weasel precision {precision:.3} below pre-registered bar; investigate, do not lower",
    );
    assert!(
        recall >= 0.65,
        "weasel recall {recall:.3} below pre-registered bar; investigate, do not lower",
    );
}
