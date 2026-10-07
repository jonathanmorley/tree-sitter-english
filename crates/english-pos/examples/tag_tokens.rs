//! Demo: POS-tag whitespace-separated tokens (one per line) with the
//! committed weights. Unlike `tag`, this takes UD-style pre-tokenized
//! input (punctuation split off, contractions split) instead of parsing
//! prose.
//!
//! Usage: `cargo run -p english-pos --example tag_tokens -- <tokens-file>`
//!
//! With `--sentences`, blank lines separate sentences and each block
//! is decoded independently (tag history resets, as production
//! `tag_sentence` does). Without it the whole file decodes as one
//! flat stream (cross-sentence history leaks — measurably worse on
//! EWT test, so accuracy work always uses `--sentences`).

use english_pos::Model;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: tag_tokens <file> [--sentences]");
    let per_sent = args.any(|a| a == "--sentences");
    let text = std::fs::read_to_string(&path).expect("failed to read input");
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");
    if per_sent {
        for block in text.split("\n\n") {
            let tokens: Vec<&str> = block.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }
            for (tok, tag) in tokens.iter().zip(model.tag(&tokens)) {
                println!("{tok}\t{tag}");
            }
            println!();
        }
        return;
    }
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (tok, tag) in tokens.iter().zip(model.tag(&tokens)) {
        println!("{tok}\t{tag}");
    }
}
