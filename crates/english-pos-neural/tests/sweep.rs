//! Sweep book-eval on the neural path (Stage-1 accuracy gate).
//!
//! Gold table ported mechanically from
//! `crates/english-pos/tests/sweep.rs` (60 hand-tagged book sentences:
//! 20 each Austen/Doyle/Stevenson) by `/tmp/opencode/round3/portsweep.py`
//! (verify-then-write: 120 arrays, strict words/tags alternation,
//! closed 17-tag map, per-pair length equality). WORD rows are grammar
//! pieces, so they feed `Model::tag` directly with no parsing.
//! Regenerate (never hand-edit) if the source table moves — the table
//! itself lives in `tables/sweep.rs`, shared with the quant gate.

use english_pos_neural::Model;
use std::path::PathBuf;

#[path = "tables/sweep.rs"]
mod tables;

/// Neural sweep gate: greedy exact meets the bar the perceptron
/// production path holds (0.87 token exact; committed neural screen
/// targets parity-or-better — see docs/neural-scope.md Stage 1).
#[test]
fn sweep_neural_meets_bar() {
    let path = PathBuf::from("/tmp/opencode/round3/bilstm.json");
    let weights = match std::fs::read_to_string(&path) {
        Ok(w) => w,
        Err(_) => {
            eprintln!("skip: /tmp weights absent");
            return;
        }
    };
    let model = Model::from_json(&weights).expect("weights load");
    let mut ok = 0usize;
    let mut tot = 0usize;
    for (_, _, words, gold) in tables::SENTENCES {
        let got = model.tag(words);
        assert_eq!(got.len(), gold.len());
        for (g, want) in got.iter().zip(gold.iter()) {
            tot += 1;
            if g.upos() == *want {
                ok += 1;
            }
        }
    }
    let acc = ok as f64 / tot as f64;
    eprintln!("neural sweep: {ok}/{tot} = {acc:.4}");
    assert!(acc >= 0.87, "sweep bar");
}
