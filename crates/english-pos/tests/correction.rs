//! Correction-engine unit tests: firing, gating, and non-matches.
//! Accuracy impact is measured separately (trainer `--correct`
//! reporting on EWT dev plus the Moby evals); these pin engine
//! behavior using a local toy rule (no production rule has passed
//! admission yet — see the rejection note in `correction.rs`).

use english_pos::{Rule, Tag, apply_rules};

/// Toy rule: the exact word `glitters` tagged NOUN becomes VERB.
const TOY: Rule = Rule {
    name: "toy",
    threshold: 2.0,
    test: |pieces, tags, i| {
        if tags[i] == Tag::Noun && pieces[i] == "glitters" {
            Some(Tag::Verb)
        } else {
            None
        }
    },
};

fn case(texts: &[&str], tags: &[(Tag, f32)]) -> (Vec<String>, Vec<(Tag, f32)>) {
    (texts.iter().map(|s| s.to_string()).collect(), tags.to_vec())
}

#[test]
fn rule_fires_inside_gate() {
    let (pieces, mut tagged) = case(
        &["Orion", "glitters", "tonight"],
        &[(Tag::Noun, 0.0), (Tag::Noun, 1.0), (Tag::Adv, 0.0)],
    );
    apply_rules(&pieces, &mut tagged, &[TOY]);
    assert_eq!(
        tagged.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
        vec![Tag::Noun, Tag::Verb, Tag::Adv]
    );
}

#[test]
fn gate_blocks_confident_tokens() {
    let (pieces, mut tagged) = case(
        &["Orion", "glitters", "tonight"],
        &[(Tag::Noun, 0.0), (Tag::Noun, 5.0), (Tag::Adv, 0.0)],
    );
    apply_rules(&pieces, &mut tagged, &[TOY]);
    assert_eq!(tagged[1].0, Tag::Noun);
}

#[test]
fn gate_blocks_exact_ties() {
    // Margin 0 carries no model signal; rules may override weak
    // signals, never invent from silence.
    let (pieces, mut tagged) = case(
        &["Orion", "glitters", "tonight"],
        &[(Tag::Noun, 0.0), (Tag::Noun, 0.0), (Tag::Adv, 0.0)],
    );
    apply_rules(&pieces, &mut tagged, &[TOY]);
    assert_eq!(tagged[1].0, Tag::Noun);
}

#[test]
fn non_matching_tokens_untouched() {
    let (pieces, mut tagged) = case(
        &["the", "glass", "broke"],
        &[(Tag::Det, 0.0), (Tag::Noun, 1.0), (Tag::Verb, 0.0)],
    );
    apply_rules(&pieces, &mut tagged, &[TOY]);
    assert_eq!(
        tagged.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
        vec![Tag::Det, Tag::Noun, Tag::Verb]
    );
}

#[test]
fn first_matching_rule_wins() {
    let other = Rule {
        name: "other",
        threshold: 2.0,
        test: |_, tags, i| {
            if tags[i] == Tag::Noun {
                Some(Tag::Adj)
            } else {
                None
            }
        },
    };
    let (pieces, mut tagged) = case(
        &["Orion", "glitters", "tonight"],
        &[(Tag::Noun, 0.0), (Tag::Noun, 1.0), (Tag::Adv, 0.0)],
    );
    apply_rules(&pieces, &mut tagged, &[TOY, other]);
    assert_eq!(tagged[1].0, Tag::Verb);
    let (pieces, mut tagged) = case(
        &["Orion", "glitters", "tonight"],
        &[(Tag::Noun, 0.0), (Tag::Noun, 1.0), (Tag::Adv, 0.0)],
    );
    apply_rules(&pieces, &mut tagged, &[other, TOY]);
    assert_eq!(tagged[1].0, Tag::Adj);
}
