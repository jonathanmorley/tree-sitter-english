//! Dev-time production probe for the neural path (Stage 4).
//! Greedy neural tags + the shipped `english-pos` correction rules
//! (`apply_rules` is model-agnostic over `(Tag, margin)` pairs) over
//! EWT dev/test, with per-rule fire counts. Compares against neural
//! greedy and the committed perceptron production numbers
//! (dev 93.67 / test 94.33). Rules fire only inside their calibrated
//! `0 < margin < threshold` gates — neural logit gaps run a different
//! scale than perceptron sums, so fire counts are the finding.
//!
//! ```sh
//! cargo run --release -p english-pos-neural --example prod_eval -- \
//!   /tmp/opencode/round3/bilstm.json
//! ```

use english_pos::{RULES, Tag, apply_rules};
use english_pos_neural::{Model, QModel};
use std::collections::HashMap;

/// UPOS -> universal-12, same projection as `eval_ud` (Petrov Table 1).
fn coarse(tag: &str) -> &str {
    match tag {
        "PROPN" => "NOUN",
        "AUX" => "VERB",
        "CCONJ" | "SCONJ" => "CONJ",
        "PART" => "PRT",
        "PUNCT" | "SYM" => ".",
        "INTJ" => "X",
        t => t,
    }
}

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
    let weights = std::fs::read_to_string(std::env::args().nth(1).expect("weights")).unwrap();
    enum Any {
        F(Model),
        Q(QModel),
    }
    let model = if weights.contains("\"qparams\"") {
        Any::Q(QModel::from_json(&weights).expect("i8 weights load"))
    } else {
        Any::F(Model::from_json(&weights).expect("weights load"))
    };
    for (name, path) in [
        ("dev", "/tmp/ud/ewt/en_ewt-ud-dev.conllu"),
        ("test", "/tmp/ud/ewt/en_ewt-ud-test.conllu"),
    ] {
        let data = read(path);
        let (mut gok, mut pok) = (0usize, 0usize);
        let (mut gcok, mut pcok) = (0usize, 0usize);
        let mut tot = 0usize;
        let mut fires: HashMap<&str, usize> = HashMap::new();
        for s in &data {
            let words: Vec<&str> = s.iter().map(|(w, _)| w.as_str()).collect();
            let low: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
            let mut tagged: Vec<(Tag, f32)> = match &model {
                Any::F(m) => m.tag_margins(&words),
                Any::Q(m) => m.tag_margins(&words),
            };
            let greedy: Vec<Tag> = tagged.iter().map(|(t, _)| *t).collect();
            let before: Vec<Tag> = greedy.clone();
            let snap = before.clone();
            apply_rules(&mut tagged, RULES, &low);
            // Attribute each changed token to its first firing rule by
            // replaying the gate chain on the pre-rule snapshot (same
            // first-fire semantics as apply_rules).
            for (i, (b, (p, m))) in before.iter().zip(tagged.iter()).enumerate() {
                if b != p {
                    if let Some(r) = RULES.iter().find(|r| {
                        *m > 0.0 && *m < r.threshold && (r.test)(&snap, &low, i).is_some()
                    }) {
                        *fires.entry(r.name).or_insert(0) += 1;
                    }
                }
            }
            for ((_, gold), (g, (p, _))) in s.iter().zip(greedy.iter().zip(tagged.iter())) {
                tot += 1;
                gok += (g.upos() == *gold) as usize;
                pok += (p.upos() == *gold) as usize;
                gcok += (coarse(g.upos()) == coarse(gold)) as usize;
                pcok += (coarse(p.upos()) == coarse(gold)) as usize;
            }
        }
        println!(
            "{name}: greedy {gok}/{tot} = {:.4} (+rules {pok}/{tot} = {:.4})  coarse {:.4} (+rules {:.4})",
            gok as f64 / tot as f64,
            pok as f64 / tot as f64,
            gcok as f64 / tot as f64,
            pcok as f64 / tot as f64
        );
        let mut fs: Vec<(&str, usize)> = fires.into_iter().collect();
        fs.sort();
        println!("  fires: {fs:?}");
    }
}
