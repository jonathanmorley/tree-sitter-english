//! Dev-time speed bench for the neural path (Stage 2).
//! Tags EWT dev words in release and reports tok/s: single-thread
//! with a shared [`WordCache`], then two scoped threads (batch path
//! is thread-safe by construction: `Model` is `Sync`, caches stay
//! per-thread). Bar ≥20k (dep-batch class).
//!
//! ```sh
//! cargo run --release -p english-pos-neural --example bench -- \
//!   /tmp/opencode/round3/bilstm.json
//! ```

use english_pos_neural::{Model, QModel, WordCache};
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
        if c.len() < 5 || c[0].contains('-') || c[0].contains('.') {
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
    // Auto-detect artifact kind (f32 "params" vs int8 "qparams").
    enum Any {
        F(Model),
        Q(QModel),
    }
    impl Any {
        fn tag_cached(&self, cache: &mut WordCache, w: &[&str]) {
            match self {
                Any::F(m) => {
                    m.tag_cached(cache, w);
                }
                Any::Q(m) => {
                    m.tag_cached(cache, w);
                }
            }
        }
    }
    let model = Arc::new(if weights.contains("\"qparams\"") {
        Any::Q(QModel::from_json(&weights).expect("i8 weights load"))
    } else {
        Any::F(Model::from_json(&weights).expect("weights load"))
    });
    let dev = read("/tmp/ud/ewt/en_ewt-ud-dev.conllu");
    let toks: usize = dev.iter().map(Vec::len).sum();
    for s in dev.iter().take(20) {
        let w: Vec<&str> = s.iter().map(|x| x.as_str()).collect();
        let mut c = WordCache::new();
        model.tag_cached(&mut c, &w);
    }
    let t0 = Instant::now();
    let mut cache = WordCache::new();
    for s in &dev {
        let w: Vec<&str> = s.iter().map(|x| x.as_str()).collect();
        model.tag_cached(&mut cache, &w);
    }
    let dt = t0.elapsed().as_secs_f64();
    println!(
        "single: {toks} toks in {dt:.2}s = {:.0} tok/s",
        toks as f64 / dt
    );
    let t0 = Instant::now();
    let mid = dev.len() / 2;
    std::thread::scope(|scope| {
        for half in [&dev[..mid], &dev[mid..]] {
            let m = Arc::clone(&model);
            scope.spawn(move || {
                let mut cache = WordCache::new();
                for s in half {
                    let w: Vec<&str> = s.iter().map(|x| x.as_str()).collect();
                    m.tag_cached(&mut cache, &w);
                }
            });
        }
    });
    let dt = t0.elapsed().as_secs_f64();
    println!(
        "scoped-x2: {toks} toks in {dt:.2}s = {:.0} tok/s",
        toks as f64 / dt
    );
}
