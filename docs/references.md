# References

Background reading for the methods used (and deliberately not used)
in this repo. Each entry notes where it applies. Prefer these over
re-deriving: the queued AGENTS.md items cite them by name.

## POS tagging

- Brill (1992): "A simple rule-based part of speech tagger", Proc.
  ANLP. The original transformation-based tagger. Applies to:
  `english-pos/src/correction.rs` (post-pass shape only — our rules
  are hand-written and margin-gated, not induced).
- Brill (1995): "Transformation-based error-driven learning and
  natural language processing: A case study in part-of-speech
  tagging", Computational Linguistics 21(4),
  <https://aclanthology.org/J95-1002/>. Rule induction + admission
  (net error reduction); the template for a future learning loop.
- Collins (2002): "Discriminative training methods for hidden Markov
  models", Proc. EMNLP, <https://aclanthology.org/W02-1001/>.
  The perceptron tagger (`Model::train`); including why averaging is
  standard — and why this repo deliberately skips it (see
  `english-pos-train` README for the measurement).
- Brants (2000): "TnT: A statistical part-of-speech tagger", Proc.
  ANLP, <https://aclanthology.org/A00-2023/>. Trigram tagger with
  suffix backoff for unknowns; the OOV baseline our perceptron
  affixes replace. TnT's seen/OOV-split reporting is the model for
  extending `verify` (AGENTS.md Brill item).
- Schmid (1994): "Probabilistic part-of-speech tagging using
  decision trees", Proc. NeMLaP,
  <https://www.cis.uni-muenchen.de/~schmid/tools/TreeTagger/>.
  Transition probabilities from a decision tree (variable context
  where data supports it) + lexicon + suffix/capitalization tree
  for unknowns + lemma output. Applies to: the queued OOV
  decision-list and lemmatizer items (margin-gated, never flat
  features), and as a `bench-taggers.py` coarse-12 competitor
  (research license — binary + `.par` stay out of the repo).

## External systems (benchmarked, never vendored)

- NLTK averaged perceptron (WSJ-trained, Penn tags) + Punkt
  sentence tokenizer: out-of-domain accuracy floor and the
  differential segmentation oracle (`scripts/sent-diff.py`).
- RDRPOSTagger (UPOS-EWT model, same tagset+domain): most
  accurate lightweight system measured (+0.25 over greedy) and
  most parameter-efficient (0.38 MB), but Python-only at 1/41
  the speed — offline competitor only.
- spaCy `en_core_web_sm`: second differential oracle (parse
  sentences) and exact-UPOS competitor on the aligned subset.
- Transformers (BERT-family, ~97-98% UPOS): oracle labelers
  only, distilled into the greedy model — never the keystroke
  path (50–1000× slower, 7–220× larger).
- Rerun: `python3 scripts/bench-taggers.py --conllu
  /tmp/ud/ewt/en_ewt-ud-test.conllu --moby /tmp/moby.txt`
  (inside `nix develop` for the ours leg; every competitor
  missing from the env is skipped with an install hint).
  Wall clock: `scripts/bench-commands.sh --moby /tmp/moby.txt`
  (hyperfine, in the devShell; see its header for the
  `BENCH_VENV`/`TREETAGGER_*` opt-ins).

## Segmentation and tokenization

- Kiss & Strunk (2006): "Unsupervised multilingual sentence boundary
  detection", Computational Linguistics 32(4),
  <https://aclanthology.org/J06-4003/>. The Punkt algorithm; source
  design for the offline abbreviation harvest
  (`scripts/fetch-ud.sh` era tooling, never runtime tables).
- Bird, Klein & Loper (2009): Natural Language Processing with
  Python, <https://www.nltk.org/book/>. NLTK implementations
  (`brill.py`, `punkt.py`, `tnt.py`) are cited by file and line in
  the AGENTS.md queued items — algorithm references only, never
  imported (license + dependency hygiene).

## Frameworks and data

- Universal Dependencies guidelines, <https://universaldependencies.org/>
  (universal pages plus `en`-specific ones). Authority for
  `CANONICAL.md` harmonization calls in `english-pos-train`.
- Tree-sitter, <https://tree-sitter.github.io/>. Parser generator
  and incremental runtime behind `grammar.js` and `Document::update`.
- Zeldes (2017): "The GUM Corpus: Creating Multilayer Resources in
  the Classroom", Language Resources and Evaluation 51(3),
  doi:[10.1007/s10579-016-9343-x](https://doi.org/10.1007/s10579-016-9343-x).
  GUM provenance, genre list, and license (CC BY-NC-SA).
- Ahrenberg (2007): "LinES: An English–Swedish Parallel Treebank",
  Proc. NODALIDA; and (2015) "Converting an English-Swedish Parallel
  Treebank to Universal Dependencies", Proc. DepLing.
  LinES provenance (auto-converted UPOS, partial review — the reason
  its divergences read as mapping artifacts).
- Honnibal (2013): "A good part-of-speech tagger in about 200
  lines of Python", Explosion blog,
  <https://explosion.ai/blog/part-of-speech-pos-tagger-in-python>.
  Averaged-perceptron recipe: two tags of history, tagdict
  fast-path, case-frequency (not case-feature) advice, greedy
  decoding as the default, train-with-guessed-history. Applies
  to the three queued probes in the AGENTS.md harvest item
  (history exposure, tagdict fast-path, case-frequency
  backoff); the averaging half is already measured-and-rejected
  for tagging here (33% vs 88%), admitted for parsing (+7.1).
