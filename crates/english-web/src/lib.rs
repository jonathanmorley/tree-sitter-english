//! Browser demo backend: the REAL pipeline in WebAssembly — native
//! parse (grammar + Rust scanner) → production tag → chunk → shallow
//! lint. No naive splitting, no fidelity gap.
//!
//! This works because tree-sitter 0.27 was designed for it: the
//! runtime crate compiles `lib.c` with `TREE_SITTER_WASM_STDLIB` +
//! shim headers on wasm targets, forwards C allocation to Rust's
//! global allocator, and needs only a wasm-capable C compiler
//! (clang from locked nixpkgs:
//! `CC_wasm32_unknown_unknown=clang` +
//! `CFLAGS_wasm32_unknown_unknown="--target=wasm32-unknown-unknown -nostdlib"`).
//!
//! Dependency rules (passive, nominalization) stay native-only: their
//! weights (32 + 3.8 MB) are Tier-1 lazy assets, not browser freight.

use english_lint::{
    annotate_shallow, line_col, ClauseComplexity, Hedge, Rule, SentenceLength, Weasel,
};
use wasm_bindgen::prelude::*;

/// Tagger weights, embedded at compile time (same bytes as native).
static UPOS_JSON: &str = include_str!("../../english-pos/weights/upos.json");

/// Parsed once per page lifetime: re-parsing 1.76 MB of weights on
/// every keystroke would defeat the keystroke path.
static TAGGER: std::sync::OnceLock<english_pos::Model> = std::sync::OnceLock::new();

fn tagger() -> &'static english_pos::Model {
    TAGGER.get_or_init(|| {
        english_pos::Model::from_json(UPOS_JSON).expect("vendored upos.json parses")
    })
}

/// Analyze `text`, returning a JSON string (never throws across the
/// boundary; errors serialize as `{"error": ...}`).
#[wasm_bindgen]
pub fn analyze(text: &str) -> String {
    let ann = annotate_shallow(tagger(), text);

    // Shallow rules: length, complexity (grammar clause counts — real
    // here), weasel, hedge. Passive/nominalization need parser
    // weights and stay native-only.
    let length = SentenceLength::default();
    let complexity = ClauseComplexity::default();
    let rules: Vec<&dyn Rule> = vec![&length, &complexity, &Weasel, &Hedge];
    let mut findings_json = Vec::new();
    for rule in &rules {
        for f in rule.check(&ann) {
            let (line, col) = line_col(&ann.source, f.span.start);
            findings_json.push(serde_json::json!({
                "rule": f.rule,
                "line": line,
                "col": col,
                "start": f.span.start,
                "end": f.span.end,
                "message": f.message,
            }));
        }
    }
    findings_json.sort_by(|a, b| {
        (
            a["start"].as_u64().unwrap_or(0),
            a["rule"].as_str().unwrap_or(""),
        )
            .cmp(&(
                b["start"].as_u64().unwrap_or(0),
                b["rule"].as_str().unwrap_or(""),
            ))
    });

    // Structure trees straight from the native parse (same order as
    // `annotate_shallow` walks, so zip is aligned).
    let doc = english::Document::parse(text.to_string());
    let mut trees = Vec::new();
    for para in doc.paragraphs() {
        for sent in para.sentences() {
            let mut clauses = Vec::new();
            for clause in sent.clauses() {
                let mut toks = Vec::new();
                for tok in clause.tokens() {
                    let span = tok.span();
                    toks.push(serde_json::json!({
                        "s": span.start,
                        "e": span.end,
                        "k": format!("{:?}", tok.kind()),
                    }));
                }
                let span = clause.span();
                clauses.push(serde_json::json!({
                    "s": span.start,
                    "e": span.end,
                    "sub": clause.is_subordinate(),
                    "tokens": toks,
                }));
            }
            trees.push(clauses);
        }
    }

    let mut sents_json = Vec::new();
    for (i, sent) in ann.sentences.iter().enumerate() {
        let tagged: Vec<(String, english_pos::Tag)> = sent
            .pieces
            .iter()
            .cloned()
            .zip(sent.tags.iter().cloned())
            .collect();
        let chunks = english_chunk::chunk_tagged(&tagged);
        // Byte spans by sequential search: every piece is a verbatim
        // substring of the sentence (contraction parts included), and
        // hidden punctuation is skipped by searching forward.
        let sent_text = &ann.source[sent.span.clone()];
        let mut cursor = 0usize;
        let mut pieces_json = Vec::new();
        for (j, (w, t)) in tagged.iter().enumerate() {
            let kind = chunks
                .iter()
                .find(|c| c.span().contains(&j))
                .map(|c| format!("{:?}", c.kind()))
                .unwrap_or_else(|| "Other".to_string());
            let (ps, pe) = match sent_text[cursor..].find(w.as_str()) {
                Some(k) => (
                    sent.span.start + cursor + k,
                    sent.span.start + cursor + k + w.len(),
                ),
                // Unreachable in practice (pieces derive from the text);
                // stay monotonic so one miss can't cascade.
                None => (sent.span.start + cursor, sent.span.start + cursor + w.len()),
            };
            cursor = pe.saturating_sub(sent.span.start).min(sent_text.len());
            pieces_json.push(serde_json::json!({
                "w": w,
                "tag": t.upos(),
                "chunk": kind,
                "s": ps,
                "e": pe,
            }));
        }
        sents_json.push(serde_json::json!({
            "text": &ann.source[sent.span.clone()],
            "clauses": sent.clauses,
            "subords": sent.subords,
            "pieces": pieces_json,
            "tree": trees.get(i).cloned().unwrap_or_default(),
        }));
    }

    serde_json::json!({
        "sentences": sents_json,
        "findings": findings_json,
        "fidelity": "full native pipeline (grammar+scanner, beam+rules tagger, chunker, shallow lint); dep rules native-only",
    })
    .to_string()
}
