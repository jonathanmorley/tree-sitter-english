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
            &fs::read_to_string(pos.join("upos.json"))
                .map_err(|e| format!("lint needs upos.json: {e}"))?,
        )
        .map_err(|e| format!("bad upos.json: {e}"))?;
        let depdir = root.join("..").join("english-dep").join("weights");
        let read = |name: &str| {
            fs::read_to_string(depdir.join(name)).map_err(|e| {
                format!(
                    "lint needs trained weights: {} ({e})",
                    depdir.join(name).display()
                )
            })
        };
        let parser =
            DepModel::from_json(&read("dep.json")?).map_err(|e| format!("bad dep.json: {e}"))?;
        let labeler = LabelModel::from_json(&read("labels.json")?)
            .map_err(|e| format!("bad labels.json: {e}"))?;
        Ok(Models {
            tagger,
            labeler,
            parser,
        })
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
            sentences.push(SentenceAnn {
                span,
                pieces,
                tags,
                heads,
                rels,
            });
        }
    }
    AnnotatedDoc {
        source: source.to_string(),
        sentences,
    }
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
                (k >= 1)
                    .then(|| sent.pieces.get(k - 1))
                    .flatten()
                    .map(|s| s.as_str())
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
            let fired = sent
                .rels
                .iter()
                .skip(1)
                .any(|r| r == "nsubj:pass" || r == "csubj:pass" || r == "aux:pass");
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

/// Light-verb nominalizations (`conduct an investigation`): a closed
/// list of light verbs governing a `-tion`/`-ment`/`-ance`/`-ence`
/// noun through `obj`/`obl`. Names the pair; no auto-rewrite v1
/// (derivational mapping is a lexicon, not a rule).
/// Scope: light-verb government ONLY. Bare nominalizations (`the
/// investigation continues`) are often fine prose and stay silent
/// by design. Known heuristic edge: suffix matching fires on
/// non-deverbial `-ment` (`take a moment`) — counted honestly in
/// eval, not special-cased.
pub struct Nominalization;

/// Closed light-verb surface forms (explicit table, no stemmer —
/// audit every addition; each form is a separate committed choice).
const LIGHT_VERBS: &[&str] = &[
    "make",
    "makes",
    "made",
    "making",
    "take",
    "takes",
    "took",
    "taken",
    "taking",
    "have",
    "has",
    "had",
    "having",
    "give",
    "gives",
    "gave",
    "given",
    "giving",
    "do",
    "does",
    "did",
    "done",
    "doing",
    "get",
    "gets",
    "got",
    "gotten",
    "getting",
    "conduct",
    "conducts",
    "conducted",
    "conducting",
    "perform",
    "performs",
    "performed",
    "performing",
];

/// Deverbial-noun suffixes (lowercase piece-end match, stem ≥ 2 chars
/// so `dance`/`chance`-shaped words stay out). Latin `-tion`/`-sion`/
/// `-ment`/`-ance`/`-ence` plus Greek `-sis` (`analysis`, `thesis`
/// — closed class, near-zero FP surface with light-verb government).
/// Deliberately NOT here: `-ing` gerunds (`give warning` stays
/// silent — the `get going`/`get moving` inceptive class would cost
/// more than the nominal readings gain; needs its own measurement)
/// and `-age`/`-edge` (`damage`, `knowledge` — mixed deverbial
/// density, same deal).
const NOMINAL_SUFFIXES: &[&str] = &["tion", "sion", "ment", "ance", "ence", "sis"];

fn is_nominalization(word: &str) -> bool {
    let w = word.to_lowercase();
    // Plurals by stem (`arrangements` ends in `ments`, not `ment`):
    // match the full word (Greek `-sis`: `analysis`) or the
    // single-`s`-stripped stem. Harmless on non-plurals (`glass` →
    // `glas` matches no suffix); words genuinely ending in `ss`
    // lose one `s` and still match nothing new.
    let stem = w.strip_suffix('s').unwrap_or(&w);
    NOMINAL_SUFFIXES.iter().any(|s| {
        (w.len() > s.len() + 1 && w.ends_with(s)) || (stem.len() > s.len() + 1 && stem.ends_with(s))
    })
}

