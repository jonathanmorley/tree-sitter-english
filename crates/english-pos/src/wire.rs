//! Wiring between the `english` parse tree and POS tagging.
//!
//! The grammar segments prose into flat clauses of unclassified words;
//! this module converts those clauses into the UD-style token stream the
//! perceptron was trained on, then tags it. Tagging stays a post-parse
//! pass: the grammar never sees word classes.
//!
//! Two alignments are handled here:
//!
//! - Closed-class visibility: [`english::Clause::words`] drops
//!   subordinators (and sentence joiners live outside clauses entirely),
//!   so this module reads [`english::Sentence::tokens`] /
//!   [`english::Clause::tokens`] instead — no token is silently dropped.
//! - Contractions: UD splits `don't` into `do` + `n't` while the grammar
//!   keeps it whole. [`split_contraction`] expands whole words into UD
//!   pieces (curly `’` normalized to ASCII) for model input.
//!
//! Hidden punctuation (commas, sentence-final `.`/`?`/`!`, parens
//! themselves) has no named node in the grammar and is not yielded by
//! `tokens()`; pieces therefore exclude it. Model weights are unchanged
//! (dev 90.35% / test 90.53% on UD EWT; ~80% on the Moby-Dick sample),
//! so accuracy on word content is preserved.

use crate::{Model, Tag};

/// Split a whole grammar word into UD-style pieces.
///
/// Splits on ASCII `'` and curly `’` (normalized to ASCII in the
/// output), keeping the apostrophe at the start of the following piece:
/// `ship’s` → `ship` + `'s`. The `n't` suffix keeps its `n` (`don't` →
/// `do` + `n't`, matching UD EWT). A leading apostrophe (`'em`) stays
/// whole; a trailing one (`dogs'`) yields a lone `"'"` piece. Words
/// without apostrophes return unchanged.
pub fn split_contraction(word: &str) -> Vec<String> {
    if !word.contains('\'') && !word.contains('’') {
        return vec![word.to_string()];
    }
    // `n't` keeps its `n`: don't → do + n't (not don + 't).
    let lower = word.to_lowercase();
    let is_nt = lower.ends_with("n't") || lower.ends_with("n’t");
    if is_nt {
        let chars: Vec<char> = word.chars().collect();
        if chars.len() > 3 {
            let stem: String = chars[..chars.len() - 3].iter().collect();
            if !stem.is_empty() {
                return vec![stem, "n't".to_string()];
            }
        }
        return vec![word.to_string()];
    }
    let mut pieces = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if ch == '\'' || ch == '’' {
            if !current.is_empty() {
                pieces.push(std::mem::take(&mut current));
            }
            current.push('\'');
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        // A lone leading apostrophe means the whole word was `'em`-style:
        // if nothing was pushed yet, the single piece is the word itself.
        pieces.push(current);
    } else if word.ends_with('\'') || word.ends_with('’') {
        pieces.push("'".to_string());
    }
    if pieces.is_empty() {
        vec![word.to_string()]
    } else {
        pieces
    }
}

/// Expand one grammar token into UD pieces for model input.
///
/// Only `Word`-kind tokens containing apostrophes expand; everything
/// else (`dotted`, `number`, conjunctions, subordinators, punctuation)
/// passes through whole.
pub fn token_pieces(token: &english::Token) -> Vec<String> {
    if token.kind() == english::TokenKind::Word {
        split_contraction(token.text())
    } else {
        vec![token.text().to_string()]
    }
}

/// UD-style pieces for a parsed clause, in order.
pub fn clause_pieces(clause: &english::Clause) -> Vec<String> {
    collect_pieces(&clause.tokens())
}

/// UD-style pieces for a parsed sentence, in order.
///
/// Includes clause joiners (`and`, `;`, `:`, `—`, …) via
/// [`english::Sentence::tokens`]; hidden punctuation (commas,
/// sentence-final marks) is excluded.
///
/// Abbreviation dots merge into the preceding word (`Mr` + `.` →
/// `Mr.`), matching UD tokenization, which keeps them attached
/// (EWT `Mr.` is one token). Sentence-final dots need no merge: the
/// grammar hides them, and UD splits those off as PUNCT (out of
/// scope — pieces exclude all terminal punctuation by design).
pub fn sentence_pieces(sentence: &english::Sentence) -> Vec<String> {
    collect_pieces(&sentence.tokens())
}

