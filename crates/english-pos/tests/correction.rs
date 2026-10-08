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
    test: |tags, low, i| {
        let alpha = low[i].chars().all(|c| c.is_ascii_lowercase());
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
    test: |tags, low, i| {
        if tags[i] == Tag::Noun && low[i] == "glitters" {
            Some(Tag::Verb)
        } else {
            None
        }
    },
};

fn case(texts: &[&str], tags: &[(Tag, f32)]) -> (Vec<String>, Vec<(Tag, f32)>) {
    (texts.iter().map(|s| s.to_string()).collect(), tags.to_vec())
}

/// Lowercased pieces for rule shape tests (test-only helper; production
/// reuses the decoder's copy — see `tag_beam_margins_lowered`).
fn low_of(pieces: &[String]) -> Vec<String> {
    pieces.iter().map(|p| p.to_lowercase()).collect()
}

#[test]
fn rule_fires_inside_gate() {
    let (pieces, mut tagged) = case(
        &["Orion", "glitters", "tonight"],
        &[(Tag::Noun, 0.0), (Tag::Noun, 1.0), (Tag::Adv, 0.0)],
    );
    apply_rules(&mut tagged, &[TOY], &low_of(&pieces));
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
    apply_rules(&mut tagged, &[TOY], &low_of(&pieces));
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
    apply_rules(&mut tagged, &[TOY], &low_of(&pieces));
    assert_eq!(tagged[1].0, Tag::Noun);
}

