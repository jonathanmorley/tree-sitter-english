//! Benchmark: chunking throughput on a text file, split by stage.
//!
//! Parses, tags every sentence, then chunks all tagged sentences,
//! reporting medians over N runs. Follows the scale convention:
//! when the text contains `CHAPTER 1. Loomings.`, measurement starts
//! there. The chunker must hold the keystroke budget established for
//! parse+tag (~47 ms on book-size input).
//!
//! Usage: `cargo run --release -p english-chunk --example chunk_bench -- <file> [iters]`

use std::time::Instant;

use english_chunk::chunk_tagged;

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[xs.len() / 2]
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: chunk_bench <file> [iters]");
    let iters: usize = std::env::args()
        .nth(2)
        .map(|s| s.parse().expect("iters must be a number"))
        .unwrap_or(5);
    let text = std::fs::read_to_string(&path).expect("failed to read input");
    let text = match text.find("CHAPTER 1. Loomings.") {
        Some(i) => &text[i..],
        None => &text,
    };
    let model =
        english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json"))
            .expect("invalid weights JSON");

    let mut chunk_ms = Vec::with_capacity(iters);
    let (mut n_sent, mut n_tok, mut n_chunks) = (0, 0, 0);
    for _ in 0..iters {
        let doc = english::Document::parse(text);
        let mut tagged_all = Vec::new();
        for para in doc.paragraphs() {
            for sent in para.sentences() {
                let pieces = english_pos::sentence_pieces(&sent);
                let tags = model.tag(&pieces);
                tagged_all.push(
                    pieces
                        .into_iter()
                        .zip(tags)
                        .map(|(w, t)| (w, t))
                        .collect::<Vec<_>>(),
                );
            }
        }
        let t = Instant::now();
        let mut chunks = 0;
        for tagged in &tagged_all {
            chunks += chunk_tagged(tagged).len();
        }
        chunk_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        n_sent = tagged_all.len();
        n_tok = tagged_all.iter().map(Vec::len).sum();
        n_chunks = chunks;
    }
    let chunk = median(chunk_ms);
    println!(
        "{}: {} sentences, {} pieces -> {} chunks",
        path, n_sent, n_tok, n_chunks
    );
    println!("chunk: {chunk:.1} ms ({:.1} us/sentence)", chunk * 1000.0 / n_sent.max(1) as f64);
}
