# tree-sitter-english

This repository holds a proof-of-concept tree-sitter grammar for English
prose. It marks the block structure of text. It is not a syntactic parser.

## What the grammar does

The grammar marks three levels of structure.

- Paragraphs. A blank line separates two paragraphs. LF and CRLF line
  endings both work.
- Sentences. A sentence ends at a period, a question mark, or an
  exclamation mark.
- Clauses. The coordinating conjunctions `and`, `but`, `or`, `nor`, `so`,
  `yet`, and `for` join clauses. Subordinators such as `because`,
  `although`, `that`, `if`, `when`, `while`, `since`, `unless`, `before`,
  `after`, `until`, `which`, `who`, `whom`, `whose`, `as`, `once`, `than`,
  `till`, `whenever`, `where`, `whereas`, `wherever`, and `whether` start
  subordinate clauses.
- Clause punctuation. A semicolon joins two coordinate clauses. A colon
  or an em dash introduces an elaborating clause. A sentence abandoned
  at an em dash or a bare colon hands off at a blank line. Parenthetical
  asides hold clauses joined by `;` and em dashes, and `&` joins like
  `and` where conjunctions are valid. All produce visible
  node types.
- Quote marks. The grammar accepts ASCII and curly quotes. Curly
  apostrophes in possessives such as `ship's` lex as part of the word.
  Non-ASCII letters such as `æ`, `œ`, and `é` lex as part of words.

The grammar keeps sentence boundaries correct around abbreviations.

- It accepts `Mr.` and `Dr.` without a sentence break.
- It accepts initialism strings such as `H.M.S.` and `e.g.`.
- It accepts single-letter initials with a space, for example
  `J. Smith arrived.`
- An unknown abbreviation before a lowercase word also keeps the sentence
  open.

The grammar recovers from unknown characters. A bad character produces an
ERROR node, and the parse continues.

## What the grammar does not do

- It does not mark subjects, verbs, or objects.
- It does not classify words. A clause is a flat run of words. Conjunctive
  adverbs such as `however` and `therefore` parse as plain words.
- It does not handle ellipses.
- It does not represent ambiguity. Garden-path sentences parse as flat
  clauses.

## Performance

The numbers come from Moby-Dick (1.27 MB, Project Gutenberg text 2701).

| Measure | Result |
|---|---|
| Full parse | 131 ms (median of five runs) |
| Re-parse after a one-word edit | 37 ms |
| Peak memory | 58 MB (about 40 times the input size) |
| Structure extracted | 2,635 paragraphs, 10,475 sentences, 204,552 words |

The memory figure limits the input size. A book fits. A large corpus does
not fit.

## Build and test

You need Nix.

1. Start the development shell: `nix develop`.
1. Generate the parser: `tree-sitter generate`.
1. Run the tests: `cargo test --workspace`. This runs the generated
   binding tests plus the typed AST crate in `crates/english`,
   including the full corpus in `test/corpus/`. Plain `cargo test`
   at the root only covers the root package.

The external scanner is written in Rust, which the tree-sitter CLI
cannot link: `tree-sitter test` and `tree-sitter parse` do not work.
(If you ran them before the port, delete the stale
`~/.cache/tree-sitter/lib/english.dylib` first.)