/// Expand tokens to pieces, merging in-sentence `period` into a
/// byte-adjacent preceding `Word`/`Dotted` token.
fn collect_pieces(tokens: &[english::Token]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // End byte + wordishness of the previous token, for dot merging.
    let mut prev: Option<(usize, bool)> = None;
    for tok in tokens {
        let span = tok.span();
        if tok.kind() == english::TokenKind::Period
            && matches!(prev, Some((end, true)) if end == span.start)
            && let Some(last) = out.last_mut()
        {
            last.push_str(tok.text());
            prev = Some((span.end, false));
            continue;
        }
        let wordish = matches!(
            tok.kind(),
            english::TokenKind::Word | english::TokenKind::Dotted
        );
        out.extend(token_pieces(tok));
        prev = Some((span.end, wordish));
    }
    out
}

/// Tag a parsed clause: `(surface piece, tag)` pairs in order.
///
/// Contraction pieces expand (see [`split_contraction`]), so the output
/// may hold more items than the clause has tokens.
pub fn tag_clause(model: &Model, clause: &english::Clause) -> Vec<(String, Tag)> {
    let pieces = clause_pieces(clause);
    model
        .tag(&pieces)
        .into_iter()
        .zip(pieces)
        .map(|(tag, text)| (text, tag))
        .collect()
}

/// Tag a parsed sentence: `(surface piece, tag)` pairs in order.
///
/// Each sentence is decoded independently (tag history resets at the
/// boundary), matching training on UD sentences.
pub fn tag_sentence(model: &Model, sentence: &english::Sentence) -> Vec<(String, Tag)> {
    let pieces = sentence_pieces(sentence);
    model
        .tag(&pieces)
        .into_iter()
        .zip(pieces)
        .map(|(tag, text)| (text, tag))
        .collect()
}

/// Tag a whole document, one entry per sentence in document order.
///
/// Each sentence is decoded independently; see [`tag_sentence`].
pub fn tag_document(model: &Model, doc: &english::Document) -> Vec<Vec<(String, Tag)>> {
    doc.paragraphs()
        .iter()
        .flat_map(|para| para.sentences())
        .map(|sent| tag_sentence(model, &sent))
        .collect()
}

/// Cache of sentence tags across re-parses.
///
/// Tagging is a pure function of the sentence's piece sequence (decode
/// history resets at the sentence boundary), so identical sentences tag
/// identically wherever they appear. After an edit, only sentences whose
/// text changed miss the cache: pair with `Document::update` (incremental
/// re-parse) and a keystroke retag costs changed-sentences × ~16 µs
/// instead of a full 170 ms pass over Moby-Dick.
///
/// Memory is bounded by unique sentence text (~1 MB per 10k Moby-scale
/// sentences); call [`TagCache::clear`] if the source distribution
/// shifts wholesale (e.g. opening a different book).
#[derive(Debug, Default)]
pub struct TagCache {
    map: std::collections::HashMap<String, CachedSentence>,
    hits: usize,
    misses: usize,
}

/// One cached sentence: UD pieces plus their tags.
///
/// Pieces are stored (not re-extracted) so a cache hit skips the tree
/// walk, contraction splitting, and key allocation — only a clone of
/// already-owned strings remains.
#[derive(Debug, Clone)]
struct CachedSentence {
    pieces: Vec<String>,
    tags: Vec<Tag>,
}

impl TagCache {
    /// Empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cache hits so far (sentences retagged for free).
    pub fn hits(&self) -> usize {
        self.hits
    }

    /// Cache misses so far (sentences actually run through the model).
    pub fn misses(&self) -> usize {
        self.misses
    }

    /// Drop all entries and reset counters.
    pub fn clear(&mut self) {
        self.map.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Tag a sentence, reusing a cached result when its text was seen.
    ///
    /// Returns `(surface piece, tag)` pairs like [`tag_sentence`]. The
    /// key is the sentence text (not pieces): pieces derive from it
    /// deterministically, so equal text means equal tags. Hits skip
    /// piece extraction entirely (stored pieces are cloned).
    pub fn tag_sentence(
        &mut self,
        model: &Model,
        sentence: &english::Sentence,
    ) -> Vec<(String, Tag)> {
        let key = sentence.text().to_string();
        if let Some(hit) = self.map.get(&key) {
            self.hits += 1;
            return hit
                .pieces
                .iter()
                .cloned()
                .zip(hit.tags.iter().copied())
                .collect();
        }
        self.misses += 1;
        let tagged = tag_sentence(model, sentence);
        self.map.insert(
            key,
            CachedSentence {
                pieces: tagged.iter().map(|(w, _)| w.clone()).collect(),
                tags: tagged.iter().map(|(_, t)| *t).collect(),
            },
        );
        tagged
    }

    /// Tag a whole document with cache reuse, one entry per sentence.
    pub fn tag_document(
        &mut self,
        model: &Model,
        doc: &english::Document,
    ) -> Vec<Vec<(String, Tag)>> {
        doc.paragraphs()
            .iter()
            .flat_map(|para| para.sentences())
            .map(|sent| self.tag_sentence(model, &sent))
            .collect()
    }
}
