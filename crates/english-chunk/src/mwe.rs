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
//! merging would hide the nominal `order`; EWT is unanimous
//! ADP+NOUN+PART 15:0), `of course` (EWT unanimous ADP+NOUN 20:0 —
//! it chunks as a Prep through the cascade already). Kinds follow
//! the head reading: subordinators → Subord, coordinative
//! `as well as` → Conj, prepositional phrases → Prep, `a lot of`
//! → Noun (pronominal quantifier), temporal/discourse adverbials
//! (`at all`, `no longer`, `in fact`, `as usual`) → Adverb.
//! `out of` / `up to` / `because of` → Prep, but ONLY when every
//! piece tags ADP (EWT: 83/84, 24/34, 39/42 — the remainders are
//! particle ADV-first or clausal SCONJ-second readings that must
//! stay split). This is the tag-aware v2: the trie consults the
//! tag stream, so `stayed up to midnight` (ADV+ADP) never merges.
//! Known imperfection: comparative `as well as I do` chunks Conj
//! (the 95% coordinative reading wins the kind).

use english_pos::Tag;

use crate::ChunkKind;

struct TrieNode {
    children: &'static [(&'static str, TrieNode)],
    kind: Option<ChunkKind>,
    /// Required tags for the whole phrase (empty = tag-blind).
    /// Checked against the input tag stream on match.
    want: &'static [Tag],
}

const NONE: &[Tag] = &[];
const ALL_ADP: &[Tag] = &[Tag::Adp, Tag::Adp];

const AS_IF: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Subord),
};
const AS_WELL_AS: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Conj),
};
const WELL: TrieNode = TrieNode {
    children: &[("as", AS_WELL_AS)],
    kind: None,
    want: NONE,
};
const AS: TrieNode = TrieNode {
    children: &[("well", WELL), ("if", AS_IF), ("usual", AS_USUAL)],
    kind: None,
    want: NONE,
};
const AS_USUAL: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Adverb),
};
const SO_THAT: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Subord),
};
const SO: TrieNode = TrieNode {
    children: &[("that", SO_THAT)],
    kind: None,
    want: NONE,
};
const SUCH_AS: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Subord),
};
const SUCH: TrieNode = TrieNode {
    children: &[("as", SUCH_AS)],
    kind: None,
    want: NONE,
};
const IN_SPITE_OF: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Prep),
};
const SPITE: TrieNode = TrieNode {
    children: &[("of", IN_SPITE_OF)],
    kind: None,
    want: NONE,
};
const IN_FRONT_OF: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Prep),
};
const FRONT: TrieNode = TrieNode {
    children: &[("of", IN_FRONT_OF)],
    kind: None,
    want: NONE,
};
const IN_FACT: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Adverb),
};
const IN: TrieNode = TrieNode {
    children: &[("spite", SPITE), ("front", FRONT), ("fact", IN_FACT)],
    kind: None,
    want: NONE,
};
const A_LOT_OF: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Noun),
};
const LOT: TrieNode = TrieNode {
    children: &[("of", A_LOT_OF)],
    kind: None,
    want: NONE,
};
const A: TrieNode = TrieNode {
    children: &[("lot", LOT)],
    kind: None,
    want: NONE,
};
const AT_ALL: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Adverb),
};
const AT: TrieNode = TrieNode {
    children: &[("all", AT_ALL)],
    kind: None,
    want: NONE,
};
const NO_LONGER: TrieNode = TrieNode {
    children: &[],
    want: NONE,
    kind: Some(ChunkKind::Adverb),
};
const NO: TrieNode = TrieNode {
    children: &[("longer", NO_LONGER)],
    kind: None,
    want: NONE,
};
const OUT_OF: TrieNode = TrieNode {
    children: &[],
    want: ALL_ADP,
    kind: Some(ChunkKind::Prep),
};
const OUT: TrieNode = TrieNode {
    children: &[("of", OUT_OF)],
    kind: None,
    want: NONE,
};
const UP_TO: TrieNode = TrieNode {
    children: &[],
    want: ALL_ADP,
    kind: Some(ChunkKind::Prep),
};
const UP: TrieNode = TrieNode {
    children: &[("to", UP_TO)],
    kind: None,
    want: NONE,
};
const BECAUSE_OF: TrieNode = TrieNode {
    children: &[],
    want: ALL_ADP,
    kind: Some(ChunkKind::Prep),
};
const BECAUSE: TrieNode = TrieNode {
    children: &[("of", BECAUSE_OF)],
    kind: None,
    want: NONE,
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
        ("out", OUT),
        ("up", UP),
        ("because", BECAUSE),
    ],
    kind: None,
    want: NONE,
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
                    // Tag-gated phrases (`out of` / `up to` /
                    // `because of`): the whole span must carry the
                    // listed tags, else the particle/clausal reading
                    // holds and the cascade chunks as before.
                    let gated = !node.want.is_empty()
                        && !input[start..i]
                            .iter()
                            .zip(node.want.iter())
                            .all(|((_, t), w)| t == w);
                    if !gated {
                        best = Some((i - start, kind));
                    }
                }
            }
            None => break,
        }
    }
    best
}
