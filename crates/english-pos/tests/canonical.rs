//! Canonical-veto regression pin: `Time flies like an arrow` must
//! keep `flies`/VERB across weight retrains and correction-rule
//! changes. The iters=15 sweep peak flipped it (plural `-ies` +
//! noun-noun `t-1` memorization outvoting its single VERB observation
//! in EWT); see the train README and the `forensics` example. This
//! test makes that veto permanent on both the raw decode and the
//! corrected path.

use english_pos::{Model, RULES, Tag, apply_rules};

#[test]
fn flies_stays_verb_decode() {
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let tags = model.tag(&["Time", "flies", "like", "an", "arrow"]);
    assert_eq!(
        tags,
        vec![Tag::Noun, Tag::Verb, Tag::Adp, Tag::Det, Tag::Noun]
    );
}

#[test]
fn flies_stays_verb_corrected() {
    // Whatever RULES holds, the corrected path must not move `flies`.
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let words = ["Time", "flies", "like", "an", "arrow"];
    let pieces: Vec<String> = words.iter().map(|s| s.to_string()).collect();
    let mut tagged = model.tag_margins(&words);
    let lower: Vec<String> = pieces.iter().map(|p| p.to_lowercase()).collect();
    apply_rules(&mut tagged, RULES, &lower);
    let tags: Vec<Tag> = tagged.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        tags,
        vec![Tag::Noun, Tag::Verb, Tag::Adp, Tag::Det, Tag::Noun]
    );
}

#[test]
fn flies_stays_verb_beam() {
    // The veto covers the beam decoder too: admission requires the
    // joint re-decode to keep the canonical reading (its `-ies`
    // memorization pressure is exactly what flipped the iters=15
    // weights).
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let tags = model.tag_beam(&["Time", "flies", "like", "an", "arrow"]);
    assert_eq!(
        tags,
        vec![Tag::Noun, Tag::Verb, Tag::Adp, Tag::Det, Tag::Noun]
    );
}
