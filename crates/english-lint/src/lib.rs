//! Syntax-aware prose linting: Vale-class output (`path:line:col [rule]
//! message`) over grammar-backed rules the regex linters cannot express.
//!
//! The pipeline runs once per document (parse → tag → beam4 parse →
//! label); every [`Rule`] reads the shared [`AnnotatedDoc`]. Pilot rule:
//! [`Passive`] (finite be/get-passives via `nsubj:pass`/`aux:pass`).
//! Sentence-level spans in v1 (piece→byte alignment for word-level
//! spans is queued follow-up, not this pilot).

use std::fs;
use std::ops::Range;
use std::path::Path;

use english_dep::{LabelModel, Model as DepModel};
use english_pos::{Model as PosModel, Tag, tag_sentence};

/// One lint finding: rule id, byte span in the document source, message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub span: Range<usize>,
    pub message: String,
}

/// A lint rule over fully-annotated documents.
pub trait Rule {
    /// Stable dotted id (`syntax.passive`), printed in CLI output.
    fn id(&self) -> &'static str;
    /// Findings in document order (span-start order enforced by `lint`).
    fn check(&self, doc: &AnnotatedDoc) -> Vec<Finding>;
}

/// One sentence with every pipeline annotation aligned by piece index
/// (index 0 dummy, mirroring the dep crate's 1-based convention).
#[derive(Debug, Clone)]
pub struct SentenceAnn {
    /// Byte span of the sentence in the document source.
    pub span: Range<usize>,
    /// Surface pieces (tagger output; contractions split).
    pub pieces: Vec<String>,
    /// UPOS tag per piece.
    pub tags: Vec<Tag>,
    /// Head index per 1-based piece (`0` = root).
    pub heads: Vec<usize>,
    /// UD relation per 1-based piece (`""` = stranded).
    pub rels: Vec<String>,
}

/// A document with pipeline annotations, shared by all rules.
#[derive(Debug, Clone)]
pub struct AnnotatedDoc {
    pub source: String,
    pub sentences: Vec<SentenceAnn>,
}

/// The three trained models (tagger vendored; parser + labeler are
/// Tier-1 lazy assets — absent on a fresh clone, see CLI error).
pub struct Models {
    pub tagger: PosModel,
    pub parser: DepModel,
    pub labeler: LabelModel,
}

impl Models {
    /// Load `upos.json` + `dep.json` + `labels.json` from `dir`
    /// (the weights directories of `english-pos` / `english-dep`).
    /// Fails loudly with the missing path (never a silent fallback).

    /// Convenience: load from the workspace weights directories.
    /// Only valid inside a checkout with trained weights present.
    pub fn load_workspace() -> Result<Self, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pos = root.join("..").join("english-pos").join("weights");
        // Parser + labeler live together (one Tier-1 pair); the tagger
        // path differs, so load pairwise and combine.
        let tagger = PosModel::from_json(
            &fs::read_to_string(pos.join("upos.json")).map_err(|e| format!("lint needs upos.json: {e}"))?,
        )
        .map_err(|e| format!("bad upos.json: {e}"))?;
        let depdir = root.join("..").join("english-dep").join("weights");
        let read = |name: &str| {
            fs::read_to_string(depdir.join(name))
                .map_err(|e| format!("lint needs trained weights: {} ({e})", depdir.join(name).display()))
        };
        let parser = DepModel::from_json(&read("dep.json")?).map_err(|e| format!("bad dep.json: {e}"))?;
        let labeler =
            LabelModel::from_json(&read("labels.json")?).map_err(|e| format!("bad labels.json: {e}"))?;
        Ok(Models { tagger, labeler, parser })
    }
}

/// Run the full pipeline once: parse → tag → beam4 parse → label.
/// Unparseable sentences (ERROR subtrees) are still annotated
/// (recovery is the grammar's contract); stranded pieces carry
/// `usize::MAX` heads and `""` rels.
pub fn annotate(models: &Models, source: &str) -> AnnotatedDoc {
    let doc = english::Document::parse(source.to_string());
    let mut sentences = Vec::new();
    // Walk paragraphs in order (document order = span order).
    for para in doc.paragraphs() {
        for sent in para.sentences() {
            let span = sent.span();
            let tagged = tag_sentence(&models.tagger, &sent);
            let pieces: Vec<String> = tagged.iter().map(|(w, _)| w.clone()).collect();
            let tags: Vec<Tag> = tagged.iter().map(|(_, t)| *t).collect();
            let upos: Vec<String> = tags.iter().map(|t| t.upos().to_string()).collect();
            let (heads, _) = models.parser.parse_beam(&pieces, &upos, 4);
            let rels = models.labeler.predict(&pieces, &upos, &heads);
            sentences.push(SentenceAnn { span, pieces, tags, heads, rels });
        }
    }
    AnnotatedDoc { source: source.to_string(), sentences }
}

/// Run every rule over one annotated document; findings sorted by
/// span start (document order), ties broken by rule id.
pub fn lint(models: &Models, source: &str, rules: &[&dyn Rule]) -> Vec<Finding> {
    let doc = annotate(models, source);
    let mut out = Vec::new();
    for rule in rules {
        out.extend(rule.check(&doc));
    }
    out.sort_by(|a, b| a.span.start.cmp(&b.span.start).then(a.rule.cmp(b.rule)));
    out
}

