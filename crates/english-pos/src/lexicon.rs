//! Shipped lexical backoff lists for gated correction rules.
//!
//! One wordlist (committed, provenance in the file header):
//! `verbs.txt`: EWT VERB lemmas — generated, see
//! `scripts/verb-lemmas.sh`. A NOUN-predicted `-s` word flips to
//! VERB only when its stem is a known base form; that membership
//! test is what separates `glitters`/`wears` from `theories` and
//! `status`, which killed the morphology-only rule.
//!
//! Lookup is binary search over sorted lines; the list is consulted
//! only below the margin gate, so steady-state cost is ~nil. This is
//! a backoff, never a tagdict: nothing here is consulted on the fast
//! path or bypasses the perceptron — the gate in `correction.rs`
//! decides every firing.

use std::sync::LazyLock;

fn load_list(text: &'static str) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    v.sort_unstable();
    v
}

static VERB_LEMMAS: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| load_list(include_str!("../lexicon/verbs.txt")));

fn contains(list: &[&str], word: &str) -> bool {
    list.binary_search(&word).is_ok()
}

/// True when `word` is a known verb base form or a regular `-s`
/// inflection of one (`wears`→`wear`, `watches`→`watch`,
/// `tries`/`flies`→`try`/`fly`). Lowercases first; never strips
/// `ss` (`glass`, `status`); bare `-s` words whose stem is not a
/// verb (`theories`→`theory`, `news`→`new`, `arms`→`arm` is — see
/// below) correctly abstain.
///
/// NOTE on `arms`/`means`/`thanks`: their stems (`arm`, `mean`,
/// `thank`) ARE verb lemmas, so this returns true and the calling
/// rule's context conditions (nominal prev, complement next) plus
/// the margin gate must carry the precision — `folded his arms`
/// (prev PRON, next end) is the known-risk shape. Measured, not
/// assumed: see `--correct` reporting.
pub fn known_verb_form(raw: &str) -> bool {
    let w = raw.to_lowercase();
    if contains(&VERB_LEMMAS, w.as_str()) {
        return true;
    }
    let b = w.as_bytes();
    if b.len() <= 3 || w.ends_with("ss") {
        return false;
    }
    if let Some(stripped) = w.strip_suffix("ies") {
        let mut cand = stripped.to_string();
        cand.push('y');
        if contains(&VERB_LEMMAS, cand.as_str()) {
            return true;
        }
    }
    if let Some(stripped) = w.strip_suffix("es")
        && contains(&VERB_LEMMAS, stripped)
    {
        return true;
    }
    if let Some(stripped) = w.strip_suffix('s') {
        return contains(&VERB_LEMMAS, stripped);
    }
    false
}
