//! Semantic verification pass: parse files, POS-tag every sentence,
//! and report sentences that fail coherence checks.
//!
//! `audit` (in the `english` crate) measures grammar *coverage*
//! (ERROR/MISSING nodes, snapshots). This asks the next question: is
//! the covered text *coherent*? Checks per sentence:
//!
//! - `error`: ERROR/MISSING nodes in the sentence subtree.
//!   Transcription markup (`_`/`*`, out of grammar scope like in the
//!   audit) reports separately as `error-transcription`.
//! - `no-predicate`: no VERB/AUX tag and no fragment excuse
//!   (single token, only INTJ/PUNCT, ≤ 4 pieces, or the sentence is
//!   its whole paragraph — chapter titles and stage directions like
//!   `Enter Ahab; to Him, Stubb.` live there). Longer verbless runs
//!   inside multi-sentence paragraphs are suspicious.
//! - `joiner`: a leading `;` or `:` (genuinely broken — nothing
//!   before it). Leading conjunctions (`And…`), trailing em-dashes
//!   (`…?—` handoff), `;—`/`:—` elaborations, and repeated
//!   intensifiers (`so, so, so`) are all accepted prose or
//!   corpus-blessed grammar readings, so doubled/dangling-joiner
//!   rules were tried and dropped as noise.
//! - `margin`: minimum tag margin over the sentence's pieces (best
//!   minus runner-up score). Reported, never failed: the lowest-margin
//!   sentences are listed for review since ambiguity is legal.
//!
//! Usage: `cargo run -p english-pos --example verify -- [files...]`
//! With no files, verifies `examples/*.txt` (skipping snapshots).
//! Gutenberg front matter (before `CHAPTER 1. Loomings.`) is sliced
//! off as transcription, matching the audit/bench convention.
//! Exit status is 0; this is an analysis tool, not a gate — the day it
//! grows teeth is the day the categories above get false-positive
//! rates measured first.

use std::fs;
use std::path::{Path, PathBuf};

use english::TokenKind;
use english_pos::{Model, Tag};

struct Finding {
    para: usize,
    sent: usize,
    kind: &'static str,
    text: String,
}

fn check_sentence(
    para: usize,
    sent_idx: usize,
    para_has_predicate: bool,
    sent: &english::Sentence,
    tagged: &[(Tag, f32)],
    findings: &mut Vec<Finding>,
    margins: &mut Vec<(f32, String)>,
) {
    let text = sent.text().replace('\n', "\\n");
    if sent.has_error() {
        // Gutenberg transcription markup is out of grammar scope
        // (same bucketing as the audit).
        let kind = if text.contains('_') || text.contains('*') {
            "error-transcription"
        } else {
            "error"
        };
        findings.push(Finding {
            para,
            sent: sent_idx,
            kind,
            text: text.clone(),
        });
    }

    let tokens = sent.tokens();
    let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind()).collect();
    // Only leading `;`/`:` is genuinely broken. Everything else that
    // looks dangling (leading conjunctions, trailing em-dash handoffs,
    // `;—` elaborations, repeated `so`) is accepted prose.
    if matches!(kinds.first(), Some(TokenKind::Semicolon | TokenKind::Colon)) {
        findings.push(Finding {
            para,
            sent: sent_idx,
            kind: "joiner",
            text: text.clone(),
        });
    }

    let pieces = english_pos::sentence_pieces(sent);
    assert_eq!(
        pieces.len(),
        tagged.len(),
        "tag/piece count mismatch (contraction drift?)"
    );
    let min_margin = tagged.iter().map(|(_, m)| *m).fold(f32::INFINITY, f32::min);
    margins.push((min_margin, format!("p{para}s{sent_idx}: {text}")));

    let tags: Vec<Tag> = tagged.iter().map(|(t, _)| *t).collect();
    let has_predicate = tags.iter().any(|t| matches!(t, Tag::Verb | Tag::Aux));
    if !has_predicate {
        // Excuses, weakest first: single token, pure interjection,
        // short (≤ 4 pieces), or in a paragraph with no predicate
        // anywhere (chapter-title blocks like `CHAPTER 42. The
        // Whiteness of the Whale.`, stage directions). A verbless
        // multi-sentence paragraph would need error nodes to be a
        // cascading failure — and those report as `error` anyway.
        let excused = tags.len() <= 1 || tags.iter().all(|t| matches!(t, Tag::Intj | Tag::Punct));
        let short = tags.len() <= 4;
        findings.push(Finding {
            para,
            sent: sent_idx,
            kind: if excused || short {
                "fragment"
            } else if !para_has_predicate {
                "title"
            } else {
                "no-predicate"
            },
            text,
        });
    }
}

fn verify(path: &Path, model: &Model) {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    // Scale-corpus convention (shared with bench): Gutenberg front
    // matter is transcription, out of scope — measure from CHAPTER 1.
    let text = match text.find("CHAPTER 1. Loomings.") {
        Some(i) => &text[i..],
        None => &text,
    };
    let doc = english::Document::parse(text);
    let mut findings = Vec::new();
    let mut margins: Vec<(f32, String)> = Vec::new();
    let mut n_sent = 0;
    for (pi, para) in doc.paragraphs().iter().enumerate() {
        let sents = para.sentences();
        // Tag once per sentence: paragraph predicate presence (title
        // blocks like `CHAPTER 42. The Whiteness of the Whale.` have
        // none anywhere) and per-sentence checks share the result.
        let tagged: Vec<Vec<(Tag, f32)>> = sents
            .iter()
            .map(|s| model.tag_margins(&english_pos::sentence_pieces(s)))
            .collect();
        let para_has_predicate = tagged
            .iter()
            .flatten()
            .any(|(t, _)| matches!(t, Tag::Verb | Tag::Aux));
        for ((si, sent), sent_tags) in sents.iter().enumerate().zip(&tagged) {
            n_sent += 1;
            check_sentence(
                pi,
                si,
                para_has_predicate,
                sent,
                sent_tags,
                &mut findings,
                &mut margins,
            );
        }
    }
    let mut counts: std::collections::BTreeMap<&str, usize> = Default::default();
    for f in &findings {
        *counts.entry(f.kind).or_default() += 1;
    }
    println!(
        "{}: {} sentences, findings: {counts:?}",
        path.display(),
        n_sent
    );
    for f in findings
        .iter()
        .filter(|f| !matches!(f.kind, "fragment" | "title" | "error-transcription"))
    {
        println!("  [{}] p{}s{}: {:?}", f.kind, f.para, f.sent, f.text);
    }
    for info in ["fragment", "title", "error-transcription"] {
        if let Some(n) = counts.get(info) {
            println!("  ({n} {info} excused/bucketed)");
        }
    }
    margins.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    println!("  lowest margins:");
    for (m, s) in margins.iter().take(5) {
        println!("    {m:.1} {s}");
    }
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let files: Vec<PathBuf> = if args.is_empty() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("examples");
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .expect("failed to read examples dir")
            .map(|e| e.expect("bad entry").path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
            .filter(|p| !p.to_string_lossy().ends_with(".parse.txt"))
            .collect();
        entries.sort();
        entries
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");
    for path in &files {
        verify(path, &model);
    }
}
