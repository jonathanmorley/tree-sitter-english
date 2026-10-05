//! Genre eval for the chunker: hand-chunked gold over the 20
//! second-genre sentences from `english-pos/tests/genre.rs` (10
//! news-report, 10 academic-expository; tokens and gold tags repeated
//! here so this crate stays self-contained).
//!
//! Two tests over one canonical table:
//! - `genre_rule_chunks`: gold tags -> gold chunks, exact. Bar 1.0 by
//!   construction (regression pin: implementation == spec in
//!   `README.md`); linguistic merit rests on CoNLL-2000 fidelity.
//! - `genre_end_to_end`: model tags (one flat call, same as the
//!   tagger's genre eval) -> chunks vs gold chunks. This is the
//!   accuracy number: tagger misses cascade into chunk misses.
//!   Sentence-exact-match rate plus token-level chunk-kind accuracy
//!   are printed; the bar sits below the measured value per eval
//!   discipline (tagger genre precedent: bar 0.86 at 0.884).

use english_chunk::{ChunkKind, ChunkKind as K, chunk_tagged};
use english_pos::{Model, Tag};

/// Canonical table: (tokens, gold tags, gold chunks) per sentence.
#[rustfmt::skip]
const SENTENCES: &[(&[&str], &[Tag], &[ChunkKind])] = &[
    // Stocks fell sharply on Tuesday as investors worried about inflation.
    (
        &["Stocks", "fell", "sharply", "on", "Tuesday", "as", "investors", "worried", "about", "inflation", "."],
        &[Tag::Noun, Tag::Verb, Tag::Adv, Tag::Adp, Tag::Propn, Tag::Sconj, Tag::Noun, Tag::Adj, Tag::Adp, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Adverb, K::Prep, K::Subord, K::Noun, K::Adj, K::Prep, K::Punct],
    ),
    // The mayor said the bridge will reopen in June.
    (
        &["The", "mayor", "said", "the", "bridge", "will", "reopen", "in", "June", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Det, Tag::Noun, Tag::Aux, Tag::Verb, Tag::Adp, Tag::Propn, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Verb, K::Prep, K::Punct],
    ),
    // Police arrested three men after the robbery.
    (
        &["Police", "arrested", "three", "men", "after", "the", "robbery", "."],
        &[Tag::Noun, Tag::Verb, Tag::Num, Tag::Noun, Tag::Adp, Tag::Det, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Prep, K::Punct],
    ),
    // The committee voted 5 to 3 to approve the budget.
    // (Bare counts are lone-NUM nouns; `to` is a particle.)
    (
        &["The", "committee", "voted", "5", "to", "3", "to", "approve", "the", "budget", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Num, Tag::Part, Tag::Num, Tag::Part, Tag::Verb, Tag::Det, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Particle, K::Noun, K::Particle, K::Verb, K::Noun, K::Punct],
    ),
    // Firefighters contained the blaze by midnight.
    (
        &["Firefighters", "contained", "the", "blaze", "by", "midnight", "."],
        &[Tag::Noun, Tag::Verb, Tag::Det, Tag::Noun, Tag::Adp, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Prep, K::Punct],
    ),
    // The court ruled that the law was unconstitutional.
    // (`was` + ADJ splits after the AUX, EWT-side participles.)
    (
        &["The", "court", "ruled", "that", "the", "law", "was", "unconstitutional", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Sconj, Tag::Det, Tag::Noun, Tag::Aux, Tag::Adj, Tag::Punct],
        &[K::Noun, K::Verb, K::Subord, K::Noun, K::Verb, K::Adj, K::Punct],
    ),
    // Exports rose 3.2 percent in the third quarter.
    (
        &["Exports", "rose", "3.2", "percent", "in", "the", "third", "quarter", "."],
        &[Tag::Noun, Tag::Verb, Tag::Num, Tag::Noun, Tag::Adp, Tag::Det, Tag::Adj, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Prep, K::Punct],
    ),
    // The president will visit France next week.
    // (`France` and `next week` are adjacent Noun chunks: an ADJ
    // breaks the nominal continuation.)
    (
        &["The", "president", "will", "visit", "France", "next", "week", "."],
        &[Tag::Det, Tag::Propn, Tag::Aux, Tag::Verb, Tag::Propn, Tag::Adj, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Noun, K::Punct],
    ),
    // Doctors urge patients to get vaccinated.
    (
        &["Doctors", "urge", "patients", "to", "get", "vaccinated", "."],
        &[Tag::Noun, Tag::Verb, Tag::Noun, Tag::Part, Tag::Verb, Tag::Verb, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Particle, K::Verb, K::Punct],
    ),
    // The team won its tenth championship in a row.
    // (`its` stands alone: PRON continues only onto NOUN/NUM.)
    (
        &["The", "team", "won", "its", "tenth", "championship", "in", "a", "row", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Pron, Tag::Adj, Tag::Noun, Tag::Adp, Tag::Det, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Noun, K::Prep, K::Punct],
    ),
    // The results suggest that sleep improves memory.
    (
        &["The", "results", "suggest", "that", "sleep", "improves", "memory", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Sconj, Tag::Noun, Tag::Verb, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Subord, K::Noun, K::Verb, K::Noun, K::Punct],
    ),
    // This study examines the effects of stress on students.
    (
        &["This", "study", "examines", "the", "effects", "of", "stress", "on", "students", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Det, Tag::Noun, Tag::Adp, Tag::Noun, Tag::Adp, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Prep, K::Prep, K::Punct],
    ),
    // The authors argue that policy failed because funding ended.
    (
        &["The", "authors", "argue", "that", "policy", "failed", "because", "funding", "ended", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Sconj, Tag::Noun, Tag::Verb, Tag::Sconj, Tag::Noun, Tag::Verb, Tag::Punct],
        &[K::Noun, K::Verb, K::Subord, K::Noun, K::Verb, K::Subord, K::Noun, K::Verb, K::Punct],
    ),
    // There are several explanations for this pattern.
    (
        &["There", "are", "several", "explanations", "for", "this", "pattern", "."],
        &[Tag::Pron, Tag::Verb, Tag::Adj, Tag::Noun, Tag::Adp, Tag::Det, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Prep, K::Punct],
    ),
    // The data were collected in 1998.
    (
        &["The", "data", "were", "collected", "in", "1998", "."],
        &[Tag::Det, Tag::Noun, Tag::Aux, Tag::Verb, Tag::Adp, Tag::Num, Tag::Punct],
        &[K::Noun, K::Verb, K::Prep, K::Punct],
    ),
    // Critics are concerned about bias in the sample.
    (
        &["Critics", "are", "concerned", "about", "bias", "in", "the", "sample", "."],
        &[Tag::Noun, Tag::Aux, Tag::Adj, Tag::Adp, Tag::Noun, Tag::Adp, Tag::Det, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Adj, K::Prep, K::Prep, K::Punct],
    ),
    // The theory predicts that markets adjust slowly.
    (
        &["The", "theory", "predicts", "that", "markets", "adjust", "slowly", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Sconj, Tag::Noun, Tag::Verb, Tag::Adv, Tag::Punct],
        &[K::Noun, K::Verb, K::Subord, K::Noun, K::Verb, K::Adverb, K::Punct],
    ),
    // We compared treatment and control groups.
    (
        &["We", "compared", "treatment", "and", "control", "groups", "."],
        &[Tag::Pron, Tag::Verb, Tag::Noun, Tag::Cconj, Tag::Noun, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Noun, K::Conj, K::Noun, K::Punct],
    ),
    // The committee demands that it be published.
    (
        &["The", "committee", "demands", "that", "it", "be", "published", "."],
        &[Tag::Det, Tag::Noun, Tag::Verb, Tag::Sconj, Tag::Pron, Tag::Aux, Tag::Verb, Tag::Punct],
        &[K::Noun, K::Verb, K::Subord, K::Noun, K::Verb, K::Punct],
    ),
    // Such findings are consistent with earlier work.
    (
        &["Such", "findings", "are", "consistent", "with", "earlier", "work", "."],
        &[Tag::Adj, Tag::Noun, Tag::Aux, Tag::Adj, Tag::Adp, Tag::Adj, Tag::Noun, Tag::Punct],
        &[K::Noun, K::Verb, K::Adj, K::Prep, K::Punct],
    ),
];

