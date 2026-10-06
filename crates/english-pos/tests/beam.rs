//! Beam re-decode tests: structural properties (no behavior pins
//! here — accuracy is measured in the trainer `--beam` sweep on EWT
//! dev/test plus the evals; behavior pins would freeze tuning).
//!
//! What these pin: beam ≡ greedy at the degenerate settings
//! (threshold 0, cap 0), output tiling, span-stat consistency, and
//! no-panic edges (empty input, single token, sentence start).

use english_pos::Model;

fn model() -> Model {
    Model::from_json(include_str!("../weights/upos.json")).unwrap()
}

fn sents() -> Vec<Vec<&'static str>> {
    vec![
        vec!["Time", "flies", "like", "an", "arrow", "."],
        vec!["Call", "me", "Ishmael", "."],
        vec![
            "The",
            "committee",
            "demands",
            "that",
            "it",
            "be",
            "published",
            ".",
        ],
        vec![
            "There",
            "are",
            "several",
            "explanations",
            "for",
            "this",
            "pattern",
            ".",
        ],
        vec!["Go"],
    ]
}

#[test]
fn zero_threshold_is_greedy() {
    // No margin is < 0.0, so no span can trigger at any cap.
    let m = model();
    for s in sents() {
        let (beam, spans, rescored) = m.tag_beam_with(&s, 0.0, 8);
        assert_eq!(beam, m.tag(&s));
        assert_eq!((spans, rescored), (0, 0));
    }
}

#[test]
fn zero_cap_is_greedy() {
    // Every span (length ≥ 1) exceeds the cap, so greedy stands.
    let m = model();
    for s in sents() {
        let (beam, spans, rescored) = m.tag_beam_with(&s, f32::INFINITY, 0);
        assert_eq!(beam, m.tag(&s));
        assert_eq!((spans, rescored), (0, 0));
    }
}

#[test]
fn output_tiles_stats_consistent() {
    let m = model();
    for s in sents() {
        let (beam, spans, rescored) = m.tag_beam_with(&s, 1.0, 8);
        assert_eq!(beam.len(), s.len());
        assert!(rescored <= s.len());
        if spans > 0 {
            assert!(rescored > 0);
        }
    }
}

#[test]
fn empty_and_single_token() {
    let m = model();
    let (beam, spans, rescored): (Vec<english_pos::Tag>, usize, usize) =
        m.tag_beam_with(&Vec::<&str>::new(), 1.0, 8);
    assert!(beam.is_empty());
    assert_eq!((spans, rescored), (0, 0));
    // Single token at sentence start exercises the START1/START2
    // boundary histories.
    let (beam, _, _) = m.tag_beam_with(&["Go"], f32::INFINITY, 8);
    assert_eq!(beam.len(), 1);
}
