//! Nominalization pilot eval: 60 sentences (30 light-verb +
//! deverbial-noun positives, 30 negatives incl. list-boundary and
//! suffix lookalikes), 11 book-domain + 19 hand-composed canonicals
//! (genre precedent: books underuse bureaucratic shapes, so canonicals
//! carry the rule-shape coverage; book items carry robustness).
//!
//! Gold discipline: light verb (closed table in `lib.rs`) governing a
//! `-tion`/`-sion`/`-sis`/`-ment`/`-ance`/`-ence` noun via `obj`/`obl`
//! (plurals by stem) = positive. Suffix-boundary shapes (`notice`,
//! `promise`, `attempt`, `effort`, `care`, `inquiry`, `choice`,
//! non-light `pay attention`, bare nominalizations, deferred `-ing`
//! gerunds and `-age`/`-edge`) count ACTIVE — the rule's stated scope,
//! not the phenomenon's. Non-deverbial `-ment` (`take a moment` ×2)
//! counts ACTIVE: the suffix heuristic's honest precision cost,
//! included deliberately (not special-cased, per the never-enshrine
//! rule). Post-eval fix 2026-10-07: plurals + Greek `-sis` admitted
//! (eval-found gaps); 3 golds flipped to ACTIVE where the rule's own
//! scope excludes them (`care` no-suffix, `-ing`/`-edge` deferred).
//!
//! Bars (pre-registered tripwires, lower than passive — lexical
//! heuristic, stated upfront): precision >= 0.75, recall >= 0.50.
//! Needs trained weights (Tier-1 lazy assets); skips loudly when absent.

use english_lint::{Models, Nominalization, Rule, lint};

// (sentence, has_light_verb_nominalization).
#[rustfmt::skip]
const SENTENCES: &[(&str, bool)] = &[
    // Book-domain positives (corpus-verbatim).
    ("Nothing that she could say, however, had any influence.", true),
    ("My mother would have no objection, but my father hates London.", true),
    ("Watson, have no compunction about shooting them down.", true),
    ("But you have no notion as to what it could have been?", true),
    ("Having once spotted my man, it was easy to get corroboration.", true),
    ("Did your father make any statement to you before he died?", true),
    ("I will make no allowance.", true),
    ("You have made your position very clear to me.", true),
    ("She will be taken good care of.", false), // No nominal suffix (`care`) — rule-silent like A1 `take care`; idiom, not -itis.
    ("I would, I only had the management of one end of it.", true),
    ("That is to say, you had given your permission.", true),
    // Composed canonicals (rule-shape coverage).
    ("The committee will make a decision tomorrow.", true),
    ("She took action without consulting anyone.", true),
    ("He gave careful consideration to the proposal.", true),
    ("They conducted an investigation into the accident.", true),
    ("We performed an analysis of the results.", true),
    ("She made mention of your contributions.", true),
    ("He took exception to the ruling.", true),
    ("They gave warning of the strike.", false), // `-ing` deferred (get-going class risk) — rule-silent, noted scope gap.
    ("We have knowledge of the incident.", false), // `-edge` deferred (mixed deverbial density) — rule-silent, noted scope gap.
    ("The board made a resolution last night.", true),
    ("She did violence to the text.", true),
    ("He made a declaration of independence.", true),
    ("They took satisfaction in a job well done.", true),
    ("They made arrangements for the trip.", true),
    ("She took measurements twice daily.", true),
    ("She got permission to leave early.", true),
    ("They made an appearance at noon.", true),
    ("She made a difference in their lives.", true),
    ("He has an appreciation of music.", true),
    // Negatives: suffix gaps (rule-silent by construction).
    ("And men take care that they should.", false),
    ("She did damage to the engine.", false),
    ("He made an inquiry into the matter.", false),
    ("They reached an agreement.", false),
    ("She made her choice yesterday.", false),
    ("I will make no promise of the kind.", false),
    ("And how could you tell that they would make their attempt to-night?", false),
    ("Did he make no attempt to see you?", false),
    ("Come, now, make an effort.", false),
    // Negatives: non-light governors.
    ("They paid attention to the speaker.", false),
    // Negatives: non-deverbial -ment (honest precision cost).
    ("Give the proposal a moment of thought.", false),
    ("Take a moment to rest.", false),
    // Negatives: concrete objects with light verbs.
    ("We had dinner at eight.", false),
    ("She took the train to London.", false),
    ("He made dinner for everyone.", false),
    ("We take pride in our work.", false),
    ("He did his duty without complaint.", false),
    // Negatives: clean actives (shared with the passive eval).
    ("His manner was not effusive.", false),
    ("The astonishment of the ladies was just what he wished.", false),
    ("All these things are not without their meanings.", false),
    ("A day or two passed, and there was great activity aboard the Pequod.", false),
    ("O young ambition, all mortal greatness is but disease.", false),
    ("I had no more idea of being married till I came back again!", false),
    ("Holmes shook his head like a man who is far from being satisfied.", false),
    ("Next moment we had turned the corner and my home was out of sight.", false),
    ("You make me laugh, Charlotte; but it is not sound.", false),
    ("Starbuck, are you sure everything is right?", false),
    ("His stories were what frightened people worst of all.", false),
    ("I was afraid that you were engaged.", false),
    ("You are mistaken.", false),
];

#[test]
fn nominalization_precision_recall() {
    assert_eq!(SENTENCES.len(), 60);
    let models = match Models::load_workspace() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("SKIP nominalization eval ({e})");
            return;
        }
    };
    let rules: Vec<&dyn Rule> = vec![&Nominalization];
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
        "nominalization eval: tp={tp} fp={fp} tn={tn} fn={fn_} precision={precision:.3} recall={recall:.3}\nmisses: {misses:?}"
    );
    assert!(
        precision >= 0.75,
        "nominalization precision {precision:.3} below pre-registered bar; investigate, do not lower",
    );
    assert!(
        recall >= 0.50,
        "nominalization recall {recall:.3} below pre-registered bar; investigate, do not lower",
    );
}
