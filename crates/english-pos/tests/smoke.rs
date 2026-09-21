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
