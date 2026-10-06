#!/usr/bin/env bash
# Adjective-form backoff list for the english-pos correction layer.
#
# Extracts distinct word forms whose majority tag in the pinned EWT
# train split is ADJ (count >= 3), for the gated attr-adj rule: a
# NOUN-predicted word between DET and a nominal flips to ADJ only
# when the form is a known adjective (this is what separates
# `sharp`/`poor` from noun-noun compounds like `steel hull`,
# which killed the lexicon-free variant at 57%).
#
# Usage: scripts/adj-forms.sh [/tmp/ud/ewt/en_ewt-ud-train.conllu]
#   > crates/english-pos/lexicon/adjectives.txt
#
# Provenance: UD_English-EWT @ 4a4d77f5 (see scripts/fetch-ud.sh).
# Regenerate after any EWT pin change and re-measure (`--correct`).
set -euo pipefail
CORPUS="${1:-/tmp/ud/ewt/en_ewt-ud-train.conllu}"
LC_ALL=C awk -F'\t' '
  NF>=4 && $2 !~ /[-.]/ {
    w=tolower($2); t=$4; key=w SUBSEP t
    c[key]++; tot[w]++
    if (t=="ADJ") adj[w]++
  }
  END {
    for (w in tot)
      if (tot[w]>=3 && adj[w]>tot[w]/2 && w ~ /^[a-z][a-z-]*$/)
        print w
  }' "$CORPUS" | LC_ALL=C sort -u
