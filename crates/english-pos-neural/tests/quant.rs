//! Stage-3 quant gates (accuracy-neutral re-gate).
//!
//! Needs /tmp `bilstm-i8.json` (+ `parity.json` for the smoke band)
//! and skips without them. Pre-set bars: parity-sample diffs ≤ 10
//! tokens (~1% smoke; the decisive gates are dev/test within
//! run-wobble and sweep ≥ 0.87, measured via `eval_ud` + the sweep
//! suite on the i8 artifact).

use english_pos_neural::QModel;

#[path = "tables/sweep.rs"]
mod tables;

#[test]
fn quant_parity_band() {
    let weights = match std::fs::read_to_string("/tmp/opencode/round3/bilstm-i8.json") {
        Ok(w) => w,
        Err(_) => {
            eprintln!("skip: /tmp i8 weights absent");
            return;
        }
    };
    let vectors = match std::fs::read_to_string("/tmp/opencode/round3/parity.json") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skip: /tmp parity vectors absent");
            return;
        }
    };
    let model = QModel::from_json(&weights).expect("i8 weights load");
    let rows: Vec<serde_json::Value> = serde_json::from_str(&vectors).expect("vectors parse");
    let mut diffs = 0;
    let mut tot = 0;
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
        for (g, w) in model.tag(&words).iter().zip(want.iter()) {
            tot += 1;
            diffs += (g.upos() != *w) as usize;
        }
    }
    eprintln!("i8 parity band: {diffs}/{tot}");
    assert!(diffs <= 10, "quant smoke band");
}

/// Stage-3 sweep gate on the int8 artifact: same 0.87 bar as f32.
#[test]
fn quant_sweep_meets_bar() {
    let weights = match std::fs::read_to_string("/tmp/opencode/round3/bilstm-i8.json") {
        Ok(w) => w,
        Err(_) => {
            eprintln!("skip: /tmp i8 weights absent");
            return;
        }
    };
    let model = QModel::from_json(&weights).expect("i8 weights load");
    let mut ok = 0usize;
    let mut tot = 0usize;
    for (_, _, words, gold) in tables::SENTENCES {
        let got = model.tag(words);
        assert_eq!(got.len(), gold.len());
        for (g, want) in got.iter().zip(gold.iter()) {
            tot += 1;
            ok += (g.upos() == *want) as usize;
        }
    }
    let acc = ok as f64 / tot as f64;
    eprintln!("i8 sweep: {ok}/{tot} = {acc:.4}");
    assert!(acc >= 0.87, "i8 sweep bar");
}
