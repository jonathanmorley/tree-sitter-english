# english-chunk

Greedy phrase chunking over tagged English prose: CoNLL-2000
chunking as pattern (flat, non-recursive phrases), never new
noun/verb rules in `grammar.js` (Tier 3 showed why — word classes
and phrase structure do not belong in the grammar).

## Input

`tag_sentence` output: `Vec<(surface piece, Tag)>` pieces, UD-style
(contractions split, hidden punctuation excluded). The chunker never
sees the tree — only the tag stream. Contraction pieces (`do` +
`n't`, `ship` + `'s`) chunk by their own tags (`n't`/`'s` are PART
→ particle chunks), which is why this consumes pieces, not words.

## Tagset (UD-adapted CoNLL-2000)

| Chunk | Shape | Notes |
|---|---|---|
| Noun | `[DET]* [ADJ\|NUM]* nominal+, lone DET` | Attributive adjectives absorb (`green fields` is one chunk). A leading PRON continues only onto NOUN/NUM (`my substitute`, `we sailors` merge; `me Ishmael`, `you Starbuck` split as vocative/address — deliberate CoNLL deviation for dialogue prose). Lone DET covers pronominals (`all`) and stranding. |
| Verb | `AUX* VERB+`, or lone `AUX+` | Copula alone (`is`) is a verb chunk; `AUX` + `VERB`/`ADJ` splits after the AUX (`are` + `concerned`) per EWT-side participles. |
| Prep | `ADP` + nominal run (`DET/ADJ/NUM/NOUN/PROPN/PRON*`, no `ADV`) | Bare particles (`looked up`) are lone preps; degree/direction adverbs (`thence`) chunk separately, CoNLL-faithful; attachment is parse-level, out of scope. |
| Adverb | `ADV+` | |
| Adj | `ADJ+` | Predicative only in practice (attributives absorb into Noun). |
| Subord | `SCONJ` (single) | Flat like CoNLL SBAR heads: the clause body chunks normally after it. |
| Conj | `CCONJ+` | |
| Particle | `PART+` | Infinitive `to`, `n't`, possessive `'s`, `not`. |
| Interj | `INTJ+` | |
| Punct | `PUNCT+` | |
| Other | `X`/`SYM` runs | Foreign, symbols, unclassified. |

## Algorithm

Single left-to-right pass, O(n): at each position take the first
matching chunk in the priority order Punct, Subord, Conj, Particle,
Interj, Noun, Verb, Prep, Adverb, Adj, Other. Maximal runs within
the chunk (greedy). No regex-over-string (ReDoS), no backtracking
beyond one-token lookahead discipline — random access over the vec,
but each token is consumed exactly once, so keystroke-time cost is
linear (measured in `bench`; must hold the ~47 ms book budget
established for parse+tag).

Deliberately NOT here: nesting or attachment (PP-attach, relative
clauses stay flat), `of`-PP merge rules, multiword tries (see the
MWE follow-up in AGENTS.md), PTB patterns verbatim.
