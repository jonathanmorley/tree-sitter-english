//! Dev-time speed bench for the joint path (Stage 2).
//! Parses EWT dev words in release: single-thread with a shared
//! cache, then two scoped threads. Bars from
//! `docs/joint-neural-scope.md` (batch viability vs tag+parse sum).
//!
//! ```sh
//! cargo run --release -p english-joint --example bench -- \
//!   /tmp/opencode/round3/joint.json
//! ```

use english_joint::JointModel;
use english_pos_neural::WordCache;
use std::sync::Arc;
use std::time::Instant;

fn read(path: &str) -> Vec<Vec<String>> {
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
        cur.push(c[1].to_string());
    }
    if !cur.is_empty() {
        sents.push(cur);
    }
    sents
}

fn main() {
    let weights = std::fs::read_to_string(std::env::args().nth(1).expect("weights")).unwrap();
    let model = Arc::new(JointModel::from_json(&weights).expect("weights load"));
    let dev = read("/tmp/ud/ewt/en_ewt-ud-dev.conllu");
    let toks: usize = dev.iter().map(Vec::len).sum();
    for s in dev.iter().take(10) {
        let w: Vec<&str> = s.iter().map(|x| x.as_str()).collect();
        let mut c = WordCache::new();
        model.parse_cached(&mut c, &w);
    }
    let t0 = Instant::now();
    let mut cache = WordCache::new();
    for s in &dev {
        let w: Vec<&str> = s.iter().map(|x| x.as_str()).collect();
        model.parse_cached(&mut cache, &w);
    }
    let dt = t0.elapsed().as_secs_f64();
    println!("single: {toks} toks in {dt:.2}s = {:.0} tok/s", toks as f64 / dt);
    let t0 = Instant::now();
    let mid = dev.len() / 2;
    std::thread::scope(|scope| {
        for half in [&dev[..mid], &dev[mid..]] {
            let m = Arc::clone(&model);
            scope.spawn(move || {
                let mut cache = WordCache::new();
                for s in half {
                    let w: Vec<&str> = s.iter().map(|x| x.as_str()).collect();
                    m.parse_cached(&mut cache, &w);
                }
            });
        }
    });
    let dt = t0.elapsed().as_secs_f64();
    println!("scoped-x2: {toks} toks in {dt:.2}s = {:.0} tok/s", toks as f64 / dt);
}
