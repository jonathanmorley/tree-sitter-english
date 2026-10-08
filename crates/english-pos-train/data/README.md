# Test splits vendored for committed regression gates.

- `en_ewt-ud-test.conllu`: UD English-EWT test, CC BY-SA 4.0,
  UD_English-EWT contributors
  (https://github.com/UniversalDependencies/UD_English-EWT),
  fetched at pinned revision `4a4d77f5` (see `scripts/fetch-ud.sh`).
- `en_pud-ud-test.conllu`: UD English-PUD test (r2.18), CC BY-SA
  4.0, UD contributors
  (https://github.com/UniversalDependencies/UD_English-PUD).

Both ship under CC BY-SA 4.0 with this attribution. GUM/LinES
test files are CC BY-NC-SA and can never ship here (fetched to
/tmp like training data). The `moby-oracle-*.conllu` files are
hand-labeled in-domain oracle data (01/02 in training; 03–05
measured, rejected, kept).

- `spark-ewt-test-tags.tsv`: Muse Spark 1.3 UPOS tags for the same
  pinned EWT test, single API run 2026-10-08 (10 chunk workers, strict
  TSV protocol). Tags only — zero EWT surface text, so no
  redistribution question. 2076/2077 sentences scored (sid 1907
  excluded: tagger merged trailing `./:)` into one tag, 17 vs 18
  words); exact 93.04 (23,331/25,076), coarse-12 94.52
  (Petrov Table 1, same projection as `bench-taggers.py`). Single-run,
  non-rerunnable by construction (no harness leg); possible train-data
  contamination unknown — not comparable head-to-head. Scorer was a
  temporary `/tmp` probe (word-echo + tag-inventory validation,
  drifted sentences excluded), since deleted.
