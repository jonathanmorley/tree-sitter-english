//! Multiword-expression chunking (CoNLL-style fixed phrases).
//!
//! A longest-match trie over lowercased pieces merges `in spite
//! of`-class phrases into single chunks. Same never-grammar reason
//! as Tier 3: phrase grouping belongs in this post-pass, not in
//! `grammar.js`. The trie runs ahead of the priority cascade in
//! [`chunk_tagged`](crate::chunk_tagged): at each position the
//! longest listed phrase wins; anything else chunks as before, so
//! unlisted text is byte-identical to the no-MWE output.
//!
//! Matching is over tagged *pieces* (the chunker's input currency,
//! already contraction-split). A comma piece between words breaks
//! the run naturally — matching is strictly consecutive, so `in,
//! spite of` never merges. Case-insensitive ASCII compare (the list
//! is ASCII; nothing else can match).
//!
//! List discipline (lexicon lesson): every entry is attested in
//! book-domain text (Moby counts: `as if` 133, `so that` 127,
//! `at all` 87, `as well as` 19, `in fact` 13, `no longer` 10,
//! `such as` 8, `a lot of` 3, `in front of` 2, `in spite of` 1,
//! `as usual` 2 — all four sweep books carry most of them).
//! Deliberately excluded: `in order to` (infinitive semantics —
//! merging would hide the nominal `order`), `because of` / `out
//! of` / `up to` (particle ambiguity: `looked out of` reads the
//! first word as a particle, and tag-blind merging would conflate
//! it), `of course` (discourse semantics unclear). Kinds follow
//! the head reading: subordinators → Subord, coordinative
//! `as well as` → Conj, prepositional phrases → Prep, `a lot of`
//! → Noun (pronominal quantifier), temporal/discourse adverbials
//! (`at all`, `no longer`, `in fact`, `as usual`) → Adverb.
//! Known imperfection: comparative `as well as I do` chunks Conj
//! (the 95% coordinative reading wins the kind).

use crate::ChunkKind;

struct TrieNode {
    children: &'static [(&'static str, TrieNode)],
    kind: Option<ChunkKind>,
}

const AS_IF: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Subord),
};
const AS_WELL_AS: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Conj),
};
const WELL: TrieNode = TrieNode {
    children: &[("as", AS_WELL_AS)],
    kind: None,
};
const AS: TrieNode = TrieNode {
    children: &[("well", WELL), ("if", AS_IF), ("usual", AS_USUAL)],
    kind: None,
};
const AS_USUAL: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Adverb),
};
const SO_THAT: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Subord),
};
const SO: TrieNode = TrieNode {
    children: &[("that", SO_THAT)],
    kind: None,
};
const SUCH_AS: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Subord),
};
const SUCH: TrieNode = TrieNode {
    children: &[("as", SUCH_AS)],
    kind: None,
};
const IN_SPITE_OF: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Prep),
};
const SPITE: TrieNode = TrieNode {
    children: &[("of", IN_SPITE_OF)],
    kind: None,
};
const IN_FRONT_OF: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Prep),
};
const FRONT: TrieNode = TrieNode {
    children: &[("of", IN_FRONT_OF)],
    kind: None,
};
const IN_FACT: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Adverb),
};
const IN: TrieNode = TrieNode {
    children: &[("spite", SPITE), ("front", FRONT), ("fact", IN_FACT)],
    kind: None,
};
const A_LOT_OF: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Noun),
};
const LOT: TrieNode = TrieNode {
    children: &[("of", A_LOT_OF)],
    kind: None,
};
const A: TrieNode = TrieNode {
    children: &[("lot", LOT)],
    kind: None,
};
const AT_ALL: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Adverb),
};
const AT: TrieNode = TrieNode {
    children: &[("all", AT_ALL)],
    kind: None,
};
const NO_LONGER: TrieNode = TrieNode {
    children: &[],
    kind: Some(ChunkKind::Adverb),
};
const NO: TrieNode = TrieNode {
    children: &[("longer", NO_LONGER)],
    kind: None,
};

const ROOT: TrieNode = TrieNode {
    children: &[
        ("in", IN),
        ("as", AS),
        ("so", SO),
        ("such", SUCH),
        ("a", A),
        ("at", AT),
        ("no", NO),
    ],
    kind: None,
};

/// Longest listed phrase starting at `start`: `(length, kind)`.
/// Pure prefix walk — no allocation, no hashing (≤ 4 children per
/// node, linear scan).
pub(crate) fn match_len(
    input: &[(String, english_pos::Tag)],
    start: usize,
) -> Option<(usize, ChunkKind)> {
    let mut node = &ROOT;
    let mut best: Option<(usize, ChunkKind)> = None;
    let mut i = start;
    while i < input.len() {
        let word = input[i].0.as_str();
        match node
            .children
            .iter()
            .find(|(key, _)| word.eq_ignore_ascii_case(key))
        {
            Some((_, next)) => {
                node = next;
                i += 1;
                if let Some(kind) = node.kind {
                    best = Some((i - start, kind));
                }
            }
            None => break,
        }
    }
    best
}
