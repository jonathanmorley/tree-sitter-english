# Feasibility study: a tree-sitter grammar for English prose

Date: 2026-08-12. Tooling: tree-sitter CLI 0.26.9 via Nix dev shell (see
`flake.nix`). Proof of concept lives in this repo.

## Verdict

Feasible for the requested scope, with one hard limit.

| Tier | Scope | Feasible | Evidence |
|---|---|---|---|
| 1 | Paragraph and sentence boundaries | Yes | 9/9 corpus tests, clean parse of Austen |
| 2 | Clauses, conjunctions as operators | Yes | 0 LR conflicts, deterministic grammar |
| 3 | Subject / verb / object | No | Attempted and dropped. See "Why tier 3 fails" |

A tree-sitter grammar works as a **prose block-structure layer**: it marks
paragraphs, sentences, and clause joins the way markdown grammars mark blocks.
It does not work as a syntactic parser.

## Proof of concept results

Reproduce with `nix develop -c tree-sitter test` and
`nix develop -c tree-sitter parse examples/<file>`.

- `tree-sitter generate` exits 0 with **zero declared conflicts**. Two LR
  conflicts during development were resolved with `prec.left`, not `conflicts`
  declarations (see `POC_NOTES.md`).
- Corpus: 16/16 pass (simple, compound `and`/`but`, subordinate
  `because`/`although`, relative `that`, multi-sentence paragraph, two
  paragraphs, SVO sentence, and seven abbreviation-robustness tests).
- Speed: 4350 bytes/ms in tests. Sample files parse in about 0.05 ms.
  Incremental re-parse comes free with tree-sitter.

Real prose results (`examples/*.parse.txt`):

- *Pride and Prejudice* opening sentence: clean parse. `that` recognized as
  `subordinator`. Bracketing coarse but self-consistent.
- *Origin of Species* opening: parses as **one sentence** after the
  abbreviation work below: `H.M.S.` lexes as a single `dotted` token. The
  remaining ERROR nodes are the two lone apostrophes in `'Beagle,'`, a
  word-token limit, not a boundary failure. Error recovery keeps the rest
  of the sentence parseable.
- Garden-path sentences: clean parse, zero structural insight. `The horse
  raced past the barn fell.` is one flat `clause` of seven `word` nodes.

## Prior art

Nothing parses English prose into sentence/clause structure with tree-sitter.

