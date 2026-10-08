//! Statistical part-of-speech tagging for English prose.
//!
//! A greedy perceptron tagger over the 17 Universal POS tags.
//! This crate holds the inference side: feature extraction shared with
//! training, the [`Model`] with greedy decoding, and JSON weight
//! (de)serialization. Training lives in `crates/english-pos-train`.
//!
//! Weights are keyed by pre-hashed `u64` feature ids in a map with a
//! trivial hasher (no SipHash re-hashing): lookups run ~3M per book
//! and are the tag pass's hottest operation after feature hashing.
//! Deliberately a post-parse pass, never grammar: the grammar segments
//! flat clauses of unclassified words (Tier 3 showed word classes do
//! not belong there); this crate annotates them with probabilities that
//! degrade gracefully instead of failing the tree.

mod correction;
mod lexicon;
mod tag;
mod wire;

pub use correction::{RULES, Rule, apply_rules};
pub use tag::Tag;
pub use wire::{
    TagCache, append_clause_pieces, append_sentence_pieces, clause_pieces, sentence_pieces,
    split_contraction, tag_clause, tag_document, tag_sentence, token_pieces,
};

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Trivial hasher for pre-hashed `u64` feature ids.
///
/// Feature ids are already 64-bit FNV-1a avalanches, so re-hashing
/// them with SipHash (the `std` default) burns ~3M hashes per book
/// for nothing. `write_u64` stores the key verbatim; `finish`
/// applies a splitmix64 finalizer to spread the low bits that pick
/// SwissTable buckets. Zero dependencies, no accuracy impact —
/// lookup results are identical, only faster.
#[derive(Default)]
struct U64Hasher(u64);

impl Hasher for U64Hasher {
    fn write(&mut self, bytes: &[u8]) {
        // Fold into running state, never replace: `HashMap<String,
        // ..>` hashes keys with two calls (bytes, then a tag byte),
        // so replacing here collapses every key into one bucket
        // (observed: 14k-entry linear scans, 52µs/lookup). Single-call
        // `u64` keys are unaffected (they go through `write_u64`).
        let mut h = self.0 ^ FNV_OFFSET_BASIS;
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
        self.0 = h;
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = v;
    }

