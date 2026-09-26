//! Benchmark: end-to-end parse + POS-tag throughput on a text file.
//!
//! Splits wall time into parse (`english`), piece extraction (wiring),
//! and tag (`english-pos`), reporting medians over N runs plus
//! end-to-end tokens/sec. Follows the AGENTS.md scale convention:
//! when the text contains `CHAPTER 1. Loomings.`, measurement starts
//! there (skipping Gutenberg front matter).
//!
//! Usage: `cargo run --release -p english-pos --example bench -- <file> [iters]`
//!
//! Always run with `--release`: debug builds are ~10x slower and do not
//! reflect tag-time cost.

use std::time::Instant;

use english_pos::{Model, TagCache, sentence_pieces};

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[xs.len() / 2]
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: bench <file> [iters]");
    let iters: usize = std::env::args()
        .nth(2)
        .map(|s| s.parse().expect("iters must be a number"))
        .unwrap_or(5);
    let text = std::fs::read_to_string(&path).expect("failed to read input");
    // Scale-corpus convention: measure from CHAPTER 1 onward.
    let text = match text.find("CHAPTER 1. Loomings.") {
        Some(i) => &text[i..],
        None => &text,
    };
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");

    let mut parse_ms = Vec::with_capacity(iters);
    let mut pieces_ms = Vec::with_capacity(iters);
    let mut tag_ms = Vec::with_capacity(iters);
    let (mut n_sent, mut n_tok) = (0, 0);
    for _ in 0..iters {
        let t = Instant::now();
        let doc = english::Document::parse(text);
        parse_ms.push(t.elapsed().as_secs_f64() * 1000.0);

        let sents: Vec<_> = doc
            .paragraphs()
            .iter()
            .flat_map(|p| p.sentences())
            .collect();
        let t = Instant::now();
        let pieced: Vec<Vec<String>> = sents.iter().map(sentence_pieces).collect();
        pieces_ms.push(t.elapsed().as_secs_f64() * 1000.0);

        let t = Instant::now();
        for pieces in &pieced {
            let _ = model.tag(pieces);
        }
        tag_ms.push(t.elapsed().as_secs_f64() * 1000.0);

        n_sent = sents.len();
        n_tok = pieced.iter().map(Vec::len).sum();
    }
    let (parse, pieces, tag) = (median(parse_ms), median(pieces_ms), median(tag_ms));
    let total_s = (parse + pieces + tag) / 1000.0;
    println!(
        "{}: {} bytes, {} sentences, {} pieces",
        path,
        text.len(),
        n_sent,
        n_tok
    );
    println!("parse (tree-sitter):  {parse:.1} ms");
    println!("pieces (wiring):      {pieces:.1} ms");
    println!("tag (perceptron):     {tag:.1} ms");
    println!(
        "end-to-end:           {:.0} tokens/sec",
        n_tok as f64 / total_s
    );
    println!(
        "per-sentence tag:       {:.1} us",
        tag * 1000.0 / n_sent.max(1) as f64
    );

    // Warm path: one-word edit, incremental re-parse, cached retag.
    // Replaces the first "whale" occurrence so exactly the sentences
    // containing it change.
    let edited = text.replacen("whale", "WHALE", 1);
    if edited != *text {
        let mut doc = english::Document::parse(text);
        let mut cache = TagCache::new();
        let t = Instant::now();
        cache.tag_document(&model, &doc);
        let fill_ms = t.elapsed().as_secs_f64() * 1000.0;
        let (fill_hits, fill_misses) = (cache.hits(), cache.misses());
        let t = Instant::now();
        doc.update(edited);
        let update_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let tagged = cache.tag_document(&model, &doc);
        let warm_ms = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(tagged.len(), n_sent);
        println!("--- one-word edit ---");
        println!("reparse (incremental): {update_ms:.1} ms (cold fill: {fill_ms:.1} ms)");
        println!("  fill: {fill_hits} hits / {fill_misses} misses (dup sentences hit)");
        println!(
            "retag (cached):         {warm_ms:.1} ms ({} hits / {} misses this pass)",
            cache.hits() - fill_hits,
            cache.misses() - fill_misses
        );
    }
}
