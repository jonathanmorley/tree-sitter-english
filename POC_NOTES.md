# POC_NOTES — tree-sitter grammar for English prose

Proof-of-concept grammar in this repo. Repo root: `/Users/alexispurslane/Development/english_ts`.
All tooling is run inside the Nix dev shell (`nix develop -c tree-sitter …`).

## Conflict summary

The final grammar (tiers 1–2) **generates with 0 LR conflicts and declares no
`conflicts` array at all** (`nix develop -c tree-sitter generate` exits 0 and
prints no conflict warnings).

During development the following LR conflicts appeared and were resolved by
precedence rather than `conflicts` declarations:

- `paragraph_repeat1 • 'because'` — a subordinator can start a new sentence, so
  the parser could not tell whether the current paragraph continued. Resolved by
  restructuring paragraph boundaries (see "paragraph-break technique") plus
  `prec.left` on `clause` and on `subordinate_clause`'s inner repeat.
- `subordinator subordinate_clause_repeat1 • 'and'` — a conjunction can continue
  a subordinate clause; a nested repeat. Resolved with `prec.left`.
- Tier 3 (SVO fields) produced a cascade of `_pronoun/_article • <token>`
  conflicts between the structured `svo_clause`/`noun_phrase` and the generic
  `_content` word-run. These were the reason tier 3 was dropped (see below).

Count of LR conflicts that required a `conflicts` declaration in the final
grammar: **0**.

## Which tiers work

- **Tier 1 (paragraph/sentence boundaries): works.** `source_file → paragraph+`,
  blank line = paragraph break, sentence ends at `.` `?` `!` (closing
  quote/paren allowed after the mark). A single newline inside a paragraph is
  whitespace and belongs to the sentence; only a blank line splits paragraphs.
- **Tier 2 (clause / conjunction / subordinate_clause): works.** `sentence` is a
  `clause` (a word/comma run) with optional `conjunction`-joined clauses and
  `subordinate_clause`s introduced by `subordinator` keywords (both lowercase and
  capitalized forms via tree-sitter keyword extraction).
- **Tier 3 (subject/verb/object fields): DROPPED (not implemented).** See below.

## Tier 3 — why it was dropped (failure mode, not hidden)

A first attempt added keyword lists for pronouns, articles, ~15 common verbs and
common nouns, plus a structured `svo_clause` with `field('subject'/'verb'/'object')`
as a higher-precedence alternative to the generic word-run clause. It failed on
two independent grounds:

1. **Precedence forcing breaks fallback.** Because the SVO clause shares its
   leading tokens (pronoun/article/noun) with the generic word-run, the parser
   commits to the SVO reading whenever one *could* start. Tree-sitter does not
   backtrack, so a sentence that merely *starts* like SVO but is not SVO ends in
   an ERROR instead of falling back to the generic clause. Concretely, with the
   tier-3 grammar:
   ```
   (ERROR [0, 0] - [0, 8]        ; "The book that I read was good."
     (noun_phrase [0, 0] - [0, 8]))
   ```
   (subject "The book" was consumed, then the verb was required but `that` is a
   subordinator, so the parse errored and words were dropped.) Raising/lowering
   precedence only moved the problem; the SVO and generic parses overlap in every
   token, so there is no LR-conflict-free way to prefer SVO only when it fully
   matches.
2. **Fields on hidden tokens do not render.** The SVO child tokens
   (`_verb`, `_noun`, `_pronoun`, `_article`) are hidden rules, so
   `field('verb', …)` etc. point at invisible nodes and the `verb`/`object`
   fields do not appear in the tree at all (only the visible `noun_phrase`
   subject field rendered).

Per the assignment, tier 3 was therefore reduced to nothing: `The dog chased the
cat.` now parses as a plain generic `clause` of five `word` tokens (see the last
corpus test). Real SVO analysis is left for a grammar that either does genuine
syntactic structure (not keyword extraction) or uses an external scanner.

## Paragraph-break lexing technique

A blank line is a `paragraph_break` token `/\n[ \t]*\n/`, while single newlines
(and spaces/tabs) are whitespace in `extras: $ => [/\s/]`. The lexer uses
longest-match: at a blank line `paragraph_break` (length ≥ 2) beats the single
`\n` extra (length 1), so the blank line lexes as `paragraph_break`; a lone
newline lexes as the `\s` extra and is silently absorbed inside the sentence.
No external scanner was needed.

