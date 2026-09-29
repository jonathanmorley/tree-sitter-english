//! Chunker unit tests over hand-tagged piece sequences (no model:
//! chunking takes tags as input, so gold tags isolate chunker behavior
//! from tagger errors).

use english_chunk::{ChunkKind, chunk_tagged};
use english_pos::Tag;

fn pieces(pairs: &[(&str, Tag)]) -> Vec<(String, Tag)> {
    pairs
        .iter()
        .map(|(w, t)| (w.to_string(), *t))
        .collect()
}

fn kinds_of(input: &[(String, Tag)]) -> Vec<ChunkKind> {
    chunk_tagged(input).iter().map(|c| c.kind()).collect()
}

#[test]
fn canonical_sentence_chunks() {
    // Time/NOUN flies/VERB like/ADP an/DET arrow/NOUN ./PUNCT
    let input = pieces(&[
        ("Time", Tag::Noun),
        ("flies", Tag::Verb),
        ("like", Tag::Adp),
        ("an", Tag::Det),
        ("arrow", Tag::Noun),
        (".", Tag::Punct),
    ]);
    let chunks = chunk_tagged(&input);
    assert_eq!(
        kinds_of(&input),
        vec![
            ChunkKind::Noun,
            ChunkKind::Verb,
            ChunkKind::Prep,
            ChunkKind::Punct
        ]
    );
    // Prep absorbs the nominal run.
    assert_eq!(chunks[2].span(), 2..5);
    assert_eq!(chunks[2].tokens(&input).len(), 3);
}

#[test]
fn aux_verb_cluster_and_lone_copula() {
    let input = pieces(&[
        ("would", Tag::Aux),
        ("sail", Tag::Verb),
        ("is", Tag::Aux),
        ("happy", Tag::Adj),
    ]);
    assert_eq!(
        kinds_of(&input),
        vec![ChunkKind::Verb, ChunkKind::Verb, ChunkKind::Adj]
    );
}

#[test]
fn attributive_absorbs_predicative_splits() {
    // the green fields -> one Noun; very green (alone) -> Adverb + Adj.
    let input = pieces(&[
        ("the", Tag::Det),
        ("green", Tag::Adj),
        ("fields", Tag::Noun),
    ]);
    assert_eq!(kinds_of(&input), vec![ChunkKind::Noun]);
    let input = pieces(&[("very", Tag::Adv), ("green", Tag::Adj)]);
    assert_eq!(kinds_of(&input), vec![ChunkKind::Adverb, ChunkKind::Adj]);
}

#[test]
fn subordinate_head_and_particles() {
    // that/DET... no: that/SCONJ he/PRON left/VERB to/PART go/VERB !/PUNCT
    let input = pieces(&[
        ("that", Tag::Sconj),
        ("he", Tag::Pron),
        ("left", Tag::Verb),
        ("to", Tag::Part),
        ("go", Tag::Verb),
    ]);
    assert_eq!(
        kinds_of(&input),
        vec![
            ChunkKind::Subord,
            ChunkKind::Noun,
            ChunkKind::Verb,
            ChunkKind::Particle,
            ChunkKind::Verb
        ]
    );
}

#[test]
fn conjunctions_interjections_other() {
    let input = pieces(&[
        ("and", Tag::Cconj),
        ("oh", Tag::Intj),
        ("$20", Tag::Sym),
    ]);
    assert_eq!(
        kinds_of(&input),
        vec![ChunkKind::Conj, ChunkKind::Interj, ChunkKind::Other]
    );
}

#[test]
fn lone_det_nouns() {
    // Pronominal `all` stands alone as Noun.
    let input = pieces(&[("all", Tag::Det)]);
    assert_eq!(kinds_of(&input), vec![ChunkKind::Noun]);
}

#[test]
fn full_coverage_spans() {
    // Spans tile the input with no gaps or overlaps.
    let input = pieces(&[
        ("Time", Tag::Noun),
        ("flies", Tag::Verb),
        ("like", Tag::Adp),
        ("an", Tag::Det),
        ("arrow", Tag::Noun),
        (".", Tag::Punct),
    ]);
    let chunks = chunk_tagged(&input);
    let mut next = 0;
    for c in &chunks {
        assert_eq!(c.span().start, next);
        next = c.span().end;
    }
    assert_eq!(next, input.len());
}
