//! Typed Rust AST over the tree-sitter English grammar.
//!
//! The grammar itself only segments prose into paragraphs, sentences and
//! flat clauses (see the crate-level docs of `tree-sitter-english`); this
//! crate wraps the resulting concrete tree in borrowed, typed nodes so
//! analysis code never matches on raw kind strings.
//!
//! Only node types the grammar actually produces are represented here:
//! there are deliberately no noun/verb phrase types (subject/object
//! extraction was tried upstream and dropped — see `grammar.js` Tier 3).

use std::ops::Range;

use tree_sitter::{Language, Node, Parser, Tree};

macro_rules! span_and_text {
    () => {
        /// Byte span of this node within the document source.
        pub fn span(&self) -> Range<usize> {
            self.node.start_byte()..self.node.end_byte()
        }

        /// Source text covered by this node.
        pub fn text(&self) -> &'a str {
            text_of(self.source, self.node)
        }
    };
}

/// The tree-sitter [`Language`] for English prose.
pub fn language() -> Language {
    tree_sitter_english::LANGUAGE.into()
}

/// A parsed English document: owned source text plus its concrete tree.
#[derive(Debug)]
pub struct Document {
    source: String,
    tree: Tree,
}

impl Document {
    /// Parse `source` into a [`Document`].
    ///
    /// Parsing only fails if the operation is cancelled or times out, which
    /// cannot happen with a default parser; use [`Document::has_error`] to
    /// detect input the grammar could not cover.
    pub fn parse(source: impl Into<String>) -> Self {
        let source = source.into();
        let mut parser = Parser::new();
        parser
            .set_language(&language())
            .expect("failed to load English grammar");
        let tree = parser
            .parse(&source, None)
            .expect("parse failed: operation cancelled");
        Self { source, tree }
    }

    /// The source text this document was parsed from.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Reparse after an edit, reusing the old tree so only changed
    /// regions are re-parsed (tree-sitter incremental parsing: ~17 ms
    /// on book-size input vs ~240 ms cold). Pair with `TagCache` in
    /// `english-pos` so tagging also skips unchanged sentences.
    ///
    /// Correctness note: the old tree's ranges must first be adjusted
    /// for the edit (`Tree::edit`), otherwise node reuse reads stale
    /// positions and silently drops shifted text. The adjustment here
    /// is a prefix/suffix diff — a valid (not minimal) edit, so reuse
    /// is conservative but always correct.
    pub fn update(&mut self, source: impl Into<String>) {
        let source = source.into();
        let edit = compute_edit(&self.source, &source);
        self.tree.edit(&edit);
        let mut parser = Parser::new();
        parser
            .set_language(&language())
            .expect("failed to load English grammar");
        let tree = parser
            .parse(&source, Some(&self.tree))
            .expect("parse failed: operation cancelled");
        self.source = source;
        self.tree = tree;
    }

    /// True when the tree contains error nodes.
    pub fn has_error(&self) -> bool {
        self.tree.root_node().has_error()
    }

    /// Top-level paragraphs in order.
    pub fn paragraphs(&self) -> Vec<Paragraph<'_>> {
        let mut out = Vec::new();
        self.for_each_paragraph(|p| out.push(p));
        out
    }

    /// Call `f` for each top-level paragraph, in order, without
    /// allocating the intermediate `Vec` ([`Document::paragraphs`]
    /// collects this). The book-scale tagging path uses this so a
    /// 10k-sentence document costs no per-node `Vec`s.
    pub fn for_each_paragraph<'s>(&'s self, mut f: impl FnMut(Paragraph<'s>)) {
        for_each_named_child(self.tree.root_node(), |node| {
            if node.kind() == "paragraph" {
                f(Paragraph {
                    node,
                    source: &self.source,
                });
            }
        });
    }
}

/// A paragraph: one or more sentences separated from neighbours by a blank line.
#[derive(Debug, Clone, Copy)]
pub struct Paragraph<'a> {
    node: Node<'a>,
    source: &'a str,
}

