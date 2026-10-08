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