/// 1-based `(line, column)` of a byte offset (column counts chars,
/// Vale-compatible).
pub fn line_col(source: &str, byte: usize) -> (usize, usize) {
    let byte = byte.min(source.len());
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i >= byte {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Finite be/get-passives via `nsubj:pass` / `csubj:pass` /
/// `aux:pass` (one finding per sentence, naming the passive verb).
/// Scope v1: finite passives with an overt auxiliary. Out of scope
/// (documented, not bugs): reduced relatives (`the man killed` —
/// gapped subject, no aux to key on; needs gap detection, same
/// lesson as relativizer-`that`), adjectival participles (`was
/// tired` — EWT tags ADJ, no pass relation fires), `get`-passives
/// where EWT reads plain `aux` (labeler follows gold).
pub struct Passive;

impl Rule for Passive {
    fn id(&self) -> &'static str {
        "syntax.passive"
    }

    fn check(&self, doc: &AnnotatedDoc) -> Vec<Finding> {
        let mut out = Vec::new();
        for sent in &doc.sentences {
            // The passive verb: head of an aux:pass dependent, else the
            // nsubj:pass dependent's head, else the first pass dependent.
            let mut aux_text: Option<&str> = None;
            let mut verb_idx: Option<usize> = None;
            // Token indices here are 1-based (dep-crate convention);
            // pieces are 0-based — every access goes through piece().
            let piece = |k: usize| -> Option<&str> {
                (k >= 1).then(|| sent.pieces.get(k - 1)).flatten().map(|s| s.as_str())
            };
            for (k, rel) in sent.rels.iter().enumerate().skip(1) {
                if rel == "aux:pass" {
                    aux_text = piece(k);
                }
                if (rel == "nsubj:pass" || rel == "csubj:pass") && k < sent.heads.len() {
                    let h = sent.heads[k];
                    if piece(h).is_some() {
                        verb_idx = Some(h);
                    }
                }
            }
            // aux:pass without a subject-pass dependent still counts
            // (impersonal/pro-dropped shapes); name the aux's head.
            if verb_idx.is_none() {
                for (k, rel) in sent.rels.iter().enumerate().skip(1) {
                    if rel == "aux:pass" && k < sent.heads.len() {
                        let h = sent.heads[k];
                        if piece(h).is_some() {
                            verb_idx = Some(h);
                        }
                        break;
                    }
                }
            }
            let fired = sent.rels.iter().skip(1).any(|r| {
                r == "nsubj:pass" || r == "csubj:pass" || r == "aux:pass"
            });
            if !fired {
                continue;
            }
            let what = match (aux_text, verb_idx.and_then(piece)) {
                (Some(a), Some(v)) => format!("\"{a} {v}\""),
                (None, Some(v)) => format!("\"{v}\""),
                _ => "passive construction".to_string(),
            };
            out.push(Finding {
                rule: self.id(),
                span: sent.span.clone(),
                message: format!("{what} is passive — prefer active voice where the actor matters"),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(pieces: &[&str], heads: &[usize], rels: &[&str]) -> SentenceAnn {
        assert_eq!(pieces.len() + 1, heads.len());
        assert_eq!(pieces.len() + 1, rels.len());
        SentenceAnn {
            span: 0..0,
            pieces: pieces.iter().map(|s| s.to_string()).collect(),
            tags: vec![Tag::X; pieces.len()],
            heads: heads.to_vec(),
            rels: rels.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn doc(sents: Vec<SentenceAnn>) -> AnnotatedDoc {
        AnnotatedDoc { source: String::new(), sentences: sents }
    }

    #[test]
    fn passive_fires_on_pass_relations() {
        // "was eaten": was/aux:pass, mouse/nsubj:pass headed by eaten.
        let d = doc(vec![sent(
            &["cheese", "was", "eaten"],
            &[0, 3, 3, 0],
            &["", "nsubj:pass", "aux:pass", "root"],
        )]);
        let got = Passive.check(&d);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].rule, "syntax.passive");
        assert!(got[0].message.contains("was eaten"), "message names the verb: {}", got[0].message);
    }

    #[test]
    fn passive_silent_on_active_and_adjectival() {
        // Active transitive: no pass relations anywhere.
        let d = doc(vec![sent(
            &["mouse", "ate", "cheese"],
            &[0, 2, 0, 2],
            &["", "nsubj", "root", "obj"],
        )]);
        assert!(Passive.check(&d).is_empty());
        // Adjectival participle ("was tired"): cop + ADJ, no pass rel.
        let d = doc(vec![sent(
            &["mouse", "was", "tired"],
            &[0, 3, 3, 0],
            &["", "nsubj", "cop", "root"],
        )]);
        assert!(Passive.check(&d).is_empty());
    }

    #[test]
    fn line_col_counts_chars_from_one() {
        assert_eq!(line_col("ab\ncd", 0), (1, 1));
        assert_eq!(line_col("ab\ncd", 3), (2, 1));
        assert_eq!(line_col("ab\ncd", 4), (2, 2));
    }
}
