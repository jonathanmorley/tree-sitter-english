//! Demo: parse a text file with the `english` crate and POS-tag every
//! token with the committed weights.
//!
//! Usage: `cargo run -p english-pos --example tag -- <file>`
//!
//! Wired via [`english_pos::tag_sentence`]: sentence tokens (no drops)
//! expanded into UD pieces (contractions split, curly apostrophes
//! normalized), then tagged. Hidden punctuation (commas, sentence-final
//! marks) has no grammar node and is not tagged.

use english_pos::{Model, tag_sentence};

fn main() {
    let path = std::env::args().nth(1).expect("usage: tag <file>");
    let text = std::fs::read_to_string(&path).expect("failed to read input");
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");
    let doc = english::Document::parse(text);
    for para in doc.paragraphs() {
        for sent in para.sentences() {
            for (w, t) in tag_sentence(&model, &sent) {
                print!("{w}/{t} ");
            }
            println!();
        }
    }
}
