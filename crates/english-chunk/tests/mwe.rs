//! MWE unit tests over hand-tagged piece sequences (same discipline
//! as `basic.rs`: gold tags isolate chunker behavior from tagger
//! errors).

use english_chunk::{ChunkKind, chunk_tagged};
use english_pos::Tag;

fn pieces(pairs: &[(&str, Tag)]) -> Vec<(String, Tag)> {
    pairs.iter().map(|(w, t)| (w.to_string(), *t)).collect()
}

fn kinds_of(input: &[(String, Tag)]) -> Vec<ChunkKind> {
    chunk_tagged(input).iter().map(|c| c.kind()).collect()
}

#[test]
fn subordinators_merge() {
    // `so that` / `as if` / `such as` each chunk as one Subord.
    for words in [
        vec![("so", Tag::Sconj), ("that", Tag::Sconj)],
        vec![("as", Tag::Sconj), ("if", Tag::Sconj)],
        vec![("such", Tag::Adj), ("as", Tag::Sconj)],
    ] {
        let input = pieces(&words);
        let chunks = chunk_tagged(&input);
        assert_eq!(kinds_of(&input), vec![ChunkKind::Subord], "{words:?}");
        assert_eq!(chunks[0].span(), 0..words.len());
    }
}

#[test]
fn preps_and_conj_merge() {
    let input = pieces(&[
        ("in", Tag::Adp),
        ("spite", Tag::Noun),
        ("of", Tag::Adp),
        ("and", Tag::Cconj),
    ]);
    // Without the trie this would be Prep + Noun + Prep + Conj.
    assert_eq!(kinds_of(&input), vec![ChunkKind::Prep, ChunkKind::Conj]);
    let input = pieces(&[
        ("men", Tag::Noun),
        ("as", Tag::Cconj),
        ("well", Tag::Adv),
        ("as", Tag::Cconj),
        ("women", Tag::Noun),
    ]);
    assert_eq!(
        kinds_of(&input),
        vec![ChunkKind::Noun, ChunkKind::Conj, ChunkKind::Noun]
    );
}

#[test]
fn adverbials_and_quantifier_merge() {
    // `at all`, `no longer`, `in fact`, `as usual` → one Adverb;
    // `a lot of` → one Noun.
    for (words, want) in [
        (
            vec![("not", Tag::Part), ("at", Tag::Adp), ("all", Tag::Det)],
            vec![ChunkKind::Particle, ChunkKind::Adverb],
        ),
        (
            vec![("no", Tag::Det), ("longer", Tag::Adj)],
            vec![ChunkKind::Adverb],
        ),
        (
            vec![("a", Tag::Det), ("lot", Tag::Noun), ("of", Tag::Adp)],
            vec![ChunkKind::Noun],
        ),
    ] {
        let input = pieces(&words);
        assert_eq!(kinds_of(&input), want, "{words:?}");
    }
}

#[test]
fn longest_match_and_case() {
    // `In spite of` (capitalized) still merges as one Prep.
    let input = pieces(&[
        ("In", Tag::Adp),
        ("spite", Tag::Noun),
        ("of", Tag::Adp),
        ("danger", Tag::Noun),
    ]);
    assert_eq!(kinds_of(&input), vec![ChunkKind::Prep, ChunkKind::Noun]);
}

#[test]
fn no_partial_or_comma_span() {
    // `in front` without `of` chunks normally (one Prep here:
    // the ADP arm absorbs the nominal run — no MWE involved).
    let input = pieces(&[("in", Tag::Adp), ("front", Tag::Noun)]);
    assert_eq!(kinds_of(&input), vec![ChunkKind::Prep]);
    // A comma piece breaks the run: no merge.
    let input = pieces(&[
        ("in", Tag::Adp),
        (",", Tag::Punct),
        ("spite", Tag::Noun),
        ("of", Tag::Adp),
    ]);
    assert_eq!(
        kinds_of(&input),
        vec![
            ChunkKind::Prep,
            ChunkKind::Punct,
            ChunkKind::Noun,
            ChunkKind::Prep
        ]
    );
}

#[test]
fn particle_ambiguous_merge_only_all_adp() {
    // `out of` / `up to` / `because of` merge to one Prep only when
    // every piece tags ADP (EWT: 83/84, 24/34, 39/42). Particle
    // readings (ADV first) and clausal seconds (SCONJ) chunk through
    // the cascade instead — the v2 tag gate.
    for words in [
        vec![("out", Tag::Adp), ("of", Tag::Adp)],
        vec![("up", Tag::Adp), ("to", Tag::Adp)],
        vec![("because", Tag::Adp), ("of", Tag::Adp)],
    ] {
        let input = pieces(&words);
        let chunks = chunk_tagged(&input);
        assert_eq!(kinds_of(&input), vec![ChunkKind::Prep], "{words:?}");
        assert_eq!(chunks[0].span(), 0..words.len());
    }
    // Particle `up` (ADV) stays split: Adverb + Prep.
    let input = pieces(&[("up", Tag::Adv), ("to", Tag::Adp), ("midnight", Tag::Propn)]);
    assert_eq!(kinds_of(&input), vec![ChunkKind::Adverb, ChunkKind::Prep]);
    // Clausal `of` (SCONJ) stays split: Subord + Prep + Noun.
    let input = pieces(&[
        ("because", Tag::Adp),
        ("of", Tag::Sconj),
        ("rain", Tag::Noun),
    ]);
    assert_eq!(
        kinds_of(&input),
        vec![ChunkKind::Prep, ChunkKind::Subord, ChunkKind::Noun]
    );
}

#[test]
fn runs_stop_at_phrase_boundaries() {
    // Maximal runs never swallow an MWE start (sweep austen-p450s1:
    // "rapidly as well as" buried the Conj phrase in an Adverb run).
    // Without the stop set this chunks Adverb + Adverb + Prep.
    let input = pieces(&[
        ("rapidly", Tag::Adv),
        ("as", Tag::Adv),
        ("well", Tag::Adv),
        ("as", Tag::Adp),
    ]);
    let chunks = chunk_tagged(&input);
    assert_eq!(kinds_of(&input), vec![ChunkKind::Adverb, ChunkKind::Conj]);
    assert_eq!(chunks[0].span(), 0..1);
    assert_eq!(chunks[1].span(), 1..4);
    // Prep run stops before "a lot of" all the same (ADP-led run
    // would absorb a/lot): Noun phrase wins, then lone "people"
    // chunks Noun ("of" rides inside the MWE span, so no fresh Prep).
    let input = pieces(&[
        ("for", Tag::Adp),
        ("a", Tag::Det),
        ("lot", Tag::Noun),
        ("of", Tag::Adp),
        ("people", Tag::Noun),
    ]);
    assert_eq!(
        kinds_of(&input),
        vec![ChunkKind::Prep, ChunkKind::Noun, ChunkKind::Noun]
    );
}