impl<'a> Paragraph<'a> {
    /// Sentences in order.
    pub fn sentences(&self) -> Vec<Sentence<'a>> {
        let mut out = Vec::new();
        self.for_each_sentence(|s| out.push(s));
        out
    }

    /// Call `f` for each sentence, in order, without allocating.
    pub fn for_each_sentence(&self, mut f: impl FnMut(Sentence<'a>)) {
        let source = self.source;
        for_each_named_child(self.node, |node| {
            if node.kind() == "sentence" {
                f(Sentence { node, source });
            }
        });
    }

    span_and_text!();
}

/// A sentence: clauses joined by conjunctions or clause punctuation,
/// terminated by `.`, `?` or `!`.
#[derive(Debug, Clone, Copy)]
pub struct Sentence<'a> {
    node: Node<'a>,
    source: &'a str,
}

impl<'a> Sentence<'a> {
    /// Coordinate and subordinate clauses in order.
    pub fn clauses(&self) -> Vec<Clause<'a>> {
        let mut out = Vec::new();
        self.for_each_clause(|c| out.push(c));
        out
    }

    /// Call `f` for each coordinate/subordinate clause, in order,
    /// without allocating.
    pub fn for_each_clause(&self, mut f: impl FnMut(Clause<'a>)) {
        let source = self.source;
        for_each_named_child(self.node, |child| {
            if child.kind() == "clause" || child.kind() == "subordinate_clause" {
                f(Clause {
                    node: child,
                    source,
                });
            }
        });
    }

    /// True when this sentence's subtree contains ERROR or MISSING nodes.
    ///
    /// Walks all children (not just named ones), so hidden-rule
    /// recoveries are included — unlike a named-only walk, which never
    /// yields MISSING aux symbols (see AGENTS.md).
    pub fn has_error(&self) -> bool {
        subtree_has_error(self.node)
    }

    /// Every visible terminal in this sentence, in order, with no drops.
    ///
    /// This flattens clause joiners (`conjunction`, `semicolon`, `colon`,
    /// `em_dash`), terminal `ellipsis`, and parenthetical interiors, so
    /// consumers (e.g. POS tagging) never silently lose tokens the way
    /// `Clause::words()` drops subordinators. Hidden punctuation (commas,
    /// sentence-final `.`/`?`/`!`, parens themselves) has no named node
    /// and is not yielded; see `text()` for the raw span.
    pub fn tokens(&self) -> Vec<Token<'a>> {
        let mut out = Vec::new();
        self.for_each_token(|t| out.push(t));
        out
    }

    /// Call `f` for every visible terminal, in order, with no drops
    /// and no intermediate `Vec` (see [`Sentence::tokens`] for what is
    /// yielded). The tagging hot path uses this.
    pub fn for_each_token(&self, mut f: impl FnMut(Token<'a>)) {
        let (node, source) = (self.node, self.source);
        for_each_named_child(node, |child| match child.kind() {
            "clause" | "subordinate_clause" => {
                for_each_clause_token(child, source, &mut f);
            }
            "complete_parenthetical" => {
                for_each_named_child(child, |inner| {
                    if inner.kind() == "clause" || inner.kind() == "subordinate_clause" {
                        for_each_clause_token(inner, source, &mut f);
                    }
                });
            }
            _ => {
                if let Some(kind) = TokenKind::from_node_kind(child.kind()) {
                    f(Token {
                        kind,
                        node: child,
                        source,
                    });
                }
            }
        });
    }

    span_and_text!();
}

/// A clause: a flat run of words (plus in-sentence periods, commas, quotes).
#[derive(Debug, Clone, Copy)]
pub struct Clause<'a> {
    node: Node<'a>,
    source: &'a str,
}

impl<'a> Clause<'a> {
    /// True for `subordinate_clause` (introduced by a subordinator).
    pub fn is_subordinate(&self) -> bool {
        self.node.kind() == "subordinate_clause"
    }

    /// True when this clause's subtree contains ERROR or MISSING nodes.
    /// See [`Sentence::has_error`] for walk semantics.
    pub fn has_error(&self) -> bool {
        subtree_has_error(self.node)
    }

