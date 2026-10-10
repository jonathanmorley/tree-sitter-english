//! Dev-time UD accuracy gate for the joint path (Stage 1).
//! Reports tag exact + UAS/LAS on dev/test/PUD from /tmp weights +
//! out-of-repo UD data (never vendored). Bars from
//! `docs/joint-neural-scope.md` (pipeline-above-banked per split).
//!
//! ```sh
//! cargo run --release -p english-joint --example eval_ud -- \
//!   /tmp/opencode/round3/joint.json
//! ```

use english_joint::{JointModel, QJointModel};

fn read(path: &str) -> Vec<Vec<(String, String, i32, String)>> {
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
        if c.len() < 10 || c[0].contains('-') || c[0].contains('.') {
            continue;
        }
        let head: i32 = c[6].parse::<i32>().map(|h| h - 1).unwrap_or(-2);
        cur.push((c[1].to_string(), c[3].to_string(), head, c[7].to_string()));
    }
    if !cur.is_empty() {
        sents.push(cur);
    }
    sents
}

fn main() {
    let weights = std::fs::read_to_string(std::env::args().nth(1).expect("weights")).unwrap();
    enum Any {
        F(JointModel),
        Q(QJointModel),
    }
    let model = if weights.contains("\"qparams\"") {
        Any::Q(QJointModel::from_json(&weights).expect("i8 weights load"))
    } else {
        Any::F(JointModel::from_json(&weights).expect("weights load"))
    };
    for (name, path) in [
        ("dev", "/tmp/ud/ewt/en_ewt-ud-dev.conllu"),
        ("test", "/tmp/ud/ewt/en_ewt-ud-test.conllu"),
        ("pud", "/tmp/ud/.cache/en_pud-ud-test.conllu"),
    ] {
        let data = read(path);
        let (mut tok, mut u, mut l) = (0usize, 0usize, 0usize);
        let mut tot = 0usize;
        for s in &data {
            let words: Vec<&str> = s.iter().map(|(w, _, _, _)| w.as_str()).collect();
            let (tags, heads, rels): (Vec<english_pos::Tag>, Vec<i32>, Vec<String>) = match &model {
                Any::F(m) => {
                    let p = m.parse(&words);
                    (p.tags, p.heads, p.rels)
                }
                Any::Q(m) => {
                    let p = m.parse(&words);
                    (p.tags, p.heads, p.rels)
                }
            };
            for ((_, g, gh, gr), ((t, h), r)) in
                s.iter().zip(tags.iter().zip(heads.iter()).zip(rels.iter()))
            {
                tot += 1;
                tok += (t.upos() == *g) as usize;
                u += (*h == *gh) as usize;
                l += (*h == *gh && *r == *gr) as usize;
            }
        }
        println!(
            "{name}: tag {tok}/{tot} = {:.4}  UAS {u}/{tot} = {:.4}  LAS {l}/{tot} = {:.4}",
            tok as f64 / tot as f64,
            u as f64 / tot as f64,
            l as f64 / tot as f64
        );
    }
}
