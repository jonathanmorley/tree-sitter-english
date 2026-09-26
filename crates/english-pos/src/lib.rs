//! Statistical part-of-speech tagging for English prose.
//!
//! A greedy perceptron tagger over the 17 Universal POS tags.
//! This crate holds the inference side: feature extraction shared with
//! training, the [`Model`] with greedy decoding, and JSON weight
//! (de)serialization. Training lives in `crates/english-pos-train`.
//!
//! Deliberately a post-parse pass, never grammar: the grammar segments
//! flat clauses of unclassified words (Tier 3 showed word classes do
//! not belong there); this crate annotates them with probabilities that
//! degrade gracefully instead of failing the tree.

mod tag;
mod wire;

pub use tag::Tag;
pub use wire::{
    TagCache, clause_pieces, sentence_pieces, split_contraction, tag_clause, tag_document,
    tag_sentence, token_pieces,
};

use std::collections::HashMap;

/// Pseudo-tags opening every sentence for tag-history features.
pub const START1: &str = "<START>";
pub const START2: &str = "<START2>";

/// All tags in a fixed order (decoding tie-breaks and JSON stability).
pub const TAGS: [&str; 17] = [
    "ADJ", "ADP", "ADV", "AUX", "CCONJ", "DET", "INTJ", "NOUN", "NUM", "PART", "PRON", "PROPN",
    "PUNCT", "SCONJ", "SYM", "VERB", "X",
];

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
pub fn features(
    raw: &[String],
    lower: &[String],
    i: usize,
    prev1: &str,
    prev2: &str,
    feats: &mut Vec<u64>,
) {
    feats.clear();
    let w = lower[i].as_str();
    feats.push(F_BIAS);
    feats.push(hash_feature(0x10, &[w]));
    feats.push(hash_feature(
        0x11,
        &[if i > 0 { &lower[i - 1] } else { START1 }],
    ));
    feats.push(hash_feature(
        0x12,
        &[lower.get(i + 1).map(String::as_str).unwrap_or(START1)],
    ));
    feats.push(hash_feature(0x13, &[prev1]));
    feats.push(hash_feature(0x14, &[prev2]));
    feats.push(hash_feature(0x15, &[prev1, prev2]));
    for n in 1..=3 {
        if let Some(p) = first_n(w, n) {
            feats.push(hash_feature(0x20 + n as u8, &[p]));
        }
        if let Some(s) = last_n(w, n) {
            feats.push(hash_feature(0x24 + n as u8, &[s]));
        }
    }
    let raw_word = raw[i].as_str();
    if raw_word.chars().next().is_some_and(|c| c.is_uppercase())
        && raw_word.chars().skip(1).all(|c| !c.is_uppercase())
    {
        feats.push(F_TITLE);
    }
    if raw_word.chars().any(|c| c.is_uppercase()) && raw_word.chars().all(|c| !c.is_lowercase()) {
        feats.push(F_UPPER);
    }
    if raw_word.chars().any(|c| c.is_numeric()) {
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
/// (feature, tag) pair. Serialized shape is unchanged
/// (`{"weights": {feature: {TAG: weight}}}`) with integer feature keys.
#[derive(Debug, Default, Clone)]
pub struct Model {
    weights: HashMap<u64, [f32; 17]>,
}

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

    fn decode<S: AsRef<str>>(&self, words: &[S]) -> (Vec<Tag>, Vec<f32>) {
        let lower: Vec<String> = words.iter().map(|w| w.as_ref().to_lowercase()).collect();
        let raw: Vec<String> = words.iter().map(|w| w.as_ref().to_string()).collect();
        let mut prev1 = START1.to_string();
        let mut prev2 = START2.to_string();
        let mut feats = Vec::with_capacity(20);
        let mut tags = Vec::with_capacity(words.len());
        let mut margins = Vec::with_capacity(words.len());
        for i in 0..words.len() {
            features(&raw, &lower, i, &prev1, &prev2, &mut feats);
            let mut acc = [0.0f32; 17];
            for f in &feats {
                if let Some(arr) = self.weights.get(f) {
                    for (a, w) in acc.iter_mut().zip(arr.iter()) {
                        *a += *w;
                    }
                }
            }
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
            prev2 = std::mem::replace(&mut prev1, TAGS[best].to_string());
        }
        (tags, margins)
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
        let mut weights = HashMap::with_capacity(raw.len());
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
        Ok(Model { weights })
    }

    /// Serialize weights for committing: compact JSON with sorted keys,
    /// so retrains diff cleanly. Compact (not pretty) keeps the artifact
    /// well under the 10 MB budget; `treefmt` excludes the weights
    /// directory (generated file, like `src/*`). Exact-zero weights
    /// (canceled +1/-1 pairs) are omitted, as in training.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let sorted: std::collections::BTreeMap<u64, std::collections::BTreeMap<String, f32>> = self
            .weights
            .iter()
            .map(|(f, arr)| {
                (
                    *f,
                    TAGS.iter()
                        .enumerate()
                        .filter(|(t, _)| arr[*t] != 0.0)
                        .map(|(t, code)| (code.to_string(), arr[t]))
                        .collect(),
                )
            })
            .collect();
        serde_json::to_string(&serde_json::json!({"weights": sorted}))
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
        let mut counts: HashMap<u64, usize> = HashMap::new();
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
        let mut weights: HashMap<u64, [f32; 17]> = HashMap::new();

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
                    prev2 = std::mem::replace(&mut prev1, g.clone());
                }
            }
        }

        // Drop all-zero rows (canceled +1/-1 pairs) and enforce
        // min_count on the final map.
        weights.retain(|f, arr| {
            arr.iter().any(|w| *w != 0.0) && counts.get(f).is_some_and(|&c| c >= min_count)
        });
        Model { weights }
    }
}
