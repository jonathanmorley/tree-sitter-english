//! Wiring tests: parse prose with the `english` crate, convert to UD
//! pieces, and tag with the committed weights.
//!
//! The model itself is unchanged (dev 90.35% / test 90.53% on UD EWT;
//! Moby-Dick sample bar ~80%), so these tests pin the wiring behavior:
//! no silently dropped tokens, UD contraction splits, and sane tags on
//! canonical input.

use english_pos::{Model, Tag, TagCache, sentence_pieces, split_contraction, tag_sentence};

fn model() -> Model {
    Model::from_json(include_str!("../weights/upos.json")).unwrap()
}

fn first_sentence(text: &str) -> english::Document {
    english::Document::parse(text)
}

#[test]
fn split_contraction_cases() {
    assert_eq!(split_contraction("arrow"), vec!["arrow".to_string()]);
    assert_eq!(
        split_contraction("don't"),
        vec!["do".to_string(), "n't".to_string()]
    );
    assert_eq!(
        split_contraction("ship’s"),
        vec!["ship".to_string(), "'s".to_string()]
    );
    assert_eq!(
        split_contraction("orator's"),
        vec!["orator".to_string(), "'s".to_string()]
    );
    assert_eq!(split_contraction("'em"), vec!["'em".to_string()]);
    assert_eq!(
        split_contraction("dogs'"),
        vec!["dogs".to_string(), "'".to_string()]
    );
}

#[test]
fn sentence_pieces_keep_joiners() {
    let doc = first_sentence("I like tea and coffee.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    assert_eq!(
        sentence_pieces(&sent),
        vec![
            "I".to_string(),
            "like".to_string(),
            "tea".to_string(),
            "and".to_string(),
            "coffee".to_string()
        ]
    );

    let doc = first_sentence("He left because he was tired.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    let pieces = sentence_pieces(&sent);
    assert!(pieces.contains(&"because".to_string()), "{pieces:?}");
    assert_eq!(
        pieces,
        vec![
            "He".to_string(),
            "left".to_string(),
            "because".to_string(),
            "he".to_string(),
            "was".to_string(),
            "tired".to_string()
        ]
    );
}

#[test]
fn sentence_pieces_split_contractions() {
    let doc = first_sentence("I don't know.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    assert_eq!(
        sentence_pieces(&sent),
        vec![
            "I".to_string(),
            "do".to_string(),
            "n't".to_string(),
            "know".to_string()
        ]
    );
}

#[test]
fn sentence_pieces_flatten_parentheticals() {
    let doc = first_sentence("I saw it (the truth) clearly.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    assert_eq!(
        sentence_pieces(&sent),
        vec![
            "I".to_string(),
            "saw".to_string(),
            "it".to_string(),
            "the".to_string(),
            "truth".to_string(),
            "clearly".to_string()
        ]
    );
}

#[test]
fn tag_canonical_sentence() {
    // Mirrors the smoke-test canonical tags, minus the hidden terminal
    // period (no named node): wiring must preserve the word tags.
    let model = model();
    let doc = first_sentence("Time flies like an arrow.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    let tagged = tag_sentence(&model, &sent);
    let texts: Vec<&str> = tagged.iter().map(|(w, _)| w.as_str()).collect();
    let tags: Vec<Tag> = tagged.iter().map(|(_, t)| *t).collect();
    assert_eq!(texts, vec!["Time", "flies", "like", "an", "arrow"]);
    assert_eq!(
        tags,
        vec![Tag::Noun, Tag::Verb, Tag::Adp, Tag::Det, Tag::Noun]
    );
}

#[test]
fn tag_cache_reuses_identical_sentences() {
    let model = model();
    let mut cache = TagCache::new();
    let doc = first_sentence("Time flies like an arrow.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    let first = cache.tag_sentence(&model, &sent);
    assert_eq!((cache.hits(), cache.misses()), (0, 1));
    // Same text re-parsed: hit, identical pairs.
    let doc2 = first_sentence("Time flies like an arrow.\n");
    let sent2 = doc2.paragraphs()[0].sentences()[0];
    let second = cache.tag_sentence(&model, &sent2);
    assert_eq!((cache.hits(), cache.misses()), (1, 1));
    assert_eq!(first, second);
    // Changed text: miss.
    let doc3 = first_sentence("Time crawls like an arrow.\n");
    let sent3 = doc3.paragraphs()[0].sentences()[0];
    cache.tag_sentence(&model, &sent3);
    assert_eq!((cache.hits(), cache.misses()), (1, 2));
}

#[test]
fn tag_sentence_marks_joiners() {
    let model = model();
    let doc = first_sentence("I like tea and coffee.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    let tagged = tag_sentence(&model, &sent);
    let and_tag = tagged.iter().find(|(w, _)| w == "and").map(|(_, t)| *t);
    assert_eq!(and_tag, Some(Tag::Cconj));

    let doc = first_sentence("He left because he was tired.\n");
    let sent = doc.paragraphs()[0].sentences()[0];
    let tagged = tag_sentence(&model, &sent);
    let because_tag = tagged.iter().find(|(w, _)| w == "because").map(|(_, t)| *t);
    assert_eq!(because_tag, Some(Tag::Sconj));
}