fn pieces(words: &[&str], tags: &[Tag]) -> Vec<(String, Tag)> {
    words
        .iter()
        .map(|w| w.to_string())
        .zip(tags.iter().copied())
        .collect()
}

/// Per-token chunk kinds from chunk spans.
fn token_kinds(chunks: &[english_chunk::Chunk], len: usize) -> Vec<ChunkKind> {
    let mut out = Vec::with_capacity(len);
    for c in chunks {
        for _ in c.span() {
            out.push(c.kind());
        }
    }
    debug_assert_eq!(out.len(), len, "chunks must tile the input");
    out
}

#[test]
fn genre_rule_chunks() {
    assert_eq!(SENTENCES.len(), 20);
    for (si, (words, tags, want)) in SENTENCES.iter().enumerate() {
        assert_eq!(words.len(), tags.len(), "sentence {si}: words/tags length");
        let input = pieces(words, tags);
        let got: Vec<ChunkKind> = chunk_tagged(&input).iter().map(|c| c.kind()).collect();
        assert_eq!(got, *want, "sentence {si}: {words:?}");
    }
}

#[test]
fn genre_end_to_end() {
    let model = Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap();
    // One flat tagging call, same as the tagger's genre eval.
    let flat: Vec<&str> = SENTENCES
        .iter()
        .flat_map(|(w, _, _)| w.iter().copied())
        .collect();
    let tags = model.tag(&flat);
    assert_eq!(tags.len(), flat.len());

    let mut sent_match = 0;
    let mut tag_exact = 0;
    let (mut tok_match, mut tok_total) = (0usize, 0usize);
    let mut miss_sentences = Vec::new();
    let mut miss_tokens: Vec<(usize, &str, ChunkKind, ChunkKind)> = Vec::new();
    let mut off = 0;
    for (si, (words, gold_tags, want)) in SENTENCES.iter().enumerate() {
        let n = words.len();
        let tagged = pieces(words, &tags[off..off + n]);
        let gold = pieces(words, gold_tags);
        let tags_ok = tags[off..off + n] == gold_tags[..];
        if tags_ok {
            tag_exact += 1;
        }
        let got_chunks = chunk_tagged(&tagged);
        let gold_chunks = chunk_tagged(&gold);
        let got: Vec<ChunkKind> = got_chunks.iter().map(|c| c.kind()).collect();
        assert_eq!(got_chunks.iter().map(|c| c.span().end).max(), Some(n));
        if got == *want {
            sent_match += 1;
        } else {
            miss_sentences.push(si);
        }
        // The cascade invariant: a tag-perfect sentence must chunk
        // perfectly (follows from the rule test; pins that the chunker
        // adds zero sentence errors of its own).
        if tags_ok {
            assert_eq!(got, *want, "sentence {si}: tags exact but chunks differ");
        }
        for (ti, (g, t)) in token_kinds(&gold_chunks, n)
            .iter()
            .zip(token_kinds(&got_chunks, n).iter())
            .enumerate()
        {
            tok_total += 1;
            if g == t {
                tok_match += 1;
            } else {
                miss_tokens.push((si, words[ti], *g, *t));
            }
        }
        off += n;
    }
    let sent_rate = sent_match as f64 / SENTENCES.len() as f64;
    let tok_rate = tok_match as f64 / tok_total as f64;
    eprintln!(
        "genre chunk end-to-end: {sent_match}/{nsent} sentences exact ({sent_rate:.3}), \
         tag-exact sentences {tag_exact}, \
         token chunk-kind {tok_match}/{tok_total} ({tok_rate:.3}), \
         miss sentences {miss_sentences:?}, miss tokens {miss_tokens:?}",
        nsent = SENTENCES.len(),
    );
    // Measured 2026-10-05: 9/20 sentences (0.450), token 151/173
    // (0.873); all 22 token misses trace to tagger misses (that/VERB
    // -s cascades above) — the tag-exact-implies-chunk-exact assert
    // pins zero chunker-introduced sentence errors. Bar 0.43.
    assert!(
        sent_rate >= 0.43,
        "genre chunk end-to-end sentence rate {sent_rate:.3} below bar; move deliberately",
    );
}
