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

use crate::{Model, RULES, Tag, apply_rules};

/// Split a whole grammar word into UD-style pieces.
///
/// Splits on ASCII `'` and curly `’` (normalized to ASCII in the
/// output), keeping the apostrophe at the start of the following piece:
/// `ship’s` → `ship` + `'s`. The `n't` suffix keeps its `n` (`don't` →
/// `do` + `n't`, matching UD EWT). A leading apostrophe (`'em`) stays
/// whole; a trailing one (`dogs'`) yields a lone `"'"` piece. Words
/// without apostrophes return unchanged.
pub fn split_contraction(word: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    push_contraction_pieces(&mut pieces, word);
    if pieces.is_empty() {
        vec![word.to_string()]
    } else {
        pieces
    }
}

/// Push UD pieces for `word` onto `out` (pushes nothing when the word
/// needs no split). Split logic shared with [`split_contraction`]:
/// the public function wraps this with the whole-word fallback.
fn push_contraction_pieces(out: &mut Vec<String>, word: &str) {
    if !word.contains('\'') && !word.contains('’') {
        return;
    }
    // `n't` keeps its `n`: don't → do + n't (not don + 't).
    let lower = word.to_lowercase();
    let is_nt = lower.ends_with("n't") || lower.ends_with("n’t");
    if is_nt {
        let chars: Vec<char> = word.chars().collect();
        if chars.len() > 3 {
            let stem: String = chars[..chars.len() - 3].iter().collect();
            if !stem.is_empty() {
                out.push(stem);
                out.push("n't".to_string());
                return;
            }
        }
        return;
    }
    let mut current = String::new();
    for ch in word.chars() {
        if ch == '\'' || ch == '’' {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            current.push('\'');
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        // A lone leading apostrophe means the whole word was `'em`-style:
        // if nothing was pushed yet, the single piece is the word itself.
        out.push(current);
    } else if word.ends_with('\'') || word.ends_with('’') {
        out.push("'".to_string());
    }
}

/// Expand one grammar token into UD pieces for model input.
///
/// Only `Word`-kind tokens containing apostrophes expand; everything
/// else (`dotted`, `number`, conjunctions, subordinators, punctuation)
/// passes through whole.
pub fn token_pieces(token: &english::Token) -> Vec<String> {
    let mut out = Vec::new();
    push_token_pieces(&mut out, token);
    out
}

/// Push UD pieces for one grammar token onto `out` without the
/// interim per-token `Vec` ([`token_pieces`] wraps this).
fn push_token_pieces(out: &mut Vec<String>, token: &english::Token) {
    if token.kind() == english::TokenKind::Word {
        if let Some(fused) = split_fused(token.text()) {
            out.extend(fused);
            return;
        }
        let before = out.len();
        push_contraction_pieces(out, token.text());
        if out.len() > before {
            return;
        }
    }
    out.push(token.text().to_string());
}

/// Split fused informal contractions without apostrophes (MacIntyre):
/// `gonna` → `gon` + `na`, `cannot` → `can` + `not`. UD splits them
/// (EWT `gon`/VERB + `na`/PART; UD convention for `cannot`), so whole
/// words would train/infer mismatched. Stem keeps surface case
/// (`Gonna` → `Gon` + `na`, matching UD surface forms); tail is
/// lowercase. `wanna`/`gotta` deliberately excluded: zero gold
/// instances in EWT+GUM combined, and `wan` collides with the pale
/// adjective — revisit with evidence.
fn split_fused(word: &str) -> Option<Vec<String>> {
    // ASCII-only forms below, so byte index 3 is always a boundary.
    let lower = word.to_lowercase();
    if lower == "gonna" || lower == "cannot" {
        let (stem, tail) = word.split_at(3);
        Some(vec![stem.to_string(), tail.to_lowercase()])
    } else {
        None
    }
}

/// UD-style pieces for a parsed clause, in order.
pub fn clause_pieces(clause: &english::Clause) -> Vec<String> {
    let mut out = Vec::new();
    append_clause_pieces(&mut out, clause);
    out
}

/// Append UD-style pieces for a parsed clause to `out`, reusing its
/// buffer instead of allocating a fresh `Vec` per call.
pub fn append_clause_pieces(out: &mut Vec<String>, clause: &english::Clause) {
    let mut prev: Option<(usize, bool)> = None;
    clause.for_each_token(|tok| push_piece_merged(&mut *out, &mut prev, tok));
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
    let mut out = Vec::new();
    append_sentence_pieces(&mut out, sentence);
    out
}

/// Append UD-style pieces for a parsed sentence to `out`, reusing its
/// buffer. The book-scale bench path clears one buffer per sentence
/// instead of allocating a fresh `Vec` (plus its `Token` staging
/// `Vec`) every time.
pub fn append_sentence_pieces(out: &mut Vec<String>, sentence: &english::Sentence) {
    let mut prev: Option<(usize, bool)> = None;
    sentence.for_each_token(|tok| push_piece_merged(&mut *out, &mut prev, tok));
}

/// Expand one visited token into pieces, merging in-sentence `period`
/// into a byte-adjacent preceding `Word`/`Dotted` token. Called per
/// token by the `append_*` walkers above, which drive the token
/// stream lazily instead of staging it in a `Vec`.
fn push_piece_merged(out: &mut Vec<String>, prev: &mut Option<(usize, bool)>, tok: english::Token) {
    let span = tok.span();
    if tok.kind() == english::TokenKind::Period
        && matches!(*prev, Some((end, true)) if end == span.start)
        && let Some(last) = out.last_mut()
    {
        last.push_str(tok.text());
        *prev = Some((span.end, false));
        return;
    }
    let wordish = matches!(
        tok.kind(),
        english::TokenKind::Word | english::TokenKind::Dotted
    );
    push_token_pieces(out, &tok);
    *prev = Some((span.end, wordish));
}

/// Tag a parsed clause: `(surface piece, tag)` pairs in order.
///
/// Contraction pieces expand (see [`split_contraction`]), so the output
/// may hold more items than the clause has tokens.
///
/// Decodes with the beam re-decoder ([`Model::tag_beam_margins`] at
/// [`BEAM_MARGIN_T`]/[`BEAM_MAX_SPAN`]), then gated correction rules
/// (`correction::RULES`); with no admitted rules the guard below
/// keeps correction at one branch. `Model::tag` stays greedy —
/// evals pin the model, the beam delta is measured at admission
/// (EWT dev +13 / test +24, Moby +1, genre/chunk ±0, `flies` holds).
pub fn tag_clause(model: &Model, clause: &english::Clause) -> Vec<(String, Tag)> {
    let pieces = clause_pieces(clause);
    let mut tagged = model.tag_beam_margins(&pieces);
    if !RULES.is_empty() {
        apply_rules(&pieces, &mut tagged, RULES);
    }
    tagged
        .into_iter()
        .zip(pieces)
        .map(|((tag, _), text)| (text, tag))
        .collect()
}

/// Tag a parsed sentence: `(surface piece, tag)` pairs in order.
///
/// Each sentence is decoded independently (tag history resets at the
/// boundary), matching training on UD sentences.
pub fn tag_sentence(model: &Model, sentence: &english::Sentence) -> Vec<(String, Tag)> {
    let pieces = sentence_pieces(sentence);
    let mut tagged = model.tag_beam_margins(&pieces);
    if !RULES.is_empty() {
        apply_rules(&pieces, &mut tagged, RULES);
    }
    tagged
        .into_iter()
        .zip(pieces)
        .map(|((tag, _), text)| (text, tag))
        .collect()
}

/// Tag a whole document, one entry per sentence in document order.
///
/// Each sentence is decoded independently; see [`tag_sentence`].
pub fn tag_document(model: &Model, doc: &english::Document) -> Vec<Vec<(String, Tag)>> {
    let mut out = Vec::new();
    doc.for_each_paragraph(|para| {
        para.for_each_sentence(|sent| out.push(tag_sentence(model, &sent)));
    });
    out
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

/// One cached sentence: its tagged `(surface piece, tag)` pairs.
///
/// Stored zipped (not as parallel `pieces`/`tags`) so a borrowed hit
/// hands out one slice with no re-zipping; pieces are stored (not
/// re-extracted) so a hit also skips the tree walk and contraction
/// splitting.
#[derive(Debug, Clone)]
struct CachedSentence {
    tagged: Vec<(String, Tag)>,
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
    /// piece extraction entirely (stored pairs are cloned).
    pub fn tag_sentence(
        &mut self,
        model: &Model,
        sentence: &english::Sentence,
    ) -> Vec<(String, Tag)> {
        self.tag_sentence_ref(model, sentence).to_vec()
    }

    /// Borrow a sentence's tags, reusing a cached result when its text
    /// was seen. Same pairs as [`TagCache::tag_sentence`] with no
    /// per-hit clone — the keystroke reader's path (one changed
    /// sentence, read in place). The lookup borrows the sentence text
    /// directly, so hits also skip the key allocation; only misses
    /// allocate (key + first tag).
    pub fn tag_sentence_ref(
        &mut self,
        model: &Model,
        sentence: &english::Sentence,
    ) -> &[(String, Tag)] {
        let key = sentence.text();
        if self.map.contains_key(key) {
            self.hits += 1;
        } else {
            self.misses += 1;
            let tagged = tag_sentence(model, sentence);
            self.map.insert(key.to_string(), CachedSentence { tagged });
        }
        &self.map.get(key).expect("inserted above").tagged
    }

    /// Tag a whole document with cache reuse, one entry per sentence.
    pub fn tag_document(
        &mut self,
        model: &Model,
        doc: &english::Document,
    ) -> Vec<Vec<(String, Tag)>> {
        let mut out = Vec::new();
        doc.for_each_paragraph(|para| {
            para.for_each_sentence(|sent| out.push(self.tag_sentence(model, &sent)));
        });
        out
    }
}
