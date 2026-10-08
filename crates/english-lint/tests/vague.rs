//! Vague-demonstrative eval: 60 (context, target) pairs (1 book +
//! 29 composed positives; 30 negatives: 12 clear book pronominals
//! with real/composed contexts, 6 determiner gate tests, 1
//! contraction, 11 scope-boundary/plains). Documents are annotated
//! as pairs — the rule NEEDS previous-sentence context, so the
//! single-sentence harness of the other evals cannot apply.
//!
//! Gold discipline: positive = a careful reader cannot tell which
//! antecedent the initial demonstrative means (competing nouns in
//! context). Clear summative/anaphoric uses (`That was my first
//! kick`), determiners (`this man` — PRON-gated out), and
//! non-initial demonstratives count NEGATIVE. Shallow path
//! (grammar + vendored tagger): runs everywhere.
//!
//! Bars (pre-registered): precision >= 0.75, recall >= 0.50
//! (nominalization's bars — the closest judgment-call precedent).

use english_lint::{Rule, VagueDemonstrative, annotate_shallow};

// (context, target, demonstrative is vague).
#[rustfmt::skip]
const PAIRS: &[(&str, &str, bool)] = &[
    // Positives: book borderline (exclamatory, no recoverable referent).
    ("To preach the Truth to the face of Falsehood!",
     "That was it!", true),
    // Positives, composed: competing-antecedent shapes.
    ("I told him about the policy and the meeting.",
     "That was unacceptable.", true),
    ("She kept the letter and the photograph on the desk.",
     "This upset him.", true),
    ("The ports were closed and the ships were delayed.",
     "That caused problems.", true),
    ("He praised the cook and the captain.",
     "This annoyed the crew.", true),
    ("They cancelled the trip and the refund never came.",
     "That was frustrating.", true),
    ("She blamed the driver and the mechanic.",
     "This started an argument.", true),
    ("He moved the boxes and the bags to the garage.",
     "That took all morning.", true),
    ("The mayor met the governor and the press.",
     "This made headlines.", true),
    ("I read the report and the appendix.",
     "That took hours.", true),
    ("She sold the house and the car.",
     "This surprised everyone.", true),
    ("They filed the forms and the receipts.",
     "Those were incomplete.", true),
    ("He interviewed the clerks and the managers.",
     "These were unhelpful.", true),
    ("She corrected the proofs and the index.",
     "This delayed publication.", true),
    ("He drained the pond and the ditch.",
     "That killed the reeds.", true),
    ("They debated the merger and the layoffs.",
     "This dragged on for weeks.", true),
    ("She photographed the church and the bridge.",
     "Those came out well.", true),
    ("He repaired the clock and the radio.",
     "This took patience.", true),
    ("The storm flooded the cellar and the kitchen.",
     "That ruined the floors.", true),
    ("I asked about the rates and the fees.",
     "Those were unclear.", true),
    ("She edited the preface and the notes.",
     "These needed work.", true),
    ("He watered the roses and the hedges.",
     "That was enough for today.", true),
    ("They inspected the brakes and the tyres.",
     "These were worn.", true),
    ("She wrapped the glasses and the plates.",
     "Those survived the move.", true),
    ("He graded the essays and the exams.",
     "These disappointed him.", true),
    ("The wind tore the shutters and the tiles.",
     "That let the rain in.", true),
    ("I copied the files and the folders.",
     "Those went missing.", true),
    ("She aired the rooms and the halls.",
     "This cleared the smell.", true),
    ("He counted the votes and the ballots.",
     "Those did not match.", true),
    ("They painted the doors and the frames.",
     "These still need a second coat.", true),
    // Negatives: clear book pronominals (verbatim targets; composed
    // contexts where the book lead is a mid-word truncation).
    ("He felt a sudden sharp poke in his rear from Captain Peleg's leg.",
     "That was my first kick.", false),
    ("But Elijah passed on, without seeming to notice us.",
     "This relieved me, and I pronounced him a humbug.", false),
    ("I account it high time to get to sea as soon as I can.",
     "This is my substitute for pistol and ball.", false),
    ("Find a gentlewoman who can make a small income go a good way.",
     "This is my advice.", false),
    ("She suggested some shelves in the closets upstairs.",
     "That is all very proper and civil, I am sure.", false),
    ("The melancholy event may not come for several years.",
     "This has been my motive, my fair cousin.", false),
    ("But the time has at last come for a new proclamation.",
     "This is Charing Cross.", false),
    ("He hailed a stranger vessel bound home for Nantucket.",
     "This is the Pequod, bound round the world!", false),
    ("For himself, he would do this, he said, whether they joined him or not.",
     "That was the last night he should spend in that den.", false),
    ("Ross Browne are pretty correct in contour, but they are wretchedly engraved.",
     "That is not his fault though.", false),
    ("The obstinacy of that leviathan is otherwise inexplicable.",
     "This is what I mean.", false),
    ("Ha, ha! Old Ahab! The White Whale! He will nail you!",
     "This is a pine tree.", false),
    // Negatives: determiner gate (tag-gated out, never fires).
    ("What business have I with this pipe?",
     "This thing is meant for sereneness.", false),
    ("There are only two books that describe the living sperm whale.",
     "Those books are Beale's and Bennett's.", false),
    ("His grand distinguishing feature, the fin, is often a conspicuous object.",
     "This fin is some three or four feet long.", false),
    ("You could pretty plainly tell how long each one had been ashore.",
     "This young fellow's healthy cheek is like a sun-toasted pear.", false),
    ("God keep me from ever completing anything.",
     "This whole book is but a draught.", false),
    ("He tasks me, and he heaps me, and I see in him outrageous strength.",
     "That inscrutable thing is chiefly what I hate.", false),
    // Negatives: contraction (fires by shape, referent clear).
    ("The landlady had taken his harpoon from him the evening before.",
     "That's strange.", false),
    // Negatives: scope boundary (non-initial demonstratives stay silent).
    ("He read the verdict.",
     "He said that was wrong.", false),
    ("The country is beautiful.",
     "I know this country well.", false),
    ("The toolbox is open.",
     "Give me that hammer.", false),
    ("The forecast changed.",
     "They say it will rain.", false),
    ("The sky darkened.",
     "It is raining.", false),
    ("The usher objected.",
     "She told him this was wrong.", false),
    // Negatives: plains.
    ("The room fell silent.",
     "The astonishment of the ladies was just what he wished.", false),
    ("The watch changed.",
     "A day or two passed, and there was great activity aboard the ship.", false),
    ("She asked a question.",
     "Well, Jane, who is it from?", false),
    ("He denied it.",
     "I was certain that it was indeed he.", false),
    ("The crew listened.",
     "He tasks me, and he heaps me.", false),
];