Crucially, `paragraph_break` is used only as a **separator at `source_file`
level**, not as an optional trailing element of `paragraph`:

```
source_file: seq(repeat(paragraph_break), paragraph,
                 repeat(seq(repeat1(paragraph_break), paragraph)),
                 repeat(paragraph_break))
paragraph:   repeat1(sentence)
```

This stops the parser from ending a paragraph after any sentence (the earlier
`paragraph: repeat1(sentence) + repeat(paragraph_break)` let the optional
trailing break reduce at every sentence end, which wrongly split sentences that
share a line into separate paragraphs). With the separator form, a paragraph
only ends at a blank line or EOF.

## Abbreviation robustness (stateful external scanner)

`word`, `conjunction`, `subordinator` and the two period tokens are all
external, lexed by `src/scanner.c`. The scanner stores the lowercased text
of the last lexed word in its payload, round-tripped through
`serialize`/`deserialize`, so the state survives incremental re-parse.

The scanner replaces tree-sitter keyword extraction, which external tokens
bypass: it emits a closed-class word as `conjunction`/`subordinator` only
where `valid_symbols` allows that symbol, and lexes it as a plain word
otherwise (`For home he left.`).

Decision for a bare period, in order:

1. previous word is a single letter other than `a`/`i` → `PERIOD` (spaced
   initial: `J. Smith`). `a`/`i` stay splittable: `am I. You`.
2. previous word is a known abbreviation (`mr` `mrs` `ms` `dr` `st` `jr`
   `sr` `vs` `inc` `ltd` `co` `no` `fig` `vol` `approx`) → `PERIOD`.
3. dot followed over whitespace by a lowercase letter or digit → `PERIOD`
   (unknown abbreviations, spaced runs like `p. 42`).
4. otherwise → sentence end.

The `dotted` token still absorbs initialism runs wholesale (`H.M.S.`,
`e.g.`): the scanner refuses letter-dot-letter sequences so the internal
token wins. `number` keeps decimal dots internal.

Two implementation traps, found by debugging:

- **The scanner runs before extras are skipped**, at the raw whitespace
  position. If it returns false there, tree-sitter does not re-invoke it
  after the skip. The scanner must skip whitespace itself, and must refuse
  when a blank line is forming, so the internal `paragraph_break` token
  still matches.
- **State updates only on emitted words.** Internal tokens (`dotted`,
  `number`) and hidden commas do not touch the state; that is harmless
  because none of them can precede a meaningful period.

Residual gaps (documented, not fixed):

- A genuine initial `I.` (`I. M. Pei`) splits; the `a`/`i` exclusion trades
  that rarity against the far more common pronouns.
- Unknown abbreviations at end of line before a capital-starting
  continuation still split.
- Ellipses: the first dot ends the sentence, the rest become ERROR nodes.

## Corpus tests

`nix develop -c tree-sitter test` — **19/19 pass**, 0 failures:

1. simple sentence (`Hello world.`)
2. compound with `and`
3. compound with `but`
4. subordinate clause `because`
5. subordinate clause `although`
6. relative clause `that`
7. two sentences in one paragraph
8. two paragraphs separated by a blank line
9. simple SVO sentence (tier 3 dropped → generic clause)
10. abbreviation does not end the sentence (`Mr. Smith arrived.`)
11. initialism run stays one sentence (`The H.M.S. Beagle sailed.`)
12. unknown abbreviation with lowercase continuation (`The dept. said …`)
13. decimal number keeps its dot (`3.50`)
14. sentence still splits after an abbreviation (`Mr. Smith arrived. He stayed.`)
15. lowercase continuation across `e.g.`
16. abbreviation before a sentence-final mark (`Did Dr. Jones arrive?`)
17. spaced single-letter initial does not end the sentence (`J. Smith arrived.`)
18. pronoun `I` still ends a sentence (`It is I. You are next.`)
19. conjunction word at sentence start lexes as a word (`For home he left.`)

## Examples — parse results and wall time

All parse outputs are saved next to each source as `<file>.parse.txt`.

