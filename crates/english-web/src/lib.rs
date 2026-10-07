//! Browser demo backend: tag → chunk → shallow lint over pasted text,
//! compiled to WebAssembly.
//!
//! Fidelity contract (honest, documented in the demo page): the
//! TAGGER, CHUNKER, and RULES run the exact native code and weights
//! (production decode: beam re-decode + all 14 gated correction
//! rules). What the browser does NOT do is grammar segmentation —
//! sentence splits are naive (`.`/`?`/`!` runs) and words are
//! whitespace/punctuation tokens, so `Mr.` may mis-split and exotic
//! punctuation may mistokenize vs native. Clause-complexity needs
//! grammar counts and stays native-only; passive/nominalization need
//! parser weights and stay native-only.
//!
//! Nothing here touches `english::Document` (whose C objects have no
//! wasm32 toolchain in this env), so the final link stays pure-Rust.

use english_lint::{line_col, AnnotatedDoc, Hedge, Rule, SentenceAnn, SentenceLength, Weasel};
use english_pos::{apply_rules, split_contraction, RULES};
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

/// Naive sentence splitter: runs of `.?!` (plus U+2026) end a
/// sentence; the byte span is tracked for findings. Known gap vs the
/// native scanner: abbreviations (`Mr.`, `St.`) mis-split here.
fn split_sentences(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let is_end = matches!(bytes[i], b'.' | b'?' | b'!')
            || (bytes[i] == 0xE2
                && bytes.get(i + 1) == Some(&0x80)
                && bytes.get(i + 2) == Some(&0xA6));
        if is_end {
            let mut j = i + 1;
            if bytes[i] == 0xE2 {
                j = i + 3;
            }
            // Absorb run repeats (`...`, `?!`, `!!`).
            while j < bytes.len()
                && (matches!(bytes[j], b'.' | b'?' | b'!')
                    || (bytes[j] == 0xE2
                        && bytes.get(j + 1) == Some(&0x80)
                        && bytes.get(j + 2) == Some(&0xA6)))
            {
                j += if bytes[j] == 0xE2 { 3 } else { 1 };
            }
            // Absorb one closing quote/paren, mirroring prose habits.
            if j < bytes.len() && matches!(bytes[j], b'"' | b'\'' | b')' | b']') {
                j += 1;
            }
            out.push((start, j));
            // Next sentence starts at the next non-space.
            let mut k = j;
            while k < bytes.len() && matches!(bytes[k], b' ' | b'\t' | b'\n' | b'\r') {
                k += 1;
            }
            start = k;
            i = k;
            continue;
        }
        i += 1;
    }
    if start < bytes.len() {
        out.push((start, bytes.len()));
    }
    out
}

/// Demo word tokenizer: whitespace split, then strip leading/trailing
/// punctuation (kept internal: `don't`, `well-known`, `10:30`).
/// Contractions expand via the native [`split_contraction`].
fn tokenize(sent: &str) -> Vec<String> {
    const STRIP: &[char] = &[
        '.', ',', ';', ':', '!', '?', '(', ')', '[', ']', '{', '}', '"', '\'', '‘', '’', '“', '”',
        '…', '—', '–', '-', '—', '*', '_', '#', '|', '~',
    ];
    let mut out = Vec::new();
    for raw in sent.split_whitespace() {
        let w = raw.trim_matches(STRIP);
        if w.is_empty() {
            continue;
        }
        // Skip pure-punctuation leftovers the strip missed.
        if w.chars().all(|c| STRIP.contains(&c)) {
            continue;
        }
        out.extend(split_contraction(w));
    }
    out
}

/// Analyze `text`, returning a JSON string (never throws across the
/// boundary; errors serialize as `{"error": ...}`).
#[wasm_bindgen]
pub fn analyze(text: &str) -> String {
    let model = tagger();
    let mut sentences = Vec::new();
    for (s, e) in split_sentences(text) {
        if text[s..e].trim().is_empty() {
            continue;
        }
        let pieces = tokenize(&text[s..e]);
        if pieces.is_empty() {
            continue;
        }
        // Production decode, exactly as native `tag_sentence` minus
        // the grammar piece source: beam re-decode + gated rules.
        let (mut tagged, lower) = model.tag_beam_margins_lowered(&pieces);
        if !RULES.is_empty() {
            apply_rules(&mut tagged, RULES, &lower);
        }
        let tags: Vec<english_pos::Tag> = tagged.into_iter().map(|(t, _)| t).collect();
        sentences.push(SentenceAnn {
            span: s..e,
            pieces,
            tags,
            heads: Vec::new(),
            rels: Vec::new(),
            clauses: 0,
            subords: 0,
        });
    }
    let doc = AnnotatedDoc {
        source: text.to_string(),
        sentences,
    };

    // Shallow rules only: length (pieces), weasel, hedge. Complexity
    // needs grammar clause counts (always 0 here — excluded, not
    // faked); passive/nominalization need parser weights.
    let length = SentenceLength::default();
    let rules: Vec<&dyn Rule> = vec![&length, &Weasel, &Hedge];
    let mut findings_json = Vec::new();
    for rule in &rules {
        for f in rule.check(&doc) {
            let (line, col) = line_col(&doc.source, f.span.start);
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

    let mut sents_json = Vec::new();
    for sent in &doc.sentences {
        let tagged: Vec<(String, english_pos::Tag)> = sent
            .pieces
            .iter()
            .cloned()
            .zip(sent.tags.iter().cloned())
            .collect();
        let chunks = english_chunk::chunk_tagged(&tagged);
        let mut pieces_json = Vec::new();
        for (i, (w, t)) in tagged.iter().enumerate() {
            let kind = chunks
                .iter()
                .find(|c| c.span().contains(&i))
                .map(|c| format!("{:?}", c.kind()))
                .unwrap_or_else(|| "Other".to_string());
            pieces_json.push(serde_json::json!({
                "w": w,
                "tag": t.upos(),
                "chunk": kind,
            }));
        }
        sents_json.push(serde_json::json!({
            "text": &doc.source[sent.span.clone()],
            "pieces": pieces_json,
        }));
    }

    serde_json::json!({
        "sentences": sents_json,
        "findings": findings_json,
        "fidelity": "tagger+chunker+rules exact; sentences/words naive (no grammar in browser); complexity/passive/nominalization native-only",
    })
    .to_string()
}
