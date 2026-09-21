//! Demo: parse a text file with the `english` crate and POS-tag every
//! word with the committed weights.
//!
//! Usage: `cargo run -p english-pos --example tag -- <file>`
//!
//! Note the tokenization caveat from the README: `english` words keep
//! contractions whole (`don't`) where UD splits them, so contraction
//! tags come from whole-word features.

use english_pos::Model;

fn main() {
    let path = std::env::args().nth(1).expect("usage: tag <file>");
    let text = std::fs::read_to_string(&path).expect("failed to read input");
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");
    let doc = english::Document::parse(text);
    for para in doc.paragraphs() {
        for sent in para.sentences() {
            let mut words = Vec::new();
            for clause in sent.clauses() {
                // Subordinators are not Words; include their text so
                // nothing is silently dropped (see Clause::subordinator).
                words.extend(clause.subordinator().into_iter().map(str::to_string));
                words.extend(clause.words().iter().map(|w| w.text().to_string()));
            }
            let tags = model.tag(&words);
            for (w, t) in words.iter().zip(&tags) {
                print!("{w}/{t} ");
            }
            println!();
        }
    }
}
