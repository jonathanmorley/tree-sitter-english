# tree-sitter-english

This repository holds a proof-of-concept tree-sitter grammar for English
prose. It marks the block structure of text. It is not a syntactic parser.

Architecture overview (layers, budgets, rules of the road):
`ARCHITECTURE.md`. Working notes and backlog: `AGENTS.md`.

Live browser demo (tag, chunk, and lint in WebAssembly):
<https://jonathanmorley.github.io/tree-sitter-english/> — source in
`docs/`, Rust glue in `crates/english-web`.

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
  `till`, `whenever`, `where`, `whereas`, `wherever`, `whether`, `lest`,
  and `supposing` start subordinate clauses.
- Clause punctuation. A semicolon joins two coordinate clauses. A colon
  or an em dash (including ASCII `--` runs) introduces an elaborating
  clause. A sentence abandoned at an em dash or a bare colon hands off
  at a blank line. Parenthetical asides hold clauses joined by `;` and
  em dashes, and `&` joins like `and` where conjunctions are valid. All
  produce visible node types.
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

## Scope and error recovery contract

The grammar covers running English prose. The prose error histogram
(counted by the `audit` example, excluding transcription) is the
quality measure: an ERROR node on ordinary prose is a grammar miss.
Out of scope, bucketed separately as transcription rather than prose:
Gutenberg markup (`_` italics, `*` markers, `[...]` illustration
captions, `M^{r.}`-style superscript), front and back matter (title
pages, contents, transcriber's notes), epitaphs, speaker labels
(`AZORE SAILOR.`), stage directions, verse and song lyrics, and
navigation notation (`62o 17′ 20″`). These parse by error recovery,
not by grammar rules, by design.

## What the grammar does not do

- It does not mark subjects, verbs, or objects.
- It does not classify words. A clause is a flat run of words. Conjunctive
  adverbs such as `however` and `therefore` parse as plain words.
- Mid-sentence ellipses parse as `ellipsis` nodes; terminal `...`
  ends the sentence.
- It does not represent ambiguity. Garden-path sentences parse as flat
  clauses.

## Performance

The numbers come from Moby-Dick (Gutenberg text 2701), body text
from `CHAPTER 1. Loomings.` onward (1.21 MB).

| Measure | Result |
|---|---|
| Full parse | ~210 ms |
| Re-parse after a one-word edit | ~17 ms |
| Peak memory | ~58 MB (about 40 times the input size; prior measurement) |
| Structure extracted | 2,530 paragraphs, 9,973 sentences, 192,527 words |
| Prose errors | 4 (583 more bucketed as transcription) |

The memory figure limits the input size. A book fits. A large corpus does
not fit.

## Build and test

You need Nix.

1. Start the development shell: `nix develop`.
1. Generate the parser: `npx -y tree-sitter-cli@0.27.0 generate` (pinned;
   the shell's 0.26.9 churns `src/tree_sitter/array.h`, so don't use it).
1. Run the tests: `cargo test --workspace`. This runs the generated
   binding tests plus the typed AST crate in `crates/english`,
   including the full corpus in `test/corpus/`. Plain `cargo test`
   at the root only covers the root package.

The external scanner is written in Rust, which the tree-sitter CLI
cannot link: `tree-sitter test` and `tree-sitter parse` do not work.
(If you ran them before the port, delete the stale
`~/.cache/tree-sitter/lib/english.dylib` first.)
