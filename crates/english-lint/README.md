# english-lint

Syntax-aware prose linting: Vale-class output (`path:line:col
[rule] message`) over grammar-backed rules a regex linter cannot
express. The pipeline runs once per document (parse → tag → beam4
parse → label); every rule reads the shared annotated document.

Rules (each with a 60-sentence hand-labeled eval and
pre-registered precision/recall bars):

| Rule | Shape | Eval |
|---|---|---|
| `syntax.passive` | finite be/get-passives via `nsubj:pass`/`aux:pass` | 0.917 / 0.733 |
| `syntax.nominalization` | light verb + deverbial noun via `obj`/`obl` | 0.912 / 0.939 |
| `syntax.sentence-length` | >30 pieces (grammar counts) | combined below |
| `syntax.clause-complexity` | ≥4 clauses or ≥2 subordinate | 0.846 / 1.000 |
| `syntax.weasel` | `very/really/extremely` + ADJ/ADV | 0.968 / 1.000 |
| `syntax.hedge` | `so/quite/rather` + ADJ/ADV | 0.941 / 0.533 (bar 0.50) |

POS/grammar rules (`sentence-length`, `clause-complexity`,
`weasel`, `hedge`) run on the shallow path — no parser weights,
keystroke-capable. Dependency rules need the Tier-1 weight pair
(see `english-dep` README). Spelling and vocabulary are explicitly
out (not a spellchecker — syntax differentiation is the point);
markdown structure (tables, lists) is out of prose scope.

```console
cargo run -p english-lint -- file.txt...
```
