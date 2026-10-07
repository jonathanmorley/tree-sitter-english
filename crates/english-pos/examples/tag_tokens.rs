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
//!
//! With `--production`, blocks decode through the production path
//! (beam re-decode + gated correction rules, as `tag_sentence`
//! wires them) instead of greedy `Model::tag`. The flags compose;
//! accuracy work uses both.

use english_pos::{Model, RULES, apply_rules};

fn decode(model: &Model, tokens: &[&str], production: bool) {
    if production {
        let (mut tagged, lower) = model.tag_beam_margins_lowered(tokens);
        if !RULES.is_empty() {
            apply_rules(&mut tagged, RULES, &lower);
        }
        for ((tag, _), tok) in tagged.into_iter().zip(tokens) {
            println!("{tok}\t{tag}");
        }
        return;
    }
    for (tok, tag) in tokens.iter().zip(model.tag(tokens)) {
        println!("{tok}\t{tag}");
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: tag_tokens <file> [--sentences] [--production]");
    let flags: Vec<String> = args.collect();
    let per_sent = flags.iter().any(|a| a == "--sentences");
    let production = flags.iter().any(|a| a == "--production");
    let text = std::fs::read_to_string(&path).expect("failed to read input");
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");
    if per_sent {
        for block in text.split("\n\n") {
            let tokens: Vec<&str> = block.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }
            decode(&model, &tokens, production);
            println!();
        }
        return;
    }
    let tokens: Vec<&str> = text.split_whitespace().collect();
    decode(&model, &tokens, production);
}