- [tree-sitter-plain](https://github.com/jbr/tree-sitter-plain): closest
  existing scaffold. Paragraph-scale units for plaintext. Mature, published
  on crates.io and npm.
- [tree-sitter/tree-sitter#3219](https://github.com/tree-sitter/tree-sitter/issues/3219):
  maintainers will not emit all parse trees for ambiguous grammars.
  Ambiguity-heavy grammars are discouraged.
- [tree-sitter/tree-sitter#1557](https://github.com/tree-sitter/tree-sitter/issues/1557):
  grammar-class limits. GLR with bounded lookahead, not arbitrary context
  sensitivity.
- [tree-sitter discussion 4702](https://github.com/tree-sitter/tree-sitter/discussions/4702):
  conflicts resolve by precedence and associativity at generate time, never
  at runtime.
- [retext / nlcst](https://github.com/retextjs/retext): mature JS prose AST
  with sentence/paragraph/word nodes. The closest node-set analogue. No
  clause layer, no conjunction operators.
- [Link Grammar](https://github.com/opencog/link-grammar): mature
  dictionary-driven English parser. Produces real subject/verb/object-style
  links and ranks ambiguous parses. Not incremental, not editor-shaped.
- Tomita, [*Graph-structured Stack and Natural Language Parsing*](https://aclanthology.org/P88-1031.pdf):
  canonical GLR-for-NLP result. Documents parse explosion under ambiguity.
- [PySBD / sentence segmentation literature](https://aclanthology.org/2021.acl-long.309.pdf):
  abbreviation boundaries are the classic hard case for sentence splitting.
- Harper and LanguageTool: rule-based proofreaders. Not structure parsers.

## Why tiers 1 and 2 fit tree-sitter

- Segmentation is near-regular. Punctuation plus a closed-class word list
  defines it, and closed-class lists map directly onto tree-sitter keyword
  extraction.
- Conjunctions behave like real operators: `sentence -> clause (conjunction
  clause)*` is the same shape as expression grammars, including left
  associativity via `prec.left`.
- Paragraph breaks need a lexer trick, and tree-sitter supports it: extras
  consume a lone `\n`, and the longer `paragraph_break` token `/\n[ \t]*\n/`
  wins longest-match on blank lines. No external scanner needed.
- Error recovery transfers from code to prose. The `'Beagle,'` error above
  damages one node, not the document.
- Sentence boundaries survive abbreviations via three layers, all proven in
  the PoC: `abbrev` keyword tokens absorb trailing dots (`Mr.`, `Dr.`); a
  `dotted` token matches initialism runs wholesale (`H.M.S.`, `e.g.`); and
  an external scanner decides every remaining bare dot by forward
  lookahead. A dot followed over whitespace by a lowercase letter or digit
  continues the sentence, because English sentences never start lowercase.
  The scanner needs no serialized state.

## Why tier 3 fails

The PoC attempted SVO fields with keyword lists for pronouns, articles, ~15
verbs, and common nouns, as a high-precedence alternative to the generic
word-run clause. It failed two ways:

1. **Precedence forces early commitment.** Tree-sitter does not backtrack.
   The SVO reading shares its leading tokens with the generic clause, so the
   parser commits whenever a sentence *starts* like SVO. `The book that I
   read was good.` then errors after consuming `The book`, instead of falling
   back. Precedence tuning only moved the failure.
2. **Fields on hidden tokens do not render.** `field('verb', $._verb)` points
   at an invisible node, so the field never appears in the tree.

Deeper reasons, independent of this PoC:

- Open-class words defeat regex classification. `fish`, `book`, and `run` are
  nouns and verbs. A grammar rule cannot tell them apart. Only a lexicon or
  tagger can.
- Real English syntax is ambiguous. `old men and women` has two coordination
  structures. A faithful grammar must keep both parses. Tree-sitter forces a
  single tree chosen by static precedence, and its maintainers consider
  ambiguity a defect to engineer out. That is the right stance for code and
  the wrong tool for syntax.
- GLR parse explosion is the documented failure mode when ambiguous natural
  language grammars meet a graph-stack parser (Tomita).

## Known hard cases

1. Initialisms and abbreviations ended sentences at every internal period.
   Now fixed for initialism runs (`dotted`), listed abbreviations
   (`abbrev`), and unknown abbreviations before lowercase continuations
   (scanner). Demonstrated: *Origin of Species* parses as one sentence.
   Residual gaps: spaced single-letter initials before a capital
   (`J. Smith`), unknown abbreviations before a capital-starting
   continuation, and `etc.` before a capital.
2. Lone quotes and apostrophes at word edges fail to lex. Demonstrated with
   `'Beagle,'`. Fix is a wider word token or scanner handling.
3. The subordinate clause is greedy in the PoC. It absorbs the rest of the
   sentence after its subordinator, which can swallow a later independent
   clause. A comma-aware boundary would fix most cases.
4. Garden paths parse clean and mean nothing. No tier-3 nodes, nothing to
   mis-analyze.
5. Decimals are handled by the `number` token. Ellipses are not: in
   `Wait... What happened.` the first dot ends the sentence and the other
   two dots become ERROR nodes. Quotation-mark conventions are the same
   unresolved boundary class.

## Alternatives, and when to prefer them

- **retext/nlcst**: if the goal is a JS prose AST for linting or transforms,
  use it. It already has the tier-1 node set. It has no clause layer and no
  operator treatment, so tiers 2 and beyond still need custom work.
- **Link Grammar**: if the goal is genuine subject/verb/object structure, use
  it or a dependency parser. Accept that it is a batch parser with no
  incremental re-parse and no editor integration.
- **Hybrid**: tree-sitter for structure (tiers 1-2) plus an external tagger
  or Link Grammar pass for roles (tier 3), joined by queries or post-hoc
  annotation. This is the only route to tier 3 that keeps the tree-sitter
  benefits.

## Recommendation

Proceed, scoped to tiers 1 and 2.

1. Build the prose block-structure grammar. The PoC shows the core is small
   and conflict-free.
2. Sentence-boundary disambiguation is proven feasible in the PoC: an
   abbreviation keyword list, an initialism token, and a forward-looking
   external scanner. Remaining work is coverage: widen the abbreviation
   list and add scanner access to the preceding word to fix spaced
   single-letter initials (`J. Smith`).
3. Treat tier 3 as out of scope for the grammar itself. If roles are needed,
   layer a real parser on top and write the results back as annotations.

The main risk is not technical. It is scope creep from a segmentation grammar
toward a syntactic one. The evidence above says that boundary is real.
