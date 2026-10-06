#!/usr/bin/env bash
# Verb-lemma backoff list for the english-pos correction layer.
#
# Extracts distinct VERB lemmas (UD lemma column) from the pinned EWT
# train split, for the gated s-verb-lex / imperative-lex rules: a
# NOUN-predicted `-s` word flips to VERB only when its stem is a known
# verb base form (this is what separates `glitters` from `theories`
# and `status`, which killed the morphology-only rule).
#
# Usage: scripts/verb-lemmas.sh [/tmp/ud/ewt/en_ewt-ud-train.conllu]
#   [MINCOUNT=1] > crates/english-pos/lexicon/verbs.txt
#
# Provenance: UD_English-EWT @ 4a4d77f5 (see scripts/fetch-ud.sh).
# Regenerate after any EWT pin change and re-measure (`--correct`).
set -euo pipefail
CORPUS="${1:-/tmp/ud/ewt/en_ewt-ud-train.conllu}"
MINCOUNT="${2:-1}"
LC_ALL=C awk -F'\t' -v mc="$MINCOUNT" '$4=="VERB" {c[tolower($3)]++} END {for (k in c) if (c[k]>=mc && k ~ /^[a-z][a-z-]*$/) print k}' "$CORPUS" | LC_ALL=C sort -u