impl Rule for Nominalization {
    fn id(&self) -> &'static str {
        "syntax.nominalization"
    }

    fn check(&self, doc: &AnnotatedDoc) -> Vec<Finding> {
        let mut out = Vec::new();
        for sent in &doc.sentences {
            // One finding per sentence (first offending pair in order).
            let mut hit: Option<(String, String)> = None;
            for (k, rel) in sent.rels.iter().enumerate().skip(1) {
                if rel != "obj" && !rel.starts_with("obl") {
                    continue;
                }
                if k > sent.pieces.len() {
                    continue;
                }
                let dep = &sent.pieces[k - 1];
                if !is_nominalization(dep) {
                    continue;
                }
                if k >= sent.heads.len() {
                    continue;
                }
                let h = sent.heads[k];
                if h == 0 || h > sent.pieces.len() {
                    continue;
                }
                // Governor must read as a verb (kills noun-`make`
                // shapes like brand names; cascade cost accepted like
                // everything else in this crate).
                if sent.tags.get(h - 1) != Some(&Tag::Verb) {
                    continue;
                }
                let gov = sent.pieces[h - 1].to_lowercase();
                if LIGHT_VERBS.contains(&gov.as_str()) {
                    hit = Some((sent.pieces[h - 1].clone(), dep.clone()));
                    break;
                }
            }
            if let Some((verb, noun)) = hit {
                out.push(Finding {
                    rule: self.id(),
                    span: sent.span.clone(),
                    message: format!(
                        "nominalization with light verb: \"{verb} {noun}\" — prefer the direct verb"
                    ),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(pieces: &[&str], tags: &[Tag], heads: &[usize], rels: &[&str]) -> SentenceAnn {
        assert_eq!(pieces.len(), tags.len());
        assert_eq!(pieces.len() + 1, heads.len());
        assert_eq!(pieces.len() + 1, rels.len());
        SentenceAnn {
            span: 0..0,
            pieces: pieces.iter().map(|s| s.to_string()).collect(),
            tags: tags.to_vec(),
            heads: heads.to_vec(),
            rels: rels.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn doc(sents: Vec<SentenceAnn>) -> AnnotatedDoc {
        AnnotatedDoc {
            source: String::new(),
            sentences: sents,
        }
    }

    #[test]
    fn passive_fires_on_pass_relations() {
        // "was eaten": was/aux:pass, mouse/nsubj:pass headed by eaten.
        let d = doc(vec![sent(
            &["cheese", "was", "eaten"],
            &[Tag::Noun, Tag::Aux, Tag::Verb],
            &[0, 3, 3, 0],
            &["", "nsubj:pass", "aux:pass", "root"],
        )]);
        let got = Passive.check(&d);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].rule, "syntax.passive");
        assert!(
            got[0].message.contains("was eaten"),
            "message names the verb: {}",
            got[0].message
        );
    }

    #[test]
    fn passive_silent_on_active_and_adjectival() {
        // Active transitive: no pass relations anywhere.
        let d = doc(vec![sent(
            &["mouse", "ate", "cheese"],
            &[Tag::Noun, Tag::Verb, Tag::Noun],
            &[0, 2, 0, 2],
            &["", "nsubj", "root", "obj"],
        )]);
        assert!(Passive.check(&d).is_empty());
        // Adjectival participle ("was tired"): cop + ADJ, no pass rel.
        let d = doc(vec![sent(
            &["mouse", "was", "tired"],
            &[Tag::Noun, Tag::Aux, Tag::Adj],
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

    fn nsent(pieces: &[&str], tags: &[Tag], heads: &[usize], rels: &[&str]) -> SentenceAnn {
        sent(pieces, tags, heads, rels)
    }

    #[test]
    fn nominalization_fires_on_light_verb_government() {
        // "conducted an investigation": conducted/VERB -obj-> investigation.
        let d = doc(vec![nsent(
            &["they", "conducted", "an", "investigation"],
            &[Tag::Pron, Tag::Verb, Tag::Det, Tag::Noun],
            &[0, 2, 0, 4, 2],
            &["", "nsubj", "root", "det", "obj"],
        )]);
        let got = Nominalization.check(&d);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].rule, "syntax.nominalization");
        assert!(
            got[0].message.contains("conducted investigation"),
            "message names the pair: {}",
            got[0].message
        );
    }

    #[test]
    fn nominalization_silent_without_light_government() {
        // Concrete object: light verb but no nominal suffix.
        let d = doc(vec![nsent(
            &["they", "make", "dinner"],
            &[Tag::Pron, Tag::Verb, Tag::Noun],
            &[0, 2, 0, 2],
            &["", "nsubj", "root", "obj"],
        )]);
        assert!(Nominalization.check(&d).is_empty());
        // Bare nominalization, no light verb: silent by design.
        let d = doc(vec![nsent(
            &["the", "investigation", "continues"],
            &[Tag::Det, Tag::Noun, Tag::Verb],
            &[0, 2, 2, 0],
            &["", "det", "nsubj", "root"],
        )]);
        assert!(Nominalization.check(&d).is_empty());
        // Suffix noun under a non-light verb: silent.
        let d = doc(vec![nsent(
            &["they", "announced", "an", "investigation"],
            &[Tag::Pron, Tag::Verb, Tag::Det, Tag::Noun],
            &[0, 2, 0, 4, 2],
            &["", "nsubj", "root", "det", "obj"],
        )]);
        assert!(Nominalization.check(&d).is_empty());
    }

    #[test]
    fn nominalization_handles_plurals_and_greek_sis() {
        // Plurals match by stem (`arrangements` ends in `ments`).
        let d = doc(vec![nsent(
            &["they", "made", "arrangements"],
            &[Tag::Pron, Tag::Verb, Tag::Noun],
            &[0, 2, 0, 2],
            &["", "nsubj", "root", "obj"],
        )]);
        assert_eq!(Nominalization.check(&d).len(), 1);
        // Greek `-sis` nominalizations (`performed an analysis`).
        let d = doc(vec![nsent(
            &["they", "performed", "an", "analysis"],
            &[Tag::Pron, Tag::Verb, Tag::Det, Tag::Noun],
            &[0, 2, 0, 4, 2],
            &["", "nsubj", "root", "det", "obj"],
        )]);
        assert_eq!(Nominalization.check(&d).len(), 1);
    }
}
