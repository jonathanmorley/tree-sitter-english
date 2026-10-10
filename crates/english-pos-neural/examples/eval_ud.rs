//! Dev-time UD accuracy gate for the neural path (Stage 1).
//! Reads Tier-1 /tmp weights + out-of-repo UD data (never vendored)
//! and reports greedy exact + OOV split, the binding bar being the
//! committed greedy numbers (dev 93.58 / test 94.23).
//!
//! ```sh
//! cargo run --release -p english-pos-neural --example eval_ud -- \
//!   /tmp/opencode/round3/bilstm.json
//! ```

use english_pos_neural::{Model, QModel};
use std::collections::HashSet;

fn read(path: &str) -> Vec<Vec<(String, String)>> {
    let mut sents = Vec::new();
    let mut cur = Vec::new();
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            if !cur.is_empty() {
                sents.push(std::mem::take(&mut cur));
            }
            continue;
        }
        let c: Vec<&str> = s.split('\t').collect();
        if c.len() < 5 || c[0].contains('-') || c[0].contains('.') {
            continue;
        }
        cur.push((c[1].to_string(), c[3].to_string()));
    }
    if !cur.is_empty() {
        sents.push(cur);
    }
    sents
}

fn main() {
    let weights_path = std::env::args().nth(1).expect("weights path");
    let weights = std::fs::read_to_string(weights_path).unwrap();
    // Auto-detect artifact kind by top-level key (f32 "params" vs
    // int8 "qparams"); both paths report the same gates.
    let quant = weights.contains("\"qparams\"");
    let fmodel;
    let qmodel;
    if quant {
        qmodel = Some(QModel::from_json(&weights).expect("i8 weights load"));
        fmodel = None;
    } else {
        fmodel = Some(Model::from_json(&weights).expect("weights load"));
        qmodel = None;
    }
    let train = read("/tmp/ud/ewt/en_ewt-ud-train.conllu");
    let vocab: HashSet<String> = train
        .iter()
        .flatten()
        .map(|(w, _)| w.to_lowercase())
        .collect();
    for (name, path) in [
        ("dev", "/tmp/ud/ewt/en_ewt-ud-dev.conllu"),
        ("test", "/tmp/ud/ewt/en_ewt-ud-test.conllu"),
    ] {
        let data = read(path);
        let (mut ok, mut tot) = (0usize, 0usize);
        let (mut ook, mut ot) = (0usize, 0usize);
        for s in &data {
            let words: Vec<&str> = s.iter().map(|(w, _)| w.as_str()).collect();
            let got: Vec<english_pos::Tag> = match (&fmodel, &qmodel) {
                (Some(m), _) => m.tag(&words),
                (_, Some(m)) => m.tag(&words),
                _ => unreachable!(),
            };
            for ((w, gold), g) in s.iter().zip(got.iter()) {
                tot += 1;
                let hit = g.upos() == *gold;
                ok += hit as usize;
                if !vocab.contains(&w.to_lowercase()) {
                    ot += 1;
                    ook += hit as usize;
                }
            }
        }
        println!(
            "{name}: exact {ok}/{tot} = {:.4}  oov {ook}/{ot} = {:.4}",
            ok as f64 / tot as f64,
            ook as f64 / ot as f64
        );
    }
}
