# english-lint

Syntax-aware prose linting: Vale-class output (`path:line:col
[rule] message`) over grammar-backed rules a regex linter cannot
express. The pipeline runs once per document (parse → tag → beam4
parse → label); every rule reads the shared annotated document.

Rules (each with a 60-sentence hand-labeled eval and
pre-registered precision/recall bars):

| Rule | Shape | Eval | Check on Moby body |
|---|---|---|---|
| `syntax.passive` | finite be/get-passives via `nsubj:pass`/`aux:pass` | 0.917 / 0.733 | 3.0 ms (1,562 findings) |
| `syntax.nominalization` | light verb + deverbial noun via `obj`/`obl` | 0.912 / 0.939 | 8.1 ms (126 findings) |
| `syntax.sentence-length` | >30 pieces (grammar counts) | combined below | 0.4 ms (2,518 findings) |
| `syntax.clause-complexity` | ≥4 clauses or ≥2 subordinate | 0.846 / 1.000 | 0.5 ms (3,363 findings) |
| `syntax.weasel` | `very/really/extremely` + ADJ/ADV | 0.968 / 1.000 | 2.7 ms (249 findings) |
| `syntax.hedge` | `so/quite/rather` + ADJ/ADV | 0.941 / 0.533 (bar 0.50) | 2.5 ms (489 findings) |
| `syntax.vague-demonstrative` | sentence-initial `this/that` + unclear antecedent | 0.824 / 0.967 | 1.1 ms (93 findings) |
| `syntax.coord-scope` | ADJ NOUN and/or/but NOUN, same category | 0.844 / 0.900 | 0.8 ms (237 findings) |
| `syntax.negation-scope` | quantifier before `n't/not/never` | 0.968 / 1.000 | 4.0 ms (165 findings) |

POS/grammar rules (`sentence-length`, `clause-complexity`,
`weasel`, `hedge`) run on the shallow path — no parser weights,
keystroke-capable. Dependency rules need the Tier-1 weight pair
(see `english-dep` README). Check costs are best-of-5 on a
pre-annotated Moby body (full annotate itself: 8.0 s, dep-beam
bound): rules total ~23 ms, 0.3% of the pipeline — rule logic is
never the latency constraint, annotation is. Spelling and vocabulary are explicitly
out (not a spellchecker — syntax differentiation is the point);
markdown structure (tables, lists) is out of prose scope.

```console
cargo run -p english-lint -- file.txt...
```