    /// The introducing subordinator (`because`, `who`, …), if any.
    pub fn subordinator(&self) -> Option<&'a str> {
        let mut found = None;
        for_each_named_child(self.node, |child| {
            if found.is_none() && child.kind() == "subordinator" {
                found = Some(text_of(self.source, child));
            }
        });
        found
    }

    /// Word-like tokens: `word`, `dotted` (initialisms) and `number`.
    ///
    /// This drops closed-class and punctuation siblings (subordinators,
    /// conjunctions, periods, quotes, …); use [`Clause::tokens`] for a
    /// lossless stream.
    pub fn words(&self) -> Vec<Word<'a>> {
        let mut words = Vec::new();
        self.for_each_word(|w| words.push(w));
        words
    }

    /// Call `f` for each word-like token, in order, without allocating.
    pub fn for_each_word(&self, mut f: impl FnMut(Word<'a>)) {
        let source = self.source;
        for_each_named_child(self.node, |child| {
            if let Some(kind) = WordKind::from_node_kind(child.kind()) {
                f(Word {
                    kind,
                    node: child,
                    source,
                });
            }
        });
    }

    /// Every visible terminal in this clause, in order, with no drops.
    ///
    /// Yields `word`/`dotted`/`number` plus `subordinator`, `conjunction`
    /// (inside subordinate clauses), in-sentence `period`, `quote`,
    /// `ellipsis` and `currency`. Nested `parenthetical` and
    /// `subordinate_clause` interiors are flattened in place; the parens
    /// themselves and hidden punctuation (commas, sentence-final marks)
    /// have no named node and are not yielded.
    pub fn tokens(&self) -> Vec<Token<'a>> {
        let mut out = Vec::new();
        self.for_each_token(|t| out.push(t));
        out
    }

    /// Call `f` for every visible terminal, in order, with no drops
    /// and no intermediate `Vec` (see [`Clause::tokens`]).
    pub fn for_each_token(&self, mut f: impl FnMut(Token<'a>)) {
        for_each_clause_token(self.node, self.source, &mut f);
    }

    span_and_text!();
}

/// A single word-like token.
#[derive(Debug, Clone, Copy)]
pub struct Word<'a> {
    kind: WordKind,
    node: Node<'a>,
    source: &'a str,
}

impl<'a> Word<'a> {
    /// Whether this is a plain word, an initialism run or a number.
    pub fn kind(&self) -> WordKind {
        self.kind
    }

    span_and_text!();
}

/// The three word-like token kinds the grammar produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKind {
    /// An open-class word run (apostrophes included).
    Word,
    /// An initialism run such as `H.M.S.` or `e.g.`.
    Dotted,
    /// An integer or decimal.
    Number,
}

impl WordKind {
    fn from_node_kind(kind: &str) -> Option<Self> {
        match kind {
            "word" => Some(Self::Word),
            "dotted" => Some(Self::Dotted),
            "number" => Some(Self::Number),
            _ => None,
        }
    }
}

/// Every visible terminal kind the grammar produces inside sentences.
///
/// Hidden punctuation (commas, sentence-final `.`/`?`/`!`, parens
/// themselves) has no named node and is therefore absent here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// An open-class word run (apostrophes included).
    Word,
    /// An initialism run such as `H.M.S.` or `e.g.`.
    Dotted,
    /// An integer or decimal.
    Number,
    /// An in-sentence period (abbreviation dots; terminal dots are hidden).
    Period,
    /// `and`, `but`, `or`, `nor`, `so`, `yet`, `for` joining clauses.
    Conjunction,
    /// `because`, `although`, `that`, `who`, … introducing a subordinate clause.
    Subordinator,
    /// Quote marks inside clauses (`"`, `'`, curly variants).
    Quote,
    /// Mid-sentence `...` (terminal `...` also surfaces as this kind).
    Ellipsis,
    /// Currency signs (`$`, `£`).
    Currency,
    /// `;` joining coordinate clauses.
    Semicolon,
    /// `:` introducing elaboration.
    Colon,
    /// Em/en dash (`—`, `–`).
    EmDash,
}

