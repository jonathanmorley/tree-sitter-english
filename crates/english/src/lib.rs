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
        children_of_kind(self.tree.root_node(), "paragraph")
            .into_iter()
            .map(|node| Paragraph {
                node,
                source: &self.source,
            })
            .collect()
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
        children_of_kind(self.node, "sentence")
            .into_iter()
            .map(|node| Sentence {
                node,
                source: self.source,
            })
            .collect()
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
        let mut clauses = Vec::new();
        for child in named_children(self.node) {
            if child.kind() == "clause" || child.kind() == "subordinate_clause" {
                clauses.push(Clause {
                    node: child,
                    source: self.source,
                });
            }
        }
        clauses
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
        for child in named_children(self.node) {
            match child.kind() {
                "clause" | "subordinate_clause" => {
                    push_clause_tokens(child, self.source, &mut out);
                }
                "complete_parenthetical" => {
                    for inner in named_children(child) {
                        if inner.kind() == "clause" || inner.kind() == "subordinate_clause" {
                            push_clause_tokens(inner, self.source, &mut out);
                        }
                    }
                }
                _ => {
                    if let Some(kind) = TokenKind::from_node_kind(child.kind()) {
                        out.push(Token {
                            kind,
                            node: child,
                            source: self.source,
                        });
                    }
                }
            }
        }
        out
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
        named_children(self.node)
            .into_iter()
            .find(|child| child.kind() == "subordinator")
            .map(|child| text_of(self.source, child))
    }

    /// Word-like tokens: `word`, `dotted` (initialisms) and `number`.
    ///
    /// This drops closed-class and punctuation siblings (subordinators,
    /// conjunctions, periods, quotes, …); use [`Clause::tokens`] for a
    /// lossless stream.
    pub fn words(&self) -> Vec<Word<'a>> {
        let mut words = Vec::new();
        for child in named_children(self.node) {
            if let Some(kind) = WordKind::from_node_kind(child.kind()) {
                words.push(Word {
                    kind,
                    node: child,
                    source: self.source,
                });
            }
        }
        words
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
        push_clause_tokens(self.node, self.source, &mut out);
        out
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

fn push_clause_tokens<'a>(node: Node<'a>, source: &'a str, out: &mut Vec<Token<'a>>) {
    for child in named_children(node) {
        match child.kind() {
            "parenthetical" => {
                for inner in named_children(child) {
                    if inner.kind() == "clause" || inner.kind() == "subordinate_clause" {
                        push_clause_tokens(inner, source, out);
                    }
                }
            }
            "clause" | "subordinate_clause" => {
                push_clause_tokens(child, source, out);
            }
            _ => {
                if let Some(kind) = TokenKind::from_node_kind(child.kind()) {
                    out.push(Token {
                        kind,
                        node: child,
                        source,
                    });
                }
            }
        }
    }
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
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

fn children_of_kind<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    named_children(node)
        .into_iter()
        .filter(|child| child.kind() == kind)
        .collect()
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
