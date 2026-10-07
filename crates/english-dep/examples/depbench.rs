//! Decode-speed bench: greedy vs beam widths on EWT dev (gold
//! tags — isolates parser decode from tagger time). Standing
//! harness (tagger `bench` equivalent); verdict in AGENTS.md:
//! dep is a batch/save-pass stage (~4 s/book at beam4), never
//! keystroke.
use english_dep::Model;
use std::fs;
use std::time::Instant;

fn main() {
    let weights = fs::read_to_string("crates/english-dep/weights/dep.json").expect("weights");
    let model = Model::from_json(&weights).expect("parse weights");
    let corpus = fs::read_to_string("/tmp/opencode/ud-committed/en_ewt-ud-dev.conllu").unwrap();
    let mut sents: Vec<(Vec<String>, Vec<String>)> = vec![];
    let mut words: Vec<String> = vec![];
    let mut tags: Vec<String> = vec![];
    for line in corpus.lines().chain([""]) {
        let line = line.trim_end();
        if line.is_empty() {
            if !words.is_empty() {
                sents.push((std::mem::take(&mut words), std::mem::take(&mut tags)));
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 8 || cols[0].contains('-') || cols[0].contains('.') {
            continue;
        }
        words.push(cols[1].to_lowercase());
        tags.push(cols[3].to_string());
    }
    let toks: usize = sents.iter().map(|(w, _)| w.len()).sum();
    println!("sents={} toks={}", sents.len(), toks);
    for (name, width) in [("greedy", 0), ("beam2", 2), ("beam4", 4)] {
        let t0 = Instant::now();
        let mut n_attached = 0;
        for (w, t) in &sents {
            if width == 0 {
                n_attached += model.parse(w, t).len();
            } else {
                n_attached += model.parse_beam(w, t, width).0.len();
            }
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        println!("{name}: {ms:.1} ms total ({:.0} tok/s, attached={n_attached})", 1000.0 * toks as f64 / ms.max(1e-9));
    }
}
