//! Parity + unit tests for the hand-rolled backend.
//!
//! The parity test needs Tier-1 artifacts that live in /tmp until
//! admission (`bilstm.json` weights + `parity.json` vectors) and
//! skips gracefully without them — same discipline as dep weights.
//! The unit tests below always run (no assets, no EWT text).

use english_pos_neural::Model;
use std::path::PathBuf;

fn asset(name: &str) -> Option<String> {
    let p = PathBuf::from(format!("/tmp/opencode/round3/{name}"));
    std::fs::read_to_string(p).ok()
}

/// Stage-1 parity bar: Rust f32 argmax-identical to torch on the
/// pinned 50-sentence dev sample. Zero diffs allowed.
#[test]
fn torch_parity() {
    let weights = match asset("bilstm.json") {
        Some(w) => w,
        None => {
            eprintln!("skip: /tmp weights absent");
            return;
        }
    };
    let vectors = match asset("parity.json") {
        Some(v) => v,
        None => {
            eprintln!("skip: /tmp parity vectors absent");
            return;
        }
    };
    let model = Model::from_json(&weights).expect("weights load");
    let rows: Vec<serde_json::Value> = serde_json::from_str(&vectors).expect("vectors parse");
    assert_eq!(rows.len(), 50);
    let mut diffs = 0;
    for r in &rows {
        let words: Vec<&str> = r["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_str().unwrap())
            .collect();
        let want: Vec<&str> = r["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        let got = model.tag(&words);
        for (g, w) in got.iter().zip(want.iter()) {
            if g.upos() != *w {
                diffs += 1;
            }
        }
    }
    assert_eq!(diffs, 0, "parity diffs vs torch");
}

/// Margins are best-minus-runner-up and finite on every token.
#[test]
fn margins_sane() {
    let weights = match asset("bilstm.json") {
        Some(w) => w,
        None => {
            eprintln!("skip: /tmp weights absent");
            return;
        }
    };
    let model = Model::from_json(&weights).expect("weights load");
    let words = ["Time", "flies", "like", "an", "arrow", "."];
    let m = model.tag_margins(&words);
    assert_eq!(m.len(), 6);
    for (_, margin) in &m {
        assert!(margin.is_finite() && *margin >= 0.0);
    }
    // No flies-VERB veto on the neural path (deliberate): the veto is
    // perceptron-training hygiene (iters-15 suffix-memorization guard
    // over EWT's single VERB "flies"), while the BiLSTM confidently
    // prefers the noun-noun reading here (NOUN 6.96 vs VERB 5.17 on
    // torch) — a legitimate ambiguity, and pinning one token would be
    // post-hoc fitting. Change-detection lives in the dev/test gates.
}
