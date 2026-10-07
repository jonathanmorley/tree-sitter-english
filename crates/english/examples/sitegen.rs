//! Site generator: parse curated showcase inputs with the NATIVE binary
//! and dump span-annotated JSON for the Pages structure demo
//! (`docs/parse-examples.json`).
//!
//! The browser cannot link the Rust external scanner (no wasm32 C
//! toolchain here; emscripten can't link Rust either), so the
//! structure demo renders THESE pre-parsed trees — zero fidelity
//! gap, honestly labeled. Regenerate with:
//! `cargo run -p english --example sitegen > docs/parse-examples.json`

use english::{Clause, Sentence};

fn token_json(t: &english::Token) -> serde_json::Value {
    serde_json::json!({
        "s": t.span().start,
        "e": t.span().end,
        "k": format!("{:?}", t.kind()),
    })
}

fn clause_json(c: &Clause) -> serde_json::Value {
    serde_json::json!({
        "s": c.span().start,
        "e": c.span().end,
        "sub": c.is_subordinate(),
        "tokens": c.tokens().iter().map(token_json).collect::<Vec<_>>(),
    })
}

fn sentence_json(s: &Sentence) -> serde_json::Value {
    serde_json::json!({
        "s": s.span().start,
        "e": s.span().end,
        "clauses": s.clauses().iter().map(clause_json).collect::<Vec<_>>(),
    })
}

fn main() {
    let examples = [
        ("Simple sentence", "Time flies like an arrow."),
        (
            "Abbreviations, time, coordination",
            "Mr. Smith arrived at 10:30, and Mrs. Jones left because the meeting ended.",
        ),
        ("Em-dash join", "She laughed — and then she cried."),
        (
            "Subordinator, semicolon, currency",
            "The book that I read was good; it cost $5.",
        ),
        ("Parenthetical", "The whale (a huge beast) swam on."),
        ("Colon elaboration", "He had one goal: to win."),
    ];
    let mut out = Vec::new();
    for (label, text) in examples {
        let doc = english::Document::parse(text.to_string());
        let mut errors = 0;
        let mut sentences = Vec::new();
        for para in doc.paragraphs() {
            for sent in para.sentences() {
                if sent.has_error() {
                    errors += 1;
                }
                sentences.push(sentence_json(&sent));
            }
        }
        out.push(serde_json::json!({
            "label": label,
            "text": text,
            "errors": errors,
            "sentences": sentences,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