#[test]
fn vague_precision_recall() {
    assert_eq!(PAIRS.len(), 60);
    let tagger =
        english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap();
    let rule = VagueDemonstrative;
    let (mut tp, mut fp, mut tn, mut fn_) = (0usize, 0usize, 0usize, 0usize);
    let mut misses = Vec::new();
    for (pi, (ctx, targ, gold)) in PAIRS.iter().enumerate() {
        let doc = annotate_shallow(&tagger, &format!("{ctx} {targ}"));
        assert!(
            doc.sentences.len() >= 2,
            "pair #{pi} did not split into 2+ sentences: {ctx} || {targ}"
        );
        // The target is the LAST sentence (contexts may split, e.g.
        // interjections); a finding counts only inside it.
        let span = doc.sentences.last().unwrap().span.clone();
        let fired = rule
            .check(&doc)
            .iter()
            .any(|f| span.contains(&f.span.start));
        match (fired, gold) {
            (true, true) => tp += 1,
            (true, false) => {
                fp += 1;
                misses.push(format!("FP#{pi}: {targ}"));
            }
            (false, false) => tn += 1,
            (false, true) => {
                fn_ += 1;
                misses.push(format!("FN#{pi}: {targ}"));
            }
        }
    }
    let precision = tp as f64 / (tp + fp).max(1) as f64;
    let recall = tp as f64 / (tp + fn_).max(1) as f64;
    eprintln!(
        "vague eval: tp={tp} fp={fp} tn={tn} fn={fn_} precision={precision:.3} recall={recall:.3}\nmisses: {misses:?}"
    );
    assert!(
        precision >= 0.75,
        "precision bar 0.75 missed: {precision:.3}"
    );
    assert!(recall >= 0.50, "recall bar 0.50 missed: {recall:.3}");
}
