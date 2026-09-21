//! Demo: POS-tag whitespace-separated tokens (one per line) with the
//! committed weights. Unlike `tag`, this takes UD-style pre-tokenized
//! input (punctuation split off, contractions split) instead of parsing
//! prose.
//!
//! Usage: `cargo run -p english-pos --example tag_tokens -- <tokens-file>`

use english_pos::Model;

fn main() {
    let path = std::env::args().nth(1).expect("usage: tag_tokens <file>");
    let text = std::fs::read_to_string(&path).expect("failed to read input");
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");
    for (tok, tag) in tokens.iter().zip(model.tag(&tokens)) {
        println!("{tok}\t{tag}");
    }
}