impl TokenKind {
    fn from_node_kind(kind: &str) -> Option<Self> {
        match kind {
            "word" => Some(Self::Word),
            "dotted" => Some(Self::Dotted),
            "number" => Some(Self::Number),
            "period" => Some(Self::Period),
            "conjunction" => Some(Self::Conjunction),
            "subordinator" => Some(Self::Subordinator),
            "quote" => Some(Self::Quote),
            "ellipsis" => Some(Self::Ellipsis),
            "currency" => Some(Self::Currency),
            "semicolon" => Some(Self::Semicolon),
            "colon" => Some(Self::Colon),
            "em_dash" => Some(Self::EmDash),
            _ => None,
        }
    }
}

/// A single visible terminal with its source text and span.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    kind: TokenKind,
    node: Node<'a>,
    source: &'a str,
}

impl<'a> Token<'a> {
    /// The terminal kind.
    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    span_and_text!();
}

/// Call `f` for every visible terminal under a clause (or nested
/// clause) node, in order, flattening `parenthetical` and
/// `subordinate_clause` interiors in place. Lazy replacement for the
/// old push-into-`Vec` walk: no per-node allocation.
fn for_each_clause_token<'a>(node: Node<'a>, source: &'a str, f: &mut impl FnMut(Token<'a>)) {
    for_each_named_child(node, |child| match child.kind() {
        "parenthetical" => {
            for_each_named_child(child, |inner| {
                if inner.kind() == "clause" || inner.kind() == "subordinate_clause" {
                    for_each_clause_token(inner, source, f);
                }
            });
        }
        "clause" | "subordinate_clause" => {
            for_each_clause_token(child, source, f);
        }
        _ => {
            if let Some(kind) = TokenKind::from_node_kind(child.kind()) {
                f(Token {
                    kind,
                    node: child,
                    source,
                });
            }
        }
    });
}

/// Call `f` for each named child of `node`, in order, driving the
/// tree cursor directly instead of collecting children into a `Vec`.
fn for_each_named_child<'x>(node: Node<'x>, mut f: impl FnMut(Node<'x>)) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        f(child);
    }
}

/// True when `node` or any descendant (named or anonymous) is ERROR or
/// MISSING. Anonymous children matter: MISSING nodes for hidden-rule
/// aux symbols never appear in named iteration.
fn subtree_has_error(node: Node<'_>) -> bool {
    if node.is_error() || node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).any(subtree_has_error)
}

fn text_of<'a>(source: &'a str, node: Node<'_>) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

/// Byte offset plus row/column of a byte position.
fn position_of(text: &str, byte: usize) -> tree_sitter::Point {
    let mut row = 0;
    let mut column = 0;
    for (i, c) in text.char_indices() {
        if i >= byte {
            break;
        }
        if c == '\n' {
            row += 1;
            column = 0;
        } else {
            column += c.len_utf8();
        }
    }
    tree_sitter::Point { row, column }
}

/// Prefix/suffix diff between `old` and `new` as a tree-sitter input
/// edit. Boundaries snap back to char boundaries; the region between
/// them covers every difference, so tree reuse outside it is sound.
fn compute_edit(old: &str, new: &str) -> tree_sitter::InputEdit {
    let old_bytes = old.as_bytes();
    let new_bytes = new.as_bytes();
    let mut prefix = 0;
    while prefix < old_bytes.len()
        && prefix < new_bytes.len()
        && old_bytes[prefix] == new_bytes[prefix]
    {
        prefix += 1;
    }
    while !old.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let mut suffix = 0;
    while suffix < old_bytes.len() - prefix
        && suffix < new_bytes.len() - prefix
        && old_bytes[old_bytes.len() - 1 - suffix] == new_bytes[new_bytes.len() - 1 - suffix]
    {
        suffix += 1;
    }
    while !old.is_char_boundary(old_bytes.len() - suffix) {
        suffix -= 1;
    }
    while !new.is_char_boundary(new_bytes.len() - suffix) {
        suffix -= 1;
    }
    tree_sitter::InputEdit {
        start_byte: prefix,
        old_end_byte: old_bytes.len() - suffix,
        new_end_byte: new_bytes.len() - suffix,
        start_position: position_of(old, prefix),
        old_end_position: position_of(old, old_bytes.len() - suffix),
        new_end_position: position_of(new, new_bytes.len() - suffix),
    }
}
