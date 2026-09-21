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

pub use tag::Tag;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Pseudo-tags opening every sentence for tag-history features.
pub const START1: &str = "<START>";
pub const START2: &str = "<START2>";

/// All tags in a fixed order (decoding tie-breaks and JSON stability).
pub const TAGS: [&str; 17] = [
    "ADJ", "ADP", "ADV", "AUX", "CCONJ", "DET", "INTJ", "NOUN", "NUM", "PART", "PRON", "PROPN",
    "PUNCT", "SCONJ", "SYM", "VERB", "X",
];

/// Feature templates for position `i` given surface `raw` and lowercased
/// `lower` words plus the two previous tags. Classic greedy-perceptron
/// set (word identity, neighbors, tag bigrams, affixes, shape flags).
/// Shape flags read `raw` — callers must pass true surface text, not
/// pre-lowercased input, or title/upper never fire. Shared verbatim by
/// training and inference: any change here invalidates committed weights.
pub fn features(
    raw: &[String],
    lower: &[String],
    i: usize,
    prev1: &str,
    prev2: &str,
) -> Vec<String> {
    let mut feats = Vec::with_capacity(16);
    let w = &lower[i];
    feats.push("bias".to_string());
    feats.push(format!("w={w}"));
    feats.push(format!(
        "w-1={}",
        if i > 0 { &lower[i - 1] } else { START1 }
    ));
    feats.push(format!(
        "w+1={}",
        lower.get(i + 1).map(String::as_str).unwrap_or(START1)
    ));
    feats.push(format!("t-1={prev1}"));
    feats.push(format!("t-2={prev2}"));
    feats.push(format!("t-1+2={prev1}+{prev2}"));
    let chars: Vec<char> = w.chars().collect();
    for n in 1..=3 {
        if chars.len() >= n {
            feats.push(format!("pref{n}={}", chars[..n].iter().collect::<String>()));
            feats.push(format!(
                "suf{n}={}",
                chars[chars.len() - n..].iter().collect::<String>()
            ));
        }
    }
    let raw_word = &raw[i];
    if raw_word.chars().next().is_some_and(|c| c.is_uppercase())
        && raw_word.chars().skip(1).all(|c| !c.is_uppercase())
    {
        feats.push("title".to_string());
    }
    if raw_word.chars().any(|c| c.is_uppercase()) && raw_word.chars().all(|c| !c.is_lowercase()) {
        feats.push("upper".to_string());
    }
    if raw_word.chars().any(|c| c.is_numeric()) {
        feats.push("digit".to_string());
    }
    if raw_word.contains('-') {
        feats.push("hyphen".to_string());
    }
    feats
}

/// Greedy perceptron model: `weights[feature][tag]`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Model {
    weights: HashMap<String, HashMap<String, f32>>,
}

impl Model {
    /// Score of `tag` for the given active features.
    fn score(&self, feats: &[String], tag: &str) -> f32 {
        feats
            .iter()
            .filter_map(|f| self.weights.get(f)?.get(tag))
            .sum()
    }

    /// Greedy left-to-right decode of surface `words` (lowercased
    /// internally; shape features read the raw forms).
    pub fn tag<S: AsRef<str>>(&self, words: &[S]) -> Vec<Tag> {
        let lower: Vec<String> = words.iter().map(|w| w.as_ref().to_lowercase()).collect();
        let raw: Vec<String> = words.iter().map(|w| w.as_ref().to_string()).collect();
        let mut prev1 = START1.to_string();
        let mut prev2 = START2.to_string();
        let mut out = Vec::with_capacity(words.len());
        for i in 0..words.len() {
            let feats = features(&raw, &lower, i, &prev1, &prev2);
            let mut best = TAGS[0];
            let mut best_score = f32::NEG_INFINITY;
            for tag in TAGS {
                let s = self.score(&feats, tag);
                if s > best_score {
                    best_score = s;
                    best = tag;
                }
            }
            out.push(Tag::from_upos(best).expect("fixed tag list is valid"));
            prev2 = std::mem::replace(&mut prev1, best.to_string());
        }
        out
    }

    /// Deserialize weights written by the trainer.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize weights for committing: compact JSON with sorted keys,
    /// so retrains diff cleanly. Compact (not pretty) keeps the artifact
    /// under the 2 MB budget; `treefmt` excludes the weights directory
    /// (generated file, like `src/*`).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let sorted: std::collections::BTreeMap<String, std::collections::BTreeMap<String, f32>> =
            self.weights
                .iter()
                .map(|(f, m)| (f.clone(), m.iter().map(|(t, w)| (t.clone(), *w)).collect()))
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
        let mut counts: HashMap<String, usize> = HashMap::new();
        for (raw, gold) in data {
            let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
            let mut prev1 = START1.to_string();
            let mut prev2 = START2.to_string();
            for (i, g) in gold.iter().enumerate() {
                for f in features(raw, &lower, i, &prev1, &prev2) {
                    *counts.entry(f).or_insert(0) += 1;
                }
                prev2 = std::mem::replace(&mut prev1, g.clone());
            }
        }
        let mut weights: HashMap<String, HashMap<String, f32>> = HashMap::new();

        for _ in 0..iters {
            for (raw, gold) in data {
                let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
                let mut prev1 = START1.to_string();
                let mut prev2 = START2.to_string();
                for (i, g) in gold.iter().enumerate() {
                    let feats: Vec<String> = features(raw, &lower, i, &prev1, &prev2)
                        .into_iter()
                        .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                        .collect();
                    let mut best = TAGS[0];
                    let mut best_score = f32::NEG_INFINITY;
                    for tag in TAGS {
                        let s = feats
                            .iter()
                            .filter_map(|f| weights.get(f)?.get(tag))
                            .sum::<f32>();
                        if s > best_score {
                            best_score = s;
                            best = tag;
                        }
                    }
                    if best != g {
                        for f in &feats {
                            *weights
                                .entry(f.clone())
                                .or_default()
                                .entry(g.clone())
                                .or_insert(0.0) += 1.0;
                            *weights
                                .entry(f.clone())
                                .or_default()
                                .entry(best.to_string())
                                .or_insert(0.0) -= 1.0;
                        }
                    }
                    prev2 = std::mem::replace(&mut prev1, g.clone());
                }
            }
        }

        // Drop exact-zero weights (canceled +1/-1 pairs) and enforce
        // min_count on the final map.
        weights.retain(|f, per_tag| {
            per_tag.retain(|_, w| *w != 0.0);
            !per_tag.is_empty() && counts.get(f).is_some_and(|&c| c >= min_count)
        });
        Model { weights }
    }
}
