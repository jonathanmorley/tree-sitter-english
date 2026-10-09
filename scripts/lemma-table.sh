#!/usr/bin/env bash
# Lemma+FEATS lookup tables for parser features (unblocks the
# Malt feats-input gap: LEMMA and morphological attributes as
# offline tables, same shelf as `english-pos/lexicon/verbs.txt`).
#
# Extracts per-(form, UPOS) majority lemma + majority FEATS from the
# pinned EWT train split. Keying includes the tag because lemmas
# are tag-dependent (`rose` NOUN->`rose` vs VERB->`rise`); the
# parser already carries predicted tags at every template
# position, so lookup needs no new runtime source (this is what
# un-blocks LEMMA/FEATS: the tagger predicts UPOS only, the table
# supplies the rest, zero-dep deterministic lookup).
#
# Usage: scripts/lemma-table.sh [/tmp/ud/ewt/en_ewt-ud-train.conllu]
#   > /tmp/opencode/lemma-table.tsv
# Columns: form-lower, UPOS, lemma, feats-or-_, count, total.
# Forms with no majority (tie) still emit (count/total document
# the margin); consumers decide the bar.
#
# Provenance: UD_English-EWT @ 4a4d77f5 (see scripts/fetch-ud.sh),
# CC BY-SA 4.0 — attribution rides along if vendored.
set -euo pipefail
CORPUS="${1:-/tmp/ud/ewt/en_ewt-ud-train.conllu}"
LC_ALL=C awk -F'\t' '
  NF >= 6 && $1 ~ /^[0-9]+$/ {
    form = tolower($2); tag = $4; lemma = $3; feats = ($6 == "" ? "_" : $6);
    if (form ~ /^[a-z]/) {
      key = form "\t" tag;
      tot[key]++;
      lk = key "\t" lemma; lc[lk]++;
      if (lc[lk] > bestn[key]) { bestn[key] = lc[lk]; bestl[key] = lemma; }
      fk = key "\t" feats; fc[fk]++;
      if (fc[fk] > bestfn[key]) { bestfn[key] = fc[fk]; bestf[key] = feats; }
    }
  }
  END { for (k in tot) print k "\t" bestl[k] "\t" bestf[k] "\t" bestn[k] "\t" tot[k]; }
' "$CORPUS" | LC_ALL=C sort