    fn finish(&self) -> u64 {
        let mut z = self.0.wrapping_add(0x9e3779b97f4a7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

/// `HashMap` keyed by pre-hashed `u64` ids (weights, feature counts).
type U64Map<V> = HashMap<u64, V, BuildHasherDefault<U64Hasher>>;

/// Pseudo-tags opening every sentence for tag-history features.
pub const START1: &str = "<START>";
pub const START2: &str = "<START2>";

/// All tags in a fixed order (decoding tie-breaks and JSON stability).
pub const TAGS: [&str; 17] = [
    "ADJ", "ADP", "ADV", "AUX", "CCONJ", "DET", "INTJ", "NOUN", "NUM", "PART", "PRON", "PROPN",
    "PUNCT", "SCONJ", "SYM", "VERB", "X",
];

/// Beam re-decode trigger: greedy spans with a margin strictly
/// below this get a width-2 joint re-decode. Calibrated in the
/// trainer (`--beam` sweep: T=2.0 wins — dev +10, test +22 over
/// greedy at MAX 8; T≤1.0 catches only exact ties and scores −6);
/// production uses this value.
pub const BEAM_MARGIN_T: f32 = 2.0;

/// Beam re-decode cap: longer spans (run plus two left-context
/// tokens) keep greedy tags. Bounds worst-case cost; low-margin runs
/// are usually 1–3 tokens.
pub const BEAM_MAX_SPAN: usize = 8;

/// FNV-1a 64-bit: stable across processes and versions (unlike
/// `DefaultHasher`), so committed weight keys stay valid. Collisions
/// merge two features benignly; at 64 bits the rate is negligible.
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Hash one feature template: `discriminant` namespaces the template,
/// `parts` are fed with separators so ("ab","c") != ("a","bc").
/// Public so forensics tooling can re-derive the id of any template
/// and look up its weights (hashes are one-way; re-derivation is the
/// only way back). Discriminants used by [`features`]: `0x10` word,
/// `0x11` prev word, `0x12` next word, `0x13`/`0x14` prev tags,
/// `0x15` tag bigram, `0x21`-`0x23` prefixes 1-3, `0x25`-`0x27`
/// suffixes 1-3.
pub fn hash_feature(discriminant: u8, parts: &[&str]) -> u64 {
    let mut h = FNV_OFFSET_BASIS;
    h ^= discriminant as u64;
    h = h.wrapping_mul(FNV_PRIME);
    for part in parts {
        for b in part.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
        h ^= 0xff;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

// Fixed ids for constant (word-independent) features.
const F_BIAS: u64 = 0x9e3779b97f4a7c15 ^ 0x01;
const F_TITLE: u64 = 0x9e3779b97f4a7c15 ^ 0x02;
const F_UPPER: u64 = 0x9e3779b97f4a7c15 ^ 0x03;
const F_DIGIT: u64 = 0x9e3779b97f4a7c15 ^ 0x04;
const F_HYPHEN: u64 = 0x9e3779b97f4a7c15 ^ 0x05;

/// First `n` chars of `s` as a subslice, or `None` when shorter.
/// No allocation (replaces the old `chars().collect::<Vec<_>>()`).
fn first_n(s: &str, n: usize) -> Option<&str> {
    let mut count = 0;
    for (i, _) in s.char_indices() {
        if count == n {
            return Some(&s[..i]);
        }
        count += 1;
    }
    if count >= n { Some(s) } else { None }
}

/// Last `n` chars of `s` as a subslice, or `None` when shorter.
fn last_n(s: &str, n: usize) -> Option<&str> {
    let total = s.chars().count();
    if total < n {
        return None;
    }
    let skip = total - n;
    s.char_indices().nth(skip).map(|(i, _)| &s[i..])
}

/// Feature templates for position `i` given surface `raw` and lowercased
/// `lower` words plus the two previous tags. Classic greedy-perceptron
/// set (word identity, neighbors, tag bigrams, affixes, shape flags).
///
/// Features are 64-bit FNV-1a hashes, extracted with zero allocation:
/// word/affix text is hashed in place and the scratch buffer is reused,
/// so steady-state tagging allocates nothing per token. Same template
/// set as the original string-keyed model, so accuracy is unchanged.
/// Shared verbatim by training and inference: any change here
/// invalidates committed weights.
///
/// Generic over the word containers so inference can score borrowed
/// `&str` slices without cloning them into `String`s first.
pub fn features<R: AsRef<str>, L: AsRef<str>>(
    raw: &[R],
    lower: &[L],
    i: usize,
    prev1: &str,
    prev2: &str,
    feats: &mut Vec<u64>,
) {
    feats.clear();
    let w = lower[i].as_ref();
    feats.push(F_BIAS);
    feats.push(hash_feature(0x10, &[w]));
    feats.push(hash_feature(
        0x11,
        &[if i > 0 { lower[i - 1].as_ref() } else { START1 }],
    ));
    feats.push(hash_feature(
        0x12,
        &[if i + 1 < lower.len() {
            lower[i + 1].as_ref()
        } else {
            START1
        }],
    ));
    feats.push(hash_feature(0x13, &[prev1]));
    feats.push(hash_feature(0x14, &[prev2]));
    feats.push(hash_feature(0x15, &[prev1, prev2]));
    push_affixes(w, feats);
    push_shape(raw[i].as_ref(), feats);
}

/// Prefix/suffix 1–3 features for `w` (hash discriminants
/// `0x21`–`0x23` / `0x25`–`0x27`). ASCII words (the common case)
/// slice bytes directly; other words fall back to the char-boundary
/// walk. Same ids in the same order either way.
fn push_affixes(w: &str, feats: &mut Vec<u64>) {
    if w.is_ascii() {
        let b = w.as_bytes();
        for n in 1..=3 {
            if b.len() >= n {
                feats.push(hash_feature(0x20 + n as u8, &[&w[..n]]));
                feats.push(hash_feature(0x24 + n as u8, &[&w[b.len() - n..]]));
            }
        }
        return;
    }
    for n in 1..=3 {
        if let Some(p) = first_n(w, n) {
            feats.push(hash_feature(0x20 + n as u8, &[p]));
        }
        if let Some(s) = last_n(w, n) {
            feats.push(hash_feature(0x24 + n as u8, &[s]));
        }
    }
}

/// Shape flags for the raw word: titlecase / all-caps / digit /
/// hyphen. One char pass with the same truth table as the old four
/// scans (empty word sets nothing).
fn push_shape(raw_word: &str, feats: &mut Vec<u64>) {
    let mut first_upper = false;
    let mut rest_upper = false;
    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_digit = false;
    for (k, c) in raw_word.chars().enumerate() {
        let up = c.is_uppercase();
        has_upper |= up;
        has_lower |= c.is_lowercase();
        if k == 0 {
            first_upper = up;
        } else {
            rest_upper |= up;
        }
        has_digit |= c.is_numeric();
    }
    if first_upper && !rest_upper {
        feats.push(F_TITLE);
    }
    if has_upper && !has_lower {
        feats.push(F_UPPER);
    }
    if has_digit {
        feats.push(F_DIGIT);
    }
    if raw_word.contains('-') {
        feats.push(F_HYPHEN);
    }
}

/// Greedy perceptron model: `weights[feature][tag_index]`.
///
/// Tags ride in a dense `[f32; 17]` in [`TAGS`] order, so decoding adds
/// one array per active feature instead of one hash lookup per
/// (feature, tag) pair. The map itself uses a trivial `u64` hasher
/// ([`U64Hasher`]): feature ids arrive pre-hashed (FNV-1a), so the
/// `std` SipHash default would re-hash ~3M lookups per book for no
/// benefit. Serialized shape is unchanged
/// (`{"weights": {feature: {TAG: weight}}}`) with integer feature keys;
/// whole-number weights serialize as integers (see [`Model::to_json`]).
#[derive(Debug, Default, Clone)]
pub struct Model {
    weights: U64Map<[f32; 17]>,
    /// Inference fast path (Honnibal tagdict probe): lowercased words
    /// seen under exactly one tag in training, hashed like every other
    /// map in this crate (one FNV + one probe — the earlier sorted-vec
    /// binary search cost MORE than scoring on misses and slowed the
    /// tag pass 102→148 ms; map probe is ~10× cheaper). `decode_lower` emits the dict tag with
    /// [`FAST_PATH_MARGIN`] instead of scoring. Consulted on the fast
    /// path only — never in training, never in rules. Empty for weight
    /// files written before it existed. Byte-identity vs full decode
    /// is empirical, not structural: any weights/rules change must
    /// re-verify on every eval (see the probe record in the train
    /// README). Finetune clears it (weight updates can move a word off
    /// its dict tag; the fast path must never shadow training).
    tagdict: HashMap<String, u8, BuildHasherDefault<U64Hasher>>,
}

/// Fabricated margin for tagdict-skipped tokens: finite (the
/// `tag_margins` contract), far above every rule threshold (max 5.0)
/// and beam span threshold, so gates and spans behave exactly as
/// with infinite confidence. Part of the API: consumers must treat
/// it as "confident by lookup", never as a measured gap (ordering
/// assertions hold among scored tokens only).
pub const FAST_PATH_MARGIN: f32 = 1e9;

/// Position of a tag code in [`TAGS`].
fn tag_index(code: &str) -> Option<usize> {
    TAGS.iter().position(|t| *t == code)
}

impl Model {
    /// Greedy left-to-right decode of surface `words` (lowercased
    /// internally; shape features read the raw forms). Steady-state
    /// allocation: none per token (scratch buffer reused).
    pub fn tag<S: AsRef<str>>(&self, words: &[S]) -> Vec<Tag> {
        self.decode(words).0
    }

    /// Decode like [`Model::tag`], pairing each tag with its margin:
    /// best score minus runner-up score (≥ 0). Tiny margins mark
    /// genuinely ambiguous tokens — the `verify` example surfaces the
    /// lowest-margin sentences instead of failing them.
    pub fn tag_margins<S: AsRef<str>>(&self, words: &[S]) -> Vec<(Tag, f32)> {
        let (tags, margins) = self.decode(words);
        tags.into_iter().zip(margins).collect()
    }

    /// Beam re-decode with [`BEAM_MARGIN_T`]/[`BEAM_MAX_SPAN`].
    /// See [`Model::tag_beam_with`] for the algorithm.
    pub fn tag_beam<S: AsRef<str>>(&self, words: &[S]) -> Vec<Tag> {
        self.tag_beam_margins(words)
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    }

    /// [`Model::tag_beam`] paired with per-token margins: greedy
    /// margins off-span, local best-minus-runner-up gaps under the
    /// winning history on-span. Feeds the correction gate the same
    /// way [`Model::tag_margins`] does.
    pub fn tag_beam_margins<S: AsRef<str>>(&self, words: &[S]) -> Vec<(Tag, f32)> {
        self.tag_beam_margins_lowered(words).0
    }

    /// [`Model::tag_beam_margins`], also returning the lowercased
    /// forms. The correction layer's shape tests read lowercase, so
    /// production callers reuse this instead of lowercasing the
    /// sentence a second time.
    pub fn tag_beam_margins_lowered<S: AsRef<str>>(
        &self,
        words: &[S],
    ) -> (Vec<(Tag, f32)>, Vec<String>) {
        let (tags, margins, lower, _) = self.decode_beam(words, BEAM_MARGIN_T, BEAM_MAX_SPAN);
        (tags.into_iter().zip(margins).collect(), lower)
    }

    /// [`Model::tag_beam`] with explicit span threshold and cap, plus
    /// span statistics for measurement: `(tags, count_of_spans,
    /// count_of_rescored_tokens)`. The trainer `--beam` sweep tunes
    /// through here; production uses the constants.
    pub fn tag_beam_with<S: AsRef<str>>(
        &self,
        words: &[S],
        margin_t: f32,
        max_span: usize,
    ) -> (Vec<Tag>, usize, usize) {
        let (tags, _, _, stats) = self.decode_beam(words, margin_t, max_span);
        (tags, stats.0, stats.1)
    }

    /// [`Model::tag_beam_margins`] with explicit threshold and cap,
    /// plus span statistics. Powers the trainer `--beam` path
    /// (including `--correct` composition with the rules).
    pub fn tag_beam_margins_with<S: AsRef<str>>(
        &self,
        words: &[S],
        margin_t: f32,
        max_span: usize,
    ) -> (Vec<(Tag, f32)>, (usize, usize)) {
        let (tags, margins, _, stats) = self.decode_beam(words, margin_t, max_span);
        (tags.into_iter().zip(margins).collect(), stats)
    }

    fn decode<S: AsRef<str>>(&self, words: &[S]) -> (Vec<Tag>, Vec<f32>) {
        let (tags, margins, _) = self.decode_lower(words);
        (tags, margins)
    }

    /// Greedy decode, also returning the lowercased forms. The beam
    /// pass reuses them instead of lowercasing the sentence a second
    /// time (the old `decode_beam` rebuilt both `lower` and `raw`).
    ///
    /// Allocation per sentence: one `lower` Vec (unavoidable — the
    /// model reads lowercased text). Tag history rides `&str`
    /// borrows of the `TAGS`/`START` statics, never owned `String`s.
    fn decode_lower<S: AsRef<str>>(&self, words: &[S]) -> (Vec<Tag>, Vec<f32>, Vec<String>) {
        let lower: Vec<String> = words.iter().map(|w| w.as_ref().to_lowercase()).collect();
        let mut prev1 = START1;
        let mut prev2 = START2;
        let mut feats = Vec::with_capacity(20);
        let mut tags = Vec::with_capacity(words.len());
        let mut margins = Vec::with_capacity(words.len());
        for i in 0..words.len() {
            // Tagdict fast path: unambiguous-in-training words skip
            // scoring with a gate-inert margin (rules and beam spans
            // never touch confident tokens by construction) instead of
            // infinity, which the `tag_margins` finite contract
            // forbids. History still advances through the dict tag, so
            // downstream positions see identical context whenever the
            // full decode agrees — byte-identity is measured, never
            // assumed (see the probe record in the train README).
            if let Some(dict_idx) = self.tagdict_lookup(&lower[i]) {
                tags.push(Tag::from_upos(TAGS[dict_idx]).expect("tagdict holds valid tags"));
                margins.push(FAST_PATH_MARGIN);
                prev2 = prev1;
                prev1 = TAGS[dict_idx];
                continue;
            }
            let acc = self.score_tag(words, &lower, i, prev1, prev2, &mut feats);
            // Single pass tracks best and runner-up; strict `>` keeps
            // the fixed TAGS order as tie-break.
            let mut best = 0;
            let mut second = f32::NEG_INFINITY;
            for t in 1..17 {
                if acc[t] > acc[best] {
                    second = acc[best];
                    best = t;
                } else if acc[t] > second {
                    second = acc[t];
                }
            }
            tags.push(Tag::from_upos(TAGS[best]).expect("fixed tag list is valid"));
            margins.push(acc[best] - second);
            prev2 = prev1;
            prev1 = TAGS[best];
        }
        (tags, margins, lower)
    }

    /// Score all 17 tags at position `i` under tag history
    /// (`prev1`, `prev2`). Single scoring path shared by greedy and
    /// beam decoding (refactored out of `decode`, bit-identical).
    /// Shape flags read the original-cased `words`; identity/affix
    /// features read `lower` — so no separate `raw` Vec is needed.
    fn score_tag<S: AsRef<str>>(
        &self,
        words: &[S],
        lower: &[String],
        i: usize,
        prev1: &str,
        prev2: &str,
        feats: &mut Vec<u64>,
    ) -> [f32; 17] {
        features(words, lower, i, prev1, prev2, feats);
        let mut acc = [0.0f32; 17];
        for f in feats.iter() {
            if let Some(arr) = self.weights.get(f) {
                for (a, w) in acc.iter_mut().zip(arr.iter()) {
                    *a += *w;
                }
            }
        }
        acc
    }

    /// Greedy decode, then a width-2 beam re-decode over low-margin
    /// spans. Returns `(tags, margins, (span_count, rescored_count))`.
    ///
    /// Algorithm: maximal runs of greedy margin `< margin_t` expand
    /// two tokens left (so the span can revise the history its first
    /// tokens were decoded under), merge across gaps ≤ 2, and drop
    /// past `max_span`. Each span runs a 2-best beam over tag
    /// histories — scoring is the model's own features, so the beam
    /// can only disagree with greedy where joint history beats local
    /// greed (the `this`-cascade shape: one wrong tag poisoning the
    /// rest through `t-1` features). Span margins are local
    /// best-minus-runner-up gaps recomputed under the winning
    /// history. Tokens right of a span keep greedy tags (their
    /// history changed silently — same class as the correction
    /// layer's snapshot semantics, documented there).
    fn decode_beam<S: AsRef<str>>(
        &self,
        words: &[S],
        margin_t: f32,
        max_span: usize,
    ) -> (Vec<Tag>, Vec<f32>, Vec<String>, (usize, usize)) {
        let (mut tags, mut margins, lower) = self.decode_lower(words);
        if words.is_empty() {
            return (tags, margins, lower, (0, 0));
        }
        // Index-space copy of greedy tags for history reads.
        let mut idx: Vec<usize> = tags
            .iter()
            .map(|t| tag_index(t.upos()).expect("tags come from TAGS"))
            .collect();
        // Maximal low-margin runs, each expanded two tokens left
        // (so the span can revise the history its first tokens were
        // decoded under), merged across gaps of ≤ 2, dropped past
        // `max_span` (those keep greedy tags).
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut i = 0;
        while i < margins.len() {
            if margins[i] < margin_t {
                let lo = i.saturating_sub(2);
                let mut hi = i;
                while hi + 1 < margins.len() && margins[hi + 1] < margin_t {
                    hi += 1;
                }
                match spans.last_mut() {
                    // Expanded `lo` reaches the previous span (gap ≤
                    // 2 between runs): extend it instead — and drop
                    // the merged span past the cap (greedy stands).
                    Some(last) if lo <= last.1 + 1 => {
                        last.1 = hi;
                        if last.1 + 1 - last.0 > max_span {
                            spans.pop();
                        }
                    }
                    _ => {
                        if hi + 1 - lo <= max_span {
                            spans.push((lo, hi));
                        }
                    }
                }
                i = hi + 1;
            } else {
                i += 1;
            }
        }
        let mut feats = Vec::with_capacity(20);
        let mut rescored = 0;
        for (lo, hi) in &spans {
            // History just left of the span, borrowed from the tag
            // statics — no allocation (the old code built `String`s).
            let (b1, b2) = (
                if *lo > 0 { TAGS[idx[*lo - 1]] } else { START1 },
                if *lo > 1 {
                    TAGS[idx[*lo - 2]]
                } else if *lo == 1 {
                    START1
                } else {
                    START2
                },
            );
            // Beam: (cumulative score, tag-index history).
            let mut hyps: Vec<(f32, Vec<usize>)> = vec![(0.0, Vec::with_capacity(hi + 1 - lo))];
            for (k, pos) in (*lo..=*hi).enumerate() {
                let mut cands: Vec<(f32, Vec<usize>)> = Vec::with_capacity(hyps.len() * 17);
                for (score, hist) in &hyps {
                    let p1 = if k >= 1 { TAGS[hist[k - 1]] } else { b1 };
                    let p2 = if k >= 2 {
                        TAGS[hist[k - 2]]
                    } else if k == 1 {
                        b1
                    } else {
                        b2
                    };
                    let acc = self.score_tag(words, &lower, pos, p1, p2, &mut feats);
                    for t in 0..17 {
                        let mut h = hist.clone();
                        h.push(t);
                        cands.push((score + acc[t], h));
                    }
                }
                cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                cands.truncate(2);
                hyps = cands;
            }
            let winner = &hyps[0].1;
            // Commit winner tags; recompute local margins under the
            // winning history for the correction gate.
            for (k, pos) in (*lo..=*hi).enumerate() {
                let p1 = if k >= 1 { TAGS[winner[k - 1]] } else { b1 };
                let p2 = if k >= 2 {
                    TAGS[winner[k - 2]]
                } else if k == 1 {
                    b1
                } else {
                    b2
                };
                let acc = self.score_tag(words, &lower, pos, p1, p2, &mut feats);
                let mut best = 0;
                let mut second = f32::NEG_INFINITY;
                for t in 1..17 {
                    if acc[t] > acc[best] {
                        second = acc[best];
                        best = t;
                    } else if acc[t] > second {
                        second = acc[t];
                    }
                }
                idx[pos] = best;
                tags[pos] = Tag::from_upos(TAGS[best]).expect("fixed tag list is valid");
                margins[pos] = acc[best] - second;
                rescored += 1;
            }
        }
        (tags, margins, lower, (spans.len(), rescored))
    }

    /// Per-tag weights for one feature id, or `None` when the feature
    /// never survived training. Introspection for forensics tooling
    /// (re-derive ids with [`hash_feature`]); decoding never calls this.
    pub fn feature_weights(&self, id: u64) -> Option<[f32; 17]> {
        self.weights.get(&id).copied()
    }

    /// Deserialize weights written by the trainer. Unknown tag codes are
    /// an error (fail loudly on a corrupt artifact, not silently sparse).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let raw: std::collections::BTreeMap<u64, HashMap<String, f32>> =
            serde_json::from_str::<serde_json::Value>(json)
                .and_then(|v| serde_json::from_value(v["weights"].clone()))?;
        let mut weights: U64Map<[f32; 17]> =
            HashMap::with_capacity_and_hasher(raw.len(), BuildHasherDefault::default());
        for (f, per_tag) in raw {
            let mut arr = [0.0f32; 17];
            for (code, w) in &per_tag {
                match tag_index(code) {
                    Some(t) => arr[t] = *w,
                    None => {
                        return Err(serde::de::Error::custom(format!(
                            "unknown tag code {code:?}"
                        )));
                    }
                }
            }
            weights.insert(f, arr);
        }
        Ok(Model {
            weights,
            tagdict: Self::parse_tagdict(&serde_json::from_str::<serde_json::Value>(json)?),
        })
    }

    /// Build the inference fast-path table: lowercased words seen under
    /// exactly one tag in training, sorted for binary search. Serialized
    /// with the weights; empty for ambiguous words (context decides).
    fn build_tagdict(
        data: &[(Vec<String>, Vec<String>)],
    ) -> HashMap<String, u8, BuildHasherDefault<U64Hasher>> {
        let mut seen: std::collections::HashMap<String, (usize, bool)> =
            std::collections::HashMap::new();
        for (raw, gold) in data {
            for (w, g) in raw.iter().zip(gold.iter()) {
                let e = seen.entry(w.to_lowercase()).or_insert_with(|| {
                    (
                        tag_index(g).expect("training tag must be a UPOS code"),
                        true,
                    )
                });
                if tag_index(g).expect("training tag must be a UPOS code") != e.0 {
                    e.1 = false;
                }
            }
        }
        let mut out: HashMap<String, u8, BuildHasherDefault<U64Hasher>> = HashMap::default();
        for (w, (t, single)) in seen {
            if single {
                out.insert(w, t as u8);
            }
        }
        out
    }

    /// Parse the optional `tagdict` table (sorted `[[word, tagidx]]`
    /// pairs); absent in weight files written before it existed.
    fn parse_tagdict(v: &serde_json::Value) -> HashMap<String, u8, BuildHasherDefault<U64Hasher>> {
        let mut out: HashMap<String, u8, BuildHasherDefault<U64Hasher>> = HashMap::default();
        if let Some(arr) = v.get("tagdict").and_then(|t| t.as_array()) {
            for pair in arr {
                let w = pair.get(0).and_then(|w| w.as_str()).unwrap_or_default();
                let t = pair.get(1).and_then(|t| t.as_u64()).unwrap_or(17) as usize;
                if !w.is_empty() && t < 17 {
                    out.insert(w.to_string(), t as u8);
                }
            }
        }
        out
    }

    /// Look up a fast-path tag index for a pre-lowered word.
    fn tagdict_lookup(&self, lower: &str) -> Option<usize> {
        self.tagdict.get(lower).map(|t| *t as usize)
    }

    /// Serialize weights for committing: compact JSON with sorted keys,
    /// so retrains diff cleanly. Compact (not pretty) keeps the artifact
    /// well under the 10 MB budget; `treefmt` excludes the weights
    /// directory (generated file, like `src/*`). Exact-zero weights
    /// (canceled +1/-1 pairs) are omitted, as in training.
    ///
    /// Whole-number weights (every weight the perceptron produces —
    /// sums of ±1 updates) serialize as integers (`3`, not `3.0`),
    /// saving ~11% of the file. `from_json` reads both forms, so the
    /// encoding is backward compatible.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let sorted: std::collections::BTreeMap<
            u64,
            std::collections::BTreeMap<String, serde_json::Value>,
        > = self
            .weights
            .iter()
            .map(|(f, arr)| {
                (
                    *f,
                    TAGS.iter()
                        .enumerate()
                        .filter(|(t, _)| arr[*t] != 0.0)
                        .map(|(t, code)| {
                            let w = arr[t];
                            let v =
                                if w.fract() == 0.0 && w >= i32::MIN as f32 && w <= i32::MAX as f32
                                {
                                    serde_json::Value::from(w as i32)
                                } else {
                                    serde_json::Number::from_f64(w as f64)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null)
                                };
                            (code.to_string(), v)
                        })
                        .collect(),
                )
            })
            .collect();
        let mut dict: Vec<(&String, &u8)> = self.tagdict.iter().collect();
        dict.sort();
        serde_json::to_string(&serde_json::json!({
            "weights": sorted,
            "tagdict": dict,
        }))
    }

    /// Fine-tune in-domain: more perceptron passes over `data starting
    /// from these weights (rather than from scratch). Few iters (2–4):
    /// the base already converges EWT, and a small in-domain set would
    /// otherwise shake shared weights with noisy updates (measured:
    /// 159 Moby tokens mixed into full joint training fix some misses
    /// but flip others). Counts — and therefore the min-count gate —
    /// are computed on `data` with threshold 1: every in-domain
    /// observation counts.
    pub fn finetune(&mut self, data: &[(Vec<String>, Vec<String>)], iters: usize) {
        // Weight updates can move a word off its dict tag; the fast
        // path must never shadow training, so it goes (finetuned
        // models are dev-only; committed weights train joint).
        self.tagdict.clear();
        let mut counts: U64Map<usize> = U64Map::default();
        let mut feats = Vec::with_capacity(20);
        for (raw, gold) in data {
            let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
            let mut prev1 = START1.to_string();
            let mut prev2 = START2.to_string();
            for (i, g) in gold.iter().enumerate() {
                features(raw, &lower, i, &prev1, &prev2, &mut feats);
                for f in &feats {
                    *counts.entry(*f).or_insert(0) += 1;
                }
                prev2 = std::mem::replace(&mut prev1, g.clone());
            }
        }

        for _ in 0..iters {
            for (raw, gold) in data {
                let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
                let mut prev1 = START1.to_string();
                let mut prev2 = START2.to_string();
                for (i, g) in gold.iter().enumerate() {
                    features(raw, &lower, i, &prev1, &prev2, &mut feats);
                    let mut acc = [0.0f32; 17];
                    for f in &feats {
                        if let Some(arr) = self.weights.get(f) {
                            for (a, w) in acc.iter_mut().zip(arr.iter()) {
                                *a += *w;
                            }
                        }
                    }
                    let mut best = 0;
                    for t in 1..17 {
                        if acc[t] > acc[best] {
                            best = t;
                        }
                    }
                    let gold_idx = tag_index(g).expect("training tag must be a UPOS code");
                    if best != gold_idx {
                        for f in &feats {
                            let arr = self.weights.entry(*f).or_insert([0.0; 17]);
                            arr[gold_idx] += 1.0;
                            arr[best] -= 1.0;
                        }
                    }
                    prev2 = std::mem::replace(&mut prev1, g.clone());
                }
            }
        }

        self.weights.retain(|_, arr| arr.iter().any(|w| *w != 0.0));
    }

    /// Fine-tune with frozen shared priors: like [`Model::finetune`],
    /// but features seen `freeze_at` or more times in `base` (the
    /// corpus the model originally trained on) never update — only
    /// novel or rare in-domain features move.
    ///
    /// Mechanism for the distillation drift (0-for-5): joint/finetune
    /// training lets a small oracle batch pull high-count shared
    /// features (`that` word-identity, tag bigrams, `be` forms),
    /// fixing book-domain misses while dragging EWT boundaries the
    /// other way. Masking per feature (not per token) keeps the
    /// in-domain lexical learning — a mispredicted `maddens` still
    /// trains its novel word-identity row — while the shared rows
    /// that EWT converged stay exactly put. `freeze_at` sweeps the
    /// boundary in the trainer (`--freeze-at`); `freeze_at = 0`
    /// freezes everything (no-op), `usize::MAX` is plain finetune.
    pub fn finetune_frozen(
        &mut self,
        base: &[(Vec<String>, Vec<String>)],
        data: &[(Vec<String>, Vec<String>)],
        iters: usize,
        freeze_at: usize,
    ) {
        // Same invalidation as `finetune`: updates can move words off
        // their dict tags.
        self.tagdict.clear();
        let mut counts: U64Map<usize> = U64Map::default();
        let mut feats = Vec::with_capacity(20);
        for (raw, gold) in base {
            let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
            let mut prev1 = START1;
            let mut prev2 = START2;
            for (i, g) in gold.iter().enumerate() {
                features(raw, &lower, i, prev1, prev2, &mut feats);
                for f in &feats {
                    *counts.entry(*f).or_insert(0) += 1;
                }
                prev2 = prev1;
                prev1 = g.as_str();
            }
        }

        for _ in 0..iters {
            for (raw, gold) in data {
                let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
                let mut prev1 = START1;
                let mut prev2 = START2;
                for (i, g) in gold.iter().enumerate() {
                    features(raw, &lower, i, prev1, prev2, &mut feats);
                    let mut acc = [0.0f32; 17];
                    for f in &feats {
                        if let Some(arr) = self.weights.get(f) {
                            for (a, w) in acc.iter_mut().zip(arr.iter()) {
                                *a += *w;
                            }
                        }
                    }
                    let mut best = 0;
                    for t in 1..17 {
                        if acc[t] > acc[best] {
                            best = t;
                        }
                    }
                    let gold_idx = tag_index(g).expect("training tag must be a UPOS code");
                    if best != gold_idx {
                        for f in &feats {
                            // Frozen shared rows sit out; novel/rare
                            // rows (missing counts as 0) still learn.
                            if counts.get(f).copied().unwrap_or(0) >= freeze_at {
                                continue;
                            }
                            let arr = self.weights.entry(*f).or_insert([0.0; 17]);
                            arr[gold_idx] += 1.0;
                            arr[best] -= 1.0;
                        }
                    }
                    prev2 = prev1;
                    prev1 = g.as_str();
                }
            }
        }

        self.weights.retain(|_, arr| arr.iter().any(|w| *w != 0.0));
    }

    /// Train on gold sentences of `(surface word, tag)` with the
    /// (unaveraged) perceptron for `iters` passes. `min_count` drops rare
    /// features (below threshold) to bound model size.
    ///
    /// Deliberately not averaged: averaging divides settled weights by
    /// total steps, so on fast-converging data the reliable early
    /// patterns shrink into noise while rarely-updated rare patterns
    /// dominate decoding (measured: 33% vs 88% dev on a 300-sentence
    /// pilot). Plain final-iteration weights win here.
    pub fn train(data: &[(Vec<String>, Vec<String>)], iters: usize, min_count: usize) -> Self {
        Self::train_impl(data, iters, min_count, false)
    }

    /// Train with history advanced from the model's own guesses.
    /// Probe of the Honnibal (2013) caveat: training history must come
    /// from the guesses, "otherwise it will be way over-reliant on the
    /// tag-history features". Updates still move toward gold; only the
    /// `prev1`/`prev2` carried to the next position is the predicted
    /// tag instead of the gold tag. The counts pass stays on gold
    /// history (gating only, no learning). Admission needs EWT
    /// dev/test ≥ the `train` baseline plus neutral-or-better evals.
    pub fn train_guessed_history(
        data: &[(Vec<String>, Vec<String>)],
        iters: usize,
        min_count: usize,
    ) -> Self {
        Self::train_impl(data, iters, min_count, true)
    }

    fn train_impl(
        data: &[(Vec<String>, Vec<String>)],
        iters: usize,
        min_count: usize,
        guessed_history: bool,
    ) -> Self {
        let mut counts: U64Map<usize> = U64Map::default();
        let mut feats = Vec::with_capacity(20);
        for (raw, gold) in data {
            let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
            let mut prev1 = START1.to_string();
            let mut prev2 = START2.to_string();
            for (i, g) in gold.iter().enumerate() {
                features(raw, &lower, i, &prev1, &prev2, &mut feats);
                for f in &feats {
                    *counts.entry(*f).or_insert(0) += 1;
                }
                prev2 = std::mem::replace(&mut prev1, g.clone());
            }
        }
        let mut weights: U64Map<[f32; 17]> = U64Map::default();
        let tagdict = Self::build_tagdict(data);

        for _ in 0..iters {
            for (raw, gold) in data {
                let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
                let mut prev1 = START1.to_string();
                let mut prev2 = START2.to_string();
                for (i, g) in gold.iter().enumerate() {
                    features(raw, &lower, i, &prev1, &prev2, &mut feats);
                    // Missing rows contribute 0, as the old
                    // `filter_map` skip did.
                    let mut acc = [0.0f32; 17];
                    for f in feats
                        .iter()
                        .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                    {
                        if let Some(arr) = weights.get(f) {
                            for (a, w) in acc.iter_mut().zip(arr.iter()) {
                                *a += *w;
                            }
                        }
                    }
                    let mut best = 0;
                    for t in 1..17 {
                        if acc[t] > acc[best] {
                            best = t;
                        }
                    }
                    let gold_idx = tag_index(g).expect("training tag must be a UPOS code");
                    if best != gold_idx {
                        for f in feats
                            .iter()
                            .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                        {
                            let arr = weights.entry(*f).or_insert([0.0; 17]);
                            arr[gold_idx] += 1.0;
                            arr[best] -= 1.0;
                        }
                    }
                    // History models inference: advance from the guess
                    // when probing (Honnibal 2013), else gold as before.
                    // The counts pass above stays on gold (gating only).
                    let next_hist = if guessed_history {
                        TAGS[best].to_string()
                    } else {
                        g.clone()
                    };
                    prev2 = std::mem::replace(&mut prev1, next_hist);
                }
            }
        }

        // Drop all-zero rows (canceled +1/-1 pairs) and enforce
        // min_count on the final map.
        weights.retain(|f, arr| {
            arr.iter().any(|w| *w != 0.0) && counts.get(f).is_some_and(|&c| c >= min_count)
        });
        Model { weights, tagdict }
    }
}