#[test]
fn non_matching_tokens_untouched() {
    let (pieces, mut tagged) = case(
        &["the", "glass", "broke"],
        &[(Tag::Det, 0.0), (Tag::Noun, 1.0), (Tag::Verb, 0.0)],
    );
    apply_rules(&mut tagged, &[TOY], &low_of(&pieces));
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
        test: |tags, _low, i| {
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
    apply_rules(&mut tagged, &[TOY, other], &low_of(&pieces));
    assert_eq!(tagged[1].0, Tag::Verb);
    let (pieces, mut tagged) = case(
        &["Orion", "glitters", "tonight"],
        &[(Tag::Noun, 0.0), (Tag::Noun, 1.0), (Tag::Adv, 0.0)],
    );
    apply_rules(&mut tagged, &[other, TOY], &low_of(&pieces));
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
    apply_rules(&mut tagged, &[POS0], &low_of(&pieces));
    assert_eq!(tagged[0].0, Tag::Verb);
}

#[test]
fn imperative_skips_non_verbs_and_verb_next() {
    // Gerund shapes never qualify ...
    let (pieces, mut tagged) = case(&["Morning", "came"], &[(Tag::Noun, 1.0), (Tag::Verb, 0.0)]);
    apply_rules(&mut tagged, &[POS0], &low_of(&pieces));
    assert_eq!(tagged[0].0, Tag::Noun);
    // ... and neither does a following verb (`Glass breaks`).
    let (pieces, mut tagged) = case(&["Glass", "breaks"], &[(Tag::Noun, 1.0), (Tag::Verb, 0.0)]);
    apply_rules(&mut tagged, &[POS0], &low_of(&pieces));
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
    apply_rules(&mut tagged, &[rule(name)], &low_of(&pieces));
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

#[test]
fn that_rel_fixes_relativizer_that() {
    // `the book that sells`: SCONJ before a verb → PRON.
    assert_eq!(
        run(
            &["book", "that", "sells"],
            &[(Tag::Noun, 9.0), (Tag::Sconj, 1.0), (Tag::Verb, 9.0)],
            "that-rel",
        )[1],
        Tag::Pron
    );
    // Complementizer (`I know that men rejoice`: NOUN next) abstains.
    assert_eq!(
        run(
            &["know", "that", "men"],
            &[(Tag::Verb, 9.0), (Tag::Sconj, 1.0), (Tag::Noun, 9.0)],
            "that-rel",
        )[1],
        Tag::Sconj
    );
}

#[test]
fn det_noun_fixes_plural_s_noun() {
    // `the glitters`: VERB after DET+barrier → NOUN.
    assert_eq!(
        run(
            &["the", "glitters"],
            &[(Tag::Det, 9.0), (Tag::Verb, 1.0)],
            "det-noun",
        )[1],
        Tag::Noun
    );
    // Genuine 3sg verb (`she glitters`: PRON prev, no DET) abstains.
    assert_eq!(
        run(
            &["she", "glitters"],
            &[(Tag::Pron, 9.0), (Tag::Verb, 1.0)],
            "det-noun",
        )[1],
        Tag::Verb
    );
}

#[test]
fn that_sconj_fixes_complementizer_that() {
    // `I know that big dogs bark`: PRON before DET+ADJ → SCONJ.
    assert_eq!(
        run(
            &["know", "that", "big", "dogs"],
            &[
                (Tag::Verb, 9.0),
                (Tag::Pron, 1.0),
                (Tag::Det, 9.0),
                (Tag::Adj, 9.0)
            ],
            "that-sconj",
        )[1],
        Tag::Sconj
    );
    // Bare nominal (`of that slouching snow`: ADJ+NOUN) abstains —
    // that is `that-det` territory, not a clause.
    assert_eq!(
        run(
            &["of", "that", "slouching", "snow"],
            &[
                (Tag::Adp, 9.0),
                (Tag::Pron, 1.0),
                (Tag::Adj, 9.0),
                (Tag::Noun, 9.0)
            ],
            "that-sconj",
        )[1],
        Tag::Pron
    );
}

#[test]
fn that_ccomp_fixes_verb_complement_that() {
    // `said that men rejoice`: PRON after VERB, nominal next, finite
    // verb ahead → SCONJ.
    assert_eq!(
        run(
            &["said", "that", "men", "rejoice"],
            &[
                (Tag::Verb, 9.0),
                (Tag::Pron, 1.0),
                (Tag::Noun, 9.0),
                (Tag::Verb, 9.0)
            ],
            "that-ccomp",
        )[1],
        Tag::Sconj
    );
    // Determiner (`saw that man`, no verb ahead) abstains.
    assert_eq!(
        run(
            &["saw", "that", "man"],
            &[(Tag::Verb, 9.0), (Tag::Pron, 1.0), (Tag::Noun, 9.0),],
            "that-ccomp",
        )[1],
        Tag::Pron
    );
}

#[test]
fn subconj_adp_fixes_plain_preposition() {
    // `after the war`: SCONJ with nominal next, no verb ahead → ADP.
    assert_eq!(
        run(
            &["after", "the", "war"],
            &[(Tag::Sconj, 1.0), (Tag::Det, 9.0), (Tag::Noun, 9.0)],
            "subconj-adp",
        )[0],
        Tag::Adp
    );
    // Clausal (`after the war ended`: verb ahead) abstains.
    assert_eq!(
        run(
            &["after", "the", "war", "ended"],
            &[
                (Tag::Sconj, 1.0),
                (Tag::Det, 9.0),
                (Tag::Noun, 9.0),
                (Tag::Verb, 9.0)
            ],
            "subconj-adp",
        )[0],
        Tag::Sconj
    );
}

#[test]
fn apos_part_fixes_possessive_s() {
    // `Daggoo's hat`: AUX after PROPN → PART.
    assert_eq!(
        run(
            &["Daggoo", "'s", "hat"],
            &[(Tag::Propn, 9.0), (Tag::Aux, 1.0), (Tag::Noun, 9.0)],
            "apos-part",
        )[1],
        Tag::Part
    );
    // Copula (`it's late`: PRON prev) abstains.
    assert_eq!(
        run(
            &["it", "'s", "late"],
            &[(Tag::Pron, 9.0), (Tag::Aux, 1.0), (Tag::Adj, 9.0)],
            "apos-part",
        )[1],
        Tag::Aux
    );
}

#[test]
fn to_part_fixes_infinitive_marker() {
    // `want to go`: ADP before a verb → PART.
    assert_eq!(
        run(
            &["want", "to", "go"],
            &[(Tag::Verb, 9.0), (Tag::Adp, 1.0), (Tag::Verb, 9.0)],
            "to-part",
        )[1],
        Tag::Part
    );
    // Genuine preposition (`go to school`: NOUN next) abstains.
    assert_eq!(
        run(
            &["go", "to", "school"],
            &[(Tag::Verb, 9.0), (Tag::Adp, 1.0), (Tag::Noun, 9.0)],
            "to-part",
        )[1],
        Tag::Adp
    );
}

#[test]
fn that_vcomp_fixes_verb_complement_that() {
    // `said that the answer`: PRON after VERB before DET → SCONJ.
    assert_eq!(
        run(
            &["said", "that", "the", "answer"],
            &[
                (Tag::Verb, 9.0),
                (Tag::Pron, 1.0),
                (Tag::Det, 9.0),
                (Tag::Noun, 9.0)
            ],
            "that-vcomp",
        )[1],
        Tag::Sconj
    );
    // Reduced relative (`the way that the group ...`: NOUN prev)
    // abstains.
    assert_eq!(
        run(
            &["way", "that", "the", "group"],
            &[
                (Tag::Noun, 9.0),
                (Tag::Pron, 1.0),
                (Tag::Det, 9.0),
                (Tag::Noun, 9.0)
            ],
            "that-vcomp",
        )[1],
        Tag::Pron
    );
}

#[test]
fn pass_by_fixes_agented_participle() {
    // `was broken by X`: ADJ after be-AUX with by-agent ahead → VERB.
    assert_eq!(
        run(
            &["was", "broken", "by", "them"],
            &[
                (Tag::Aux, 9.0),
                (Tag::Adj, 1.0),
                (Tag::Adp, 9.0),
                (Tag::Pron, 9.0)
            ],
            "pass-by",
        )[1],
        Tag::Verb
    );
    // Bare stative (no agent): abstains — `was tired` stays ADJ.
    assert_eq!(
        run(
            &["was", "tired"],
            &[(Tag::Aux, 9.0), (Tag::Adj, 1.0)],
            "pass-by",
        )[1],
        Tag::Adj
    );
    // Non-participle ADJ with agent (`was glad by...` — no ed/en):
    // abstains; the morphology guard is load-bearing (EWT 130:4 → 110:0).
    assert_eq!(
        run(
            &["was", "glad", "by", "them"],
            &[
                (Tag::Aux, 9.0),
                (Tag::Adj, 1.0),
                (Tag::Adp, 9.0),
                (Tag::Pron, 9.0)
            ],
            "pass-by",
        )[1],
        Tag::Adj
    );
    // PUNCT barrier: `was tired. By morning...` — no cross-sentence agent.
    assert_eq!(
        run(
            &["was", "tired", ".", "by", "then"],
            &[
                (Tag::Aux, 9.0),
                (Tag::Adj, 1.0),
                (Tag::Punct, 9.0),
                (Tag::Adp, 9.0),
                (Tag::Adv, 9.0)
            ],
            "pass-by",
        )[1],
        Tag::Adj
    );
    // Non-be AUX prev (`seemed broken by...` — seemed is VERB): abstains.
    assert_eq!(
        run(
            &["seemed", "broken", "by", "them"],
            &[
                (Tag::Verb, 9.0),
                (Tag::Adj, 1.0),
                (Tag::Adp, 9.0),
                (Tag::Pron, 9.0)
            ],
            "pass-by",
        )[1],
        Tag::Adj
    );
}

#[test]
fn quite_adv_fixes_determiner_quite() {
    // `quite sure`: DET before ADJ → ADV (EWT 27:0 in this shape).
    assert_eq!(
        run(
            &["quite", "sure"],
            &[(Tag::Det, 1.0), (Tag::Adj, 9.0)],
            "quite-adv",
        )[0],
        Tag::Adv
    );
    // Determiner use (`quite a few`) abstains: DET next, not ADJ/ADV.
    assert_eq!(
        run(
            &["quite", "a", "few"],
            &[(Tag::Det, 1.0), (Tag::Det, 9.0), (Tag::Adj, 9.0)],
            "quite-adv",
        )[0],
        Tag::Det
    );
    // Already ADV: nothing to do.
    assert_eq!(
        run(
            &["quite", "sure"],
            &[(Tag::Adv, 1.0), (Tag::Adj, 9.0)],
            "quite-adv",
        )[0],
        Tag::Adv
    );
}

#[test]
fn those_pron_fixes_elliptical_those() {
    // `those of you`: DET before ADP → PRON (EWT train 7:0).
    assert_eq!(
        run(
            &["those", "of", "you"],
            &[(Tag::Det, 1.0), (Tag::Adp, 9.0), (Tag::Pron, 9.0)],
            "those-pron",
        )[0],
        Tag::Pron
    );
    // Determiner use (`those books`) abstains: NOUN next.
    assert_eq!(
        run(
            &["those", "books", "are"],
            &[(Tag::Det, 1.0), (Tag::Noun, 9.0), (Tag::Aux, 9.0)],
            "those-pron",
        )[0],
        Tag::Det
    );
    // ADJ next abstains (DET 14:5 in EWT — no majority).
    assert_eq!(
        run(
            &["those", "yellow", "creatures"],
            &[(Tag::Det, 1.0), (Tag::Adj, 9.0), (Tag::Noun, 9.0)],
            "those-pron",
        )[0],
        Tag::Det
    );
    // Already PRON: nothing to do.
    assert_eq!(
        run(
            &["those", "of", "you"],
            &[(Tag::Pron, 1.0), (Tag::Adp, 9.0), (Tag::Pron, 9.0)],
            "those-pron",
        )[0],
        Tag::Pron
    );
}

#[test]
fn there_adv_fixes_final_locative_there() {
    // `water-gazers there`: PRON in last two positions -> ADV
    // (EWT train 22:0; existentials lead sentences instead).
    assert_eq!(
        run(
            &["gazers", "there"],
            &[(Tag::Noun, 9.0), (Tag::Pron, 1.0)],
            "there-adv",
        )[1],
        Tag::Adv
    );
    // Sentence-initial existential abstains (not final).
    assert_eq!(
        run(
            &["there", "stand", "trees"],
            &[(Tag::Pron, 1.0), (Tag::Verb, 9.0), (Tag::Noun, 9.0)],
            "there-adv",
        )[0],
        Tag::Pron
    );
    // Already ADV: nothing to do.
    assert_eq!(
        run(
            &["go", "there"],
            &[(Tag::Verb, 9.0), (Tag::Adv, 1.0)],
            "there-adv",
        )[1],
        Tag::Adv
    );
    // Mid-sentence abstains.
    assert_eq!(
        run(
            &["there", "sleep", "cattle"],
            &[(Tag::Pron, 1.0), (Tag::Verb, 9.0), (Tag::Noun, 9.0)],
            "there-adv",
        )[0],
        Tag::Pron
    );
}

#[test]
fn be_aux_fixes_infinitive_be() {
    // `seems to be a duchess`: VERB after `to` -> AUX (EWT 38:5).
    assert_eq!(
        run(
            &["seems", "to", "be", "a"],
            &[
                (Tag::Verb, 9.0),
                (Tag::Part, 9.0),
                (Tag::Verb, 1.0),
                (Tag::Det, 9.0)
            ],
            "be-aux",
        )[2],
        Tag::Aux
    );
    // Modal-prev stays out (thin and mixed in EWT: `can be` 3:2).
    assert_eq!(
        run(
            &["can", "be", "done"],
            &[(Tag::Aux, 9.0), (Tag::Verb, 1.0), (Tag::Verb, 9.0)],
            "be-aux",
        )[1],
        Tag::Verb
    );
    // Imperative `be` (no prev) abstains.
    assert_eq!(
        run(
            &["be", "quiet"],
            &[(Tag::Verb, 1.0), (Tag::Adj, 9.0)],
            "be-aux",
        )[0],
        Tag::Verb
    );
    // Above-gate margin abstains even in the shape.
    assert_eq!(
        run(
            &["to", "be", "fair"],
            &[(Tag::Part, 9.0), (Tag::Verb, 5.0), (Tag::Adj, 9.0)],
            "be-aux",
        )[1],
        Tag::Verb
    );
}
