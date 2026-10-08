//! Inference smoke tests against committed weights plus a memorization
//! regression test for the trainer itself.

use english_pos::{Model, Tag};

fn tiny_data() -> Vec<(Vec<String>, Vec<String>)> {
    vec![(
        vec!["the".into(), "dog".into(), "ran".into()],
        vec!["DET".into(), "NOUN".into(), "VERB".into()],
    )]
}

#[test]
fn trainer_memorizes_tiny_data() {
    let data = tiny_data();
    let model = Model::train(&data, 20, 1);
    for (words, gold) in &data {
        let got: Vec<String> = model
            .tag(words)
            .iter()
            .map(|t| t.upos().to_string())
            .collect();
        assert_eq!(&got, gold);
    }
}

#[test]
fn canonical_sentence_tags() {
    // Against committed weights: the garden-path classic. If the model
    // changes, update these deliberately, not blindly.
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let words = ["Time", "flies", "like", "an", "arrow", "."];
    let got: Vec<Tag> = model.tag(&words);
    assert_eq!(
        got,
        vec![
            Tag::Noun,
            Tag::Verb,
            Tag::Adp,
            Tag::Det,
            Tag::Noun,
            Tag::Punct
        ]
    );
}

#[test]
fn tag_enum_roundtrips() {
    assert_eq!(Tag::from_upos("NOUN"), Some(Tag::Noun));
    assert_eq!(Tag::Noun.upos(), "NOUN");
    assert_eq!(Tag::from_upos("BOGUS"), None);
}

#[test]
fn finetune_teaches_new_mapping() {
    // Base model knows "dog" as NOUN; a conflicting in-domain pass
    // moves it (few passes, tiny data — direction, not convergence).
    let data = tiny_data();
    let mut model = Model::train(&data, 20, 1);
    assert_eq!(model.tag(&["dog"]), vec![Tag::Noun]);
    let fix = vec![(vec!["dog".to_string()], vec!["VERB".to_string()])];
    model.finetune(&fix, 5);
    assert_eq!(model.tag(&["dog"]), vec![Tag::Verb]);
}

#[test]
fn tag_margins_agree_with_tags() {
    let model = Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let words = ["Time", "flies", "like", "an", "arrow", "."];
    let tags = model.tag(&words);
    let margined = model.tag_margins(&words);
    assert_eq!(margined.len(), tags.len());
    for ((t, m), expected) in margined.iter().zip(&tags) {
        assert_eq!(t, expected);
        assert!(*m >= 0.0, "margin is best minus runner-up");
        assert!(m.is_finite());
    }
    // Dict-skipped tokens (`flies`, `.` — unambiguous in training)
    // carry the documented placeholder, not a measured gap; the
    // ambiguous-vs-closed-class ordering holds among scored tokens
    // (`Time` 3 < `an` 26 — garden-path subject vs determiner).
    assert_eq!(margined[1].1, english_pos::FAST_PATH_MARGIN);
    assert_eq!(margined[5].1, english_pos::FAST_PATH_MARGIN);
    let time_margin = margined[0].1;
    let an_margin = margined[3].1;
    assert!(
        time_margin < an_margin,
        "time {time_margin} vs an {an_margin}"
    );
}