### pride_and_prejudice.txt — clean parse (no ERROR)
Text: `It is a truth universally acknowledged, that a single man in possession
of a good fortune, must be in want of a wife.`

- **Right:** one sentence/paragraph; `that` at `[0,40]-[0,44]` recognised as a
  `subordinator`; the rest of the sentence folded into a `subordinate_clause`.
  Commas are hidden and correctly absorbed.
- **Wrong:** none — no ERROR node; the greedy `subordinate_clause` is coarse but
  bracketing is self-consistent.

### origin_of_species.txt — one sentence, two apostrophe errors
Text: `When on board H.M.S. 'Beagle,' as naturalist, … of that continent.`

- **Right:** the whole paragraph parses as **one sentence** (previously four
  fragments): `H.M.S.` lexes as a single `dotted` token `[0,14]-[0,20]`, so
  its periods never reach the sentence-end logic. `When` is a `subordinator`;
  `and`/`that` act as operator/subordinator on the tail
  (`conjunction [0,140]-[0,143]`, `subordinator [0,214]-[0,218]`).
- **Wrong (lone apostrophes):** the word pattern `[A-Za-z]+('[A-Za-z]+)?` only
  allows an apostrophe *between* letters, so both apostrophes of `'Beagle,'`
  error out:
  ```
  (ERROR [0, 21] - [0, 22])     ; the opening "'"
  (ERROR [0, 29] - [0, 30])     ; the closing "'" (the inner comma is absorbed)
  ```

### garden_path.txt — clean parse (no ERROR), but no structural insight
Text: the three garden-path sentences (`The horse raced past the barn fell.`,
`The old man the boat.`, `The complex houses married and single soldiers and
their families.`).

- **Right:** sentence boundaries at each period (all three sentences stay in one
  paragraph since they are separated only by single newlines); `and` recognised
  as `conjunction` in the third sentence (`[2,27]-[2,30]`, `[2,47]-[2,50]`),
  which parses as `clause + and + clause + and + clause`.
- **Wrong:** the grammar cannot express the garden-path phenomenon. `The horse
  raced past the barn fell.` is a single flat `clause` of seven `word` tokens —
  there is no verb/noun structure, so the famous mis-parse ("raced past the
  barn" as a reduced relative clause) is neither detected nor even representable.
  `The old man the boat.` likewise parses as a flat clause. This is a direct
  consequence of tier 3 (SVO) being dropped: without subject/verb/object
  fields the grammar has no notion of how `fell` attaches.

### Wall time

`nix develop -c tree-sitter parse examples/<file>`:

- wall time ≈ **1.07–1.10 s** per example, dominated by `nix develop` shell
  startup (the parse itself is microseconds).
- The CLI only prints its own `Parse: X ms` footer for files with errors; the one
  it printed was `origin_of_species.txt  Parse: 0.09 ms  2690 bytes/ms`.
  The two clean files are equally tiny (~0.05 ms).

## Limitations / where the grammar breaks

1. Any word that isn't in the closed-class keyword lists is an opaque `word`; the
   grammar performs **no real syntactic analysis** — a "clause" is just a word
   run. It marks boundaries (paragraph/sentence/clause) and operators
   (conjunction/subordinator), nothing more.
2. **Abbreviations:** initialism runs, listed abbreviations, spaced
   single-letter initials, and unknown abbreviations before lowercase
   continuations no longer split sentences (stateful external scanner +
   `dotted`/`number` tokens). Residual: a genuine initial `I.` (`I. M.
   Pei`), unknown abbreviations before a capital-starting continuation,
   and ellipses.
3. **Lone apostrophes / quotes** at word edges (leading or trailing) error out.
4. **Sentence-final punctuation inside quoted material** is handled, but an
   opening quote before a word (like `'Beagle`) errors.
5. `subordinate_clause` is greedy and absorbs everything after its subordinator
   to the end of the sentence, so it can swallow what should be later independent
   clauses (e.g. the "we went out" after `Although it rained,` is folded into the
   `Although` clause).
6. A sentence must start with a `clause` or `subordinate_clause`; leading
   parentheticals or standalone punctuation that isn't covered will error.
7. Tier 3 (SVO fields) is intentionally absent; see the failure-mode write-up
   above.
