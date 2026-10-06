//! Greedy phrase chunking over tagged pieces (CoNLL-2000 pattern).
//!
//! See `README.md` for the tagset spec. Single left-to-right pass:
//! each token is consumed exactly once, so cost is linear in the
//! sentence length (no regex-over-string, no unbounded lookahead —
//! every scan stops at the first non-matching token).

use std::ops::Range;

use english_pos::Tag;

mod mwe;

/// Phrase kinds (UD-adapted CoNLL-2000; see `README.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    Noun,
    Verb,
    Prep,
    Adverb,
    Adj,
    Subord,
    Conj,
    Particle,
    Interj,
    Punct,
    Other,
}

/// One flat phrase: its kind plus token span into the input slice.
/// Index-based (no borrows, no copies) so chunking allocates only the
/// `Vec<Chunk>` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    kind: ChunkKind,
    span: Range<usize>,
}

impl Chunk {
    /// The phrase kind.
    pub fn kind(&self) -> ChunkKind {
        self.kind
    }

    /// Token span into the input slice.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// The `(surface piece, tag)` pairs covered, borrowed from input.
    pub fn tokens<'x>(&self, input: &'x [(String, Tag)]) -> &'x [(String, Tag)] {
        &input[self.span.clone()]
    }
}

fn tag_at(input: &[(String, Tag)], i: usize) -> Option<Tag> {
    input.get(i).map(|(_, t)| *t)
}

fn is_nominal(t: Tag) -> bool {
    matches!(t, Tag::Noun | Tag::Propn | Tag::Pron | Tag::Num)
}

fn consume_while(input: &[(String, Tag)], mut i: usize, f: impl Fn(Tag) -> bool) -> usize {
    while tag_at(input, i).is_some_and(&f) {
        i += 1;
    }
    i
}

/// End index (exclusive) of a noun chunk starting at `i`, or `None`
/// when no nominal follows the prefix. Prefix: DET* ADJ* NUM* (no
/// ADV — degree adverbs chunk separately, CoNLL-faithful). A leading
/// PRON continues only onto NOUN/NUM (`my substitute`, `we sailors`
/// merge) but not onto PROPN/PRON (`me Ishmael`, `you Starbuck`
/// split — vocative/address; a deliberate CoNLL deviation for
/// dialogue-heavy prose, where pronoun+name adjacency is address,
/// not compounding).
fn noun_end(input: &[(String, Tag)], mut i: usize) -> Option<usize> {
    while matches!(
        tag_at(input, i),
        Some(Tag::Det) | Some(Tag::Adj) | Some(Tag::Num)
    ) {
        i += 1;
    }
    let first = tag_at(input, i)?;
    if !is_nominal(first) {
        return None;
    }
    i += 1;
    if first == Tag::Pron {
        while matches!(tag_at(input, i), Some(Tag::Noun) | Some(Tag::Num)) {
            i += 1;
        }
    } else {
        while matches!(
            tag_at(input, i),
            Some(Tag::Noun) | Some(Tag::Propn) | Some(Tag::Num)
        ) {
            i += 1;
        }
    }
    Some(i)
}

/// End index of a verb chunk: AUX* VERB+ (one total minimum; a lone
/// copula counts, and `AUX` + `ADJ` splits after the AUX per EWT-side
/// participles).
fn verb_end(input: &[(String, Tag)], mut i: usize) -> usize {
    while tag_at(input, i) == Some(Tag::Aux) {
        i += 1;
    }
    while tag_at(input, i) == Some(Tag::Verb) {
        i += 1;
    }
    i
}

/// Chunk tagged pieces greedily. Priority at each position: fixed
/// phrases (`mwe` longest match), Punct,
/// Subord, Conj, Particle, Interj, Noun, Verb, Prep, Adverb, Adj,
/// Other. Noun-before-Adverb/Adj gives attributive-vs-predicative
/// disambiguation for free (`green fields` → Noun via nominal
/// lookahead; lone `green` falls to Adj).
pub fn chunk_tagged(input: &[(String, Tag)]) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let start = i;
        // Fixed phrases first (longest match): unlisted text chunks
        // exactly as without the trie.
        if let Some((len, kind)) = mwe::match_len(input, i) {
            i += len;
            out.push(Chunk {
                kind,
                span: start..i,
            });
            continue;
        }
        let kind = match tag_at(input, i) {
            Some(Tag::Punct) => {
                i = consume_while(input, i, |t| t == Tag::Punct);
                ChunkKind::Punct
            }
            Some(Tag::Sconj) => {
                i += 1;
                ChunkKind::Subord
            }
            Some(Tag::Cconj) => {
                i = consume_while(input, i, |t| t == Tag::Cconj);
                ChunkKind::Conj
            }
            Some(Tag::Part) => {
                i = consume_while(input, i, |t| t == Tag::Part);
                ChunkKind::Particle
            }
            Some(Tag::Intj) => {
                i = consume_while(input, i, |t| t == Tag::Intj);
                ChunkKind::Interj
            }
            Some(Tag::Det) | Some(Tag::Adj) | Some(Tag::Num) | Some(Tag::Noun)
            | Some(Tag::Propn) | Some(Tag::Pron) => {
                match noun_end(input, i) {
                    Some(end) => {
                        i = end;
                        ChunkKind::Noun
                    }
                    // Lone DET (`all`, stranding) and lone NUM (`5`, vote
                    // counts) still nouns; a bare NUM must never fall
                    // through to the Adj arm, whose `consume_while` would
                    // emit a zero-width chunk. Bare ADJ with no nominal
                    // ahead is predicative (a NUM always satisfies
                    // noun_end, so only ADJ lands here).
                    None if matches!(tag_at(input, i), Some(Tag::Det) | Some(Tag::Num)) => {
                        i += 1;
                        ChunkKind::Noun
                    }
                    None => {
                        i = consume_while(input, i, |t| t == Tag::Adj);
                        ChunkKind::Adj
                    }
                }
            }
            Some(Tag::Aux) | Some(Tag::Verb) => {
                i = verb_end(input, i);
                ChunkKind::Verb
            }
            Some(Tag::Adp) => {
                i += 1;
                while matches!(
                    tag_at(input, i),
                    Some(Tag::Det)
                        | Some(Tag::Adj)
                        | Some(Tag::Num)
                        | Some(Tag::Noun)
                        | Some(Tag::Propn)
                        | Some(Tag::Pron)
                ) {
                    i += 1;
                }
                ChunkKind::Prep
            }
            Some(Tag::Adv) => {
                i = consume_while(input, i, |t| t == Tag::Adv);
                ChunkKind::Adverb
            }
            _ => {
                // X, SYM, and anything unlisted. (A standalone Adj arm
                // here would be unreachable: the Noun arm above matches
                // Adj first — attributive absorbs, predicative falls
                // through to Adj inside that arm.)
                i += 1;
                ChunkKind::Other
            }
        };
        out.push(Chunk {
            kind,
            span: start..i,
        });
    }
    out
}
