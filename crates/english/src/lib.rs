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

    /// The introducing subordinator (`because`, `who`, …), if any.
    pub fn subordinator(&self) -> Option<&'a str> {
        named_children(self.node)
            .into_iter()
            .find(|child| child.kind() == "subordinator")
            .map(|child| text_of(self.source, child))
    }

    /// Word-like tokens: `word`, `dotted` (initialisms) and `number`.
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

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
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
