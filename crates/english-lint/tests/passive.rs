//! Passive-voice pilot eval: 60 book-domain sentences (30 passive, 30
//! active incl. adjectival lookalikes), hand-picked from Austen / Doyle /
//! Stevenson / Melville bodies (public domain) and hand-labeled.
//!
//! Gold discipline: finite be/get + eventive past participle with patient
//! subject = passive (UD-convention eventivity — EWT train is silent on
//! most of these lemmas, so construction shape decides; stative-capable
//! participles — married/engaged/mistaken/satisfied/decided — follow the
//! EWT-side predicative-ADJ convention and count ACTIVE). Two items are
//! trimmed at `;` (noted below); everything else is corpus-verbatim.
//! Reduced relatives (`the man killed`) are out of pilot scope (no aux
//! to key on — needs gap detection); none included.
//!
//! Bars (pre-registered tripwires): precision >= 0.85, recall >= 0.60.
//! Needs trained weights (Tier-1 lazy assets); skips loudly when absent
//! — never fails spuriously on a fresh clone.
//!
//! Miss arbitration 2026-10-07 (all 10 pipeline-caused, gold stands):
//! 6× tagger participles→ADJ overfire on eventives (unfolded, set,
//! overpowered, fastened, stranded, broken — the known deferred
//! participle gap, rediscovered independently: a tagger participle
//! fix would directly lift passive recall); 1× labeler miss on
//! correct VERB tags (`are wanted` → aux/nsubj, verified with gold
//! tags); 1× parse garble on verbless apposition (`fair or foul`);
//! 2× tagger-mistag FPs the rule follows correctly (`engaged`→VERB,
//! absurd `sound`→VERB + aux:pass).

use english_lint::{Models, Passive, Rule, lint};

// (sentence, is_passive).
#[rustfmt::skip]
const SENTENCES: &[(&str, bool)] = &[
    ("I do not imagine that much has been unfolded.", true),
    ("It will be her turn soon to be teased.", true),
    ("Her inquiries after her sister were not very favourably answered.", true),
    ("But it has twice been burgled.", true),
    ("And your address had been given me.", true),
    ("I have been trained as an actress myself.", true),
    ("And has your business been attended to in your absence?", true),
    ("He wasn't the one to be cheated.", true),
    ("Elizabeth was summoned to dinner.", true),
    ("She will be taken good care of.", true),
    ("I fancy she was wanted about the mince-pies.", true),
    ("Darcy is not to be laughed at!", true),
    ("Has she been presented?", true),
    ("They are wanted in the farm much oftener than I can get them.", true),
    ("At last the anchor was up, the sails were set, and off we glided.", true),
    ("Greenland whale is deposed.", true),
    ("No, for it was followed by a protestation of innocence.", true),
    ("And then all of a sudden he was interrupted by a noise.", true),
    ("But his wild screams were answered by others quite as wild.", true),
    ("I am not afraid of being overpowered by the impression.", true),
    ("I tried to open it, but it was fastened inside.", true),
    ("Till the morrow their going was deferred.", true),
    // Trimmed at `;` (original continues "; and Mrs. Bennet...").
    ("This was agreed to.", true),
    ("I hope it will soon be increased by seeing her quite well.", true),
    ("God of breezes fair or foul is first invoked for favourable winds.", true),
    ("American whale was stranded.", true),
    ("But I understand that all the other clothes were found in the room.", true),
    ("And this also was given with a will.", true),
    ("At last, however, the party was made up.", true),
    ("I wish it was broken, or that I didn't have any nose at all!", true),
    ("Their manners are not equal to his.", false),
    ("My dear Miss Eliza, why are not you dancing?", false),
    ("Well, Jane, who is it from?", false),
    ("His manner was not effusive.", false),
    ("I was certain that it was indeed he.", false),
    ("Oh, the cause is excellent!", false),
    ("You have really got it!", false),
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
    ("I was afraid that you were engaged.", false),
    ("You are mistaken.", false),
    ("Irene Adler is married.", false),
    ("The astonishment of the ladies was just what he wished.", false),
    ("Next moment we had turned the corner and my home was out of sight.", false),
    ("Holmes shook his head like a man who is far from being satisfied.", false),
    ("I had no more idea of being married till I came back again!", false),
    ("You make me laugh, Charlotte; but it is not sound.", false),
    ("Starbuck, are you sure everything is right?", false),
    ("His stories were what frightened people worst of all.", false),
    // Trimmed at `;` (original continues "; a wild duck flew up...").
    ("All at once there began to go a sort of bustle among the bulrushes.", false),
];

#[test]
fn passive_precision_recall() {
    assert_eq!(SENTENCES.len(), 60);
    let models = match Models::load_workspace() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("SKIP passive eval ({e})");
            return;
        }
    };
    let rules: Vec<&dyn Rule> = vec![&Passive];
    let (mut tp, mut fp, mut tn, mut fn_) = (0usize, 0usize, 0usize, 0usize);
    let mut misses = Vec::new();
    for (si, (text, gold)) in SENTENCES.iter().enumerate() {
        let fired = !lint(&models, text, &rules).is_empty();
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
        "passive eval: tp={tp} fp={fp} tn={tn} fn={fn_} precision={precision:.3} recall={recall:.3}\nmisses: {misses:?}"
    );
    assert!(
        precision >= 0.85,
        "passive precision {precision:.3} below pre-registered bar; investigate, do not lower",
    );
    assert!(
        recall >= 0.60,
        "passive recall {recall:.3} below pre-registered bar; investigate, do not lower",
    );
}
