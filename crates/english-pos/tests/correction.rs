//! Correction-engine unit tests: firing, gating, and non-matches.
//! Accuracy impact is measured separately (trainer `--correct`
//! reporting on EWT dev plus the Moby evals); these pin engine
//! behavior using a local toy rule (no production rule has passed
//! admission yet — see the rejection note in `correction.rs`).

use english_pos::{RULES, Rule, Tag, apply_rules};

/// Local stand-in for the rejected `imperative-0` shape (pos-0 NOUN
/// + complement next → VERB): exercises engine paths without shipping
/// the rule. See `correction.rs` for why it stays out of `RULES`.
const POS0: Rule = Rule {
    name: "pos0-toy",
    threshold: 2.0,
    test: |pieces, tags, i| {
        let alpha = pieces[i]
            .to_lowercase()
            .chars()
            .all(|c| c.is_ascii_lowercase());
        if i == 0 && tags[i] == Tag::Noun && alpha {
            match tags.get(1).copied() {
                Some(Tag::Det) | Some(Tag::Adj) | Some(Tag::Adp) | Some(Tag::Pron) => {
                    Some(Tag::Verb)
                }
                _ => None,
            }
        } else {
            None
        }
    },
};

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

#[test]
fn imperative_fires_on_base_verb_with_complement() {
    // `Look at ...` decoded as NOUN with a weak margin flips.
    let (pieces, mut tagged) = case(
        &["Look", "at", "the", "crowds"],
        &[
            (Tag::Noun, 1.0),
            (Tag::Adp, 0.0),
            (Tag::Det, 0.0),
            (Tag::Noun, 0.0),
        ],
    );
    apply_rules(&pieces, &mut tagged, &[POS0]);
    assert_eq!(tagged[0].0, Tag::Verb);
}

#[test]
fn imperative_skips_non_verbs_and_verb_next() {
    // Gerund shapes never qualify ...
    let (pieces, mut tagged) = case(&["Morning", "came"], &[(Tag::Noun, 1.0), (Tag::Verb, 0.0)]);
    apply_rules(&pieces, &mut tagged, &[POS0]);
    assert_eq!(tagged[0].0, Tag::Noun);
    // ... and neither does a following verb (`Glass breaks`).
    let (pieces, mut tagged) = case(&["Glass", "breaks"], &[(Tag::Noun, 1.0), (Tag::Verb, 0.0)]);
    apply_rules(&pieces, &mut tagged, &[POS0]);
    assert_eq!(tagged[0].0, Tag::Noun);
}

/// Fetch a candidate rule by name (tests the production predicate,
// not a toy). Rules here are under measurement — see `correction.rs`.
fn rule(name: &str) -> Rule {
    *RULES
        .iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("candidate rule missing: {name}"))
}

fn run(pieces: &[&str], tags: &[(Tag, f32)], name: &str) -> Vec<Tag> {
    let pieces: Vec<String> = pieces.iter().map(|s| s.to_string()).collect();
    let mut tagged = tags.to_vec();
    apply_rules(&pieces, &mut tagged, &[rule(name)]);
    tagged.iter().map(|(t, _)| *t).collect()
}

#[test]
fn to_verb_fixes_infinitive_head() {
    assert_eq!(
        run(
            &["to", "approve"],
            &[(Tag::Part, 0.0), (Tag::Noun, 1.0)],
            "to-verb",
        )[1],
        Tag::Verb
    );
    // Genuine nominal after `to` stays (`to Detroit` shape).
    assert_eq!(
        run(
            &["to", "Detroit"],
            &[(Tag::Part, 0.0), (Tag::Noun, 1.0)],
            "to-verb",
        )[1],
        Tag::Noun
    );
}

#[test]
fn function_words_disambiguate() {
    // `to Coenties`: PART before nominal → ADP; `to approve`
    // abstains via the verb-stem guard (leaves it for `to-verb`).
    assert_eq!(
        run(
            &["to", "Coenties"],
            &[(Tag::Part, 1.0), (Tag::Propn, 0.0)],
            "to-prep",
        )[0],
        Tag::Adp
    );
    assert_eq!(
        run(
            &["to", "approve"],
            &[(Tag::Part, 1.0), (Tag::Noun, 0.0)],
            "to-prep",
        )[0],
        Tag::Part
    );
    // Possessive `have`: AUX before nominal → VERB; `have been`
    // and `have to go` stay.
    assert_eq!(
        run(
            &["have", "of"],
            &[(Tag::Aux, 1.0), (Tag::Adp, 0.0)],
            "have-verb",
        )[0],
        Tag::Verb
    );
    for next in [("been", Tag::Aux), ("to", Tag::Part)] {
        assert_eq!(
            run(
                &["have", next.0],
                &[(Tag::Aux, 1.0), (next.1, 0.0)],
                "have-verb",
            )[0],
            Tag::Aux,
            "have {} stays",
            next.0
        );
    }
}

#[test]
fn that_det_fixes_determiner_that() {
    // `of that slouching snow`: PRON after ADP before nominal → DET.
    assert_eq!(
        run(
            &["of", "that", "snow"],
            &[(Tag::Adp, 5.0), (Tag::Pron, 1.0), (Tag::Noun, 9.0)],
            "that-det",
        )[1],
        Tag::Det
    );
    // Sentence-initial `That man ...` (no prev) fires too.
    assert_eq!(
        run(
            &["that", "man"],
            &[(Tag::Pron, 1.0), (Tag::Noun, 9.0)],
            "that-det",
        )[0],
        Tag::Det
    );
    // Relative clause (`all that glitters`: VERB next) abstains.
    assert_eq!(
        run(
            &["all", "that", "glitters"],
            &[(Tag::Pron, 4.0), (Tag::Pron, 1.0), (Tag::Verb, 9.0)],
            "that-det",
        )[1],
        Tag::Pron
    );
    // Non-preposition prev abstains even when the model says PRON:
    // `said that men rejoice` is a complement clause, `saw that man`
    // a determiner — the guard stays out of genuinely ambiguous
    // territory (EWT never shows DET there the way ADP-prev does).
    assert_eq!(
        run(
            &["said", "that", "men"],
            &[(Tag::Verb, 9.0), (Tag::Pron, 1.0), (Tag::Noun, 9.0)],
            "that-det",
        )[1],
        Tag::Pron
    );
}
