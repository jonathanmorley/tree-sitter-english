//! Coordination-scope eval: 60 sentences (15 book + 15 composed
//! positives; 8 book + 22 composed negatives). Book targets trimmed
//! to the coordination sentence where noted (hedge-eval precedent);
//! composed items marked below (canonical distribution shapes).
//!
//! Gold discipline: positive = a careful reader cannot tell from
//! the sentence alone whether the adjective distributes over both
//! conjuncts (same-category nouns). Category-blocked readings
//! (towns can't be winters), contrastive adjectives
//! (`old men and young women` — no match by construction),
//! repeated adjectives, adjective-less and coordination-less
//! sentences, and verb-mistag shapes count NEGATIVE. Shallow path
//! (grammar + vendored tagger): runs everywhere.
//!
//! Bars (pre-registered): precision >= 0.75, recall >= 0.50
//! (nominalization's bars — the closest judgment-call precedent).

use english_lint::{CoordScope, Rule, annotate_shallow};

// (sentence, adjective scope is unclear).
#[rustfmt::skip]
const SENTENCES: &[(&str, bool)] = &[
    // Positives, book (trimmed to the coordination sentence).
    ("It seemed to have melted the packed snow and ice from before the house.", true),
    ("You wondered what monstrous cannibal and savage could have gone a death-harvesting.", true),
    ("The entry was hung with a heathenish array of monstrous clubs and spears.", true),
    ("He was the most perfect reasoning and observing machine.", true),
    ("I knew little of my former friend and companion.", true),
    ("It must be some great hoax or fraud.", true),
    ("No legal papers or certificates?", true),
    ("There was nothing remarkable save the expression of extreme chagrin and discontent.", true),
    ("She condescends to drive by in her little phaeton and ponies.", true),
    ("A little cold water and salts soon brought her back.", true),
    ("He was full of anticipations of strange islands and adventures.", true),
    ("He left hold of me, and with incredible accuracy and nimbleness, skipped out.", true),
    ("To our indescribable joy and gratitude, it died slowly away.", true),
    ("I am in the most magnificent health and spirits.", true),
    ("It is remarkable for its numerous glass-factories and paper-mills.", true),
    // Positives, composed: canonical distribution shapes.
    ("Old men and women gathered in the square.", true),
    ("Tall trees and houses lined the road.", true),
    ("Hot coffee and tea were served.", true),
    ("Small cats and dogs slept by the fire.", true),
    ("French wine and cheese arrived.", true),
    ("Poor farmers and workers protested.", true),
    ("Bright stars and planets filled the sky.", true),
    ("Heavy boxes and furniture blocked the door.", true),
    ("Young boys and girls played outside.", true),
    ("Dark clouds and rain approached.", true),
    ("Fresh bread and butter tasted wonderful.", true),
    ("Wild horses and cattle roamed free.", true),
    ("Long novels and poems bored him.", true),
    ("Rich merchants and bankers met downtown.", true),
    ("Bitter winds and snow arrived early.", true),
    // Negatives, book: verb-mistag and separate-clause shapes.
    ("He had made a tolerable fortune, and risen to the honour of knighthood.", false),
    ("You cover yourself with your own blanket, and sleep in your own skin.", false),
    ("I love to sail forbidden seas, and land on barbarous coasts.", false),
    ("It was to remain the whole winter, and Meryton was the head-quarters.", false),
    ("He cursed the doctor, in a feeble voice but heartily.", false),
    ("Those books are Beale's and Bennett's.", false),
    ("This young fellow's healthy cheek is like a sun-toasted pear.", false),
    ("She is tolerable, but not handsome enough to tempt me.", false),
    // Negatives, composed: contrastive adjectives (no match by construction).
    ("Old men and young women arrived.", false),
    ("Tall trees and short houses lined the road.", false),
    ("Hot coffee and cold tea were served.", false),
    ("Wild horses and tame cattle roamed free.", false),
    ("Bright stars and dim planets appeared.", false),
    ("Long novels and short poems bored him.", false),
    // Negatives, composed: repeated adjectives.
    ("Old men and old women arrived.", false),
    ("The hot coffee and hot tea burned him.", false),
    // Negatives, composed: no adjective.
    ("Men and women arrived.", false),
    ("Cats and dogs slept by the fire.", false),
    ("Wine and cheese arrived.", false),
    ("Farmers and workers protested.", false),
    ("Stars and planets filled the sky.", false),
    // Negatives, composed: no coordination.
    ("Old men gathered in the square.", false),
    ("The bitter winds arrived early.", false),
    ("Rich merchants met downtown.", false),
    ("Dark clouds gathered.", false),
    // Negatives, composed: category-blocked distribution.
    ("The project took the whole year and Paris waited.", false),
    ("He read the thick book and Mary laughed.", false),
    ("They painted the red barn and John watched.", false),
    // Negatives, composed: verb in fourth slot.
    ("They watched the old game and cheered.", false),
    ("She bought fresh milk and drank.", false),
];

#[test]
fn coordscope_precision_recall() {
    assert_eq!(SENTENCES.len(), 60);
    let tagger =
        english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap();
    let rule = CoordScope;
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
        "coordscope eval: tp={tp} fp={fp} tn={tn} fn={fn_} precision={precision:.3} recall={recall:.3}\nmisses: {misses:?}"
    );
    assert!(
        precision >= 0.75,
        "precision bar 0.75 missed: {precision:.3}"
    );
    assert!(recall >= 0.50, "recall bar 0.50 missed: {recall:.3}");
}
