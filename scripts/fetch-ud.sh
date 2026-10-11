#!/usr/bin/env bash
# Fetch UD training corpora at pinned revisions for english-pos-train.
#
# The corpora stay OUT of the repo (see below for why) and land in a
# directory the trainer accepts as --corpus:
#
#   scripts/fetch-ud.sh --dir /tmp/ud ewt
#   cargo run --release -p english-pos-train -- --corpus /tmp/ud/ewt \
#     --iters 20 --min-count 1
#
# Presets (train = concatenation, dev/test = first treebank's):
#   ewt         EWT train/dev/test (the committed-weights setup)
#   ewt+gum     ablation: EWT + GUM train, EWT dev/test
#   ewt+lines   ablation: EWT + LinES train, EWT dev/test
#   all         ablation: EWT + GUM + LinES train, EWT dev/test
#   eg+lines    screen bundle: EWT + GUM + LinES train, EWT dev/test
#   eg+clean    screen bundle: EWT + GUM + GUMReddit + ParTUT train
#   eg+diverse  screen bundle: EWT + GUM + ESLSpok + Atis train
#   eg+child    screen bundle: EWT + GUM + CHILDES train (kept solo:
#               221k toks would dominate any bundle it joins)
#   eg+all      screen bundle: every train-capable bank below
#
# Screen bundles (R3-7 corpus expansion, see AGENTS.md): each new
# bank joins the shipped EWT+GUM base once, impact measured on the
# tagger screen against the EWT+GUM baseline, rejects need no
# attribution. `eg+all` follows only if every bundle passes.
#
# Why not vendored:
# - GUM is CC BY-NC-SA 4.0 (fiction/wikiHow portions prohibit
#   commercial and non-open-source use) and LinES is CC BY-NC-SA 4.0:
#   neither can ship in this MIT repo.
# - EWT is CC BY-SA 4.0: vendoring is possible with attribution, but
#   ShareAlike's reach over trained weights is murky and it is 15 MB
#   of bloat. Pinned fetch keeps training reproducible instead.
#
# Attribution (required by all three licenses when sharing the data):
# - EWT: UD_English-EWT contributors; CC BY-SA 4.0;
#   https://github.com/UniversalDependencies/UD_English-EWT
# - GUM: Georgetown GUM team (see https://corpling.uis.georgetown.edu/gum/
#   for annotators) plus the underlying text sources (Wikimedia CC-BY,
#   wikiHow CC-BY-NC-SA, SBC, OpenStax, public-domain speeches);
#   CC BY-NC-SA 4.0; https://github.com/UniversalDependencies/UD_English-GUM
# - LinES: UD_English-LinES contributors; CC BY-NC-SA 4.0;
#   https://github.com/UniversalDependencies/UD_English-LinES
# - ESLSpok: UD_English-ESLSpok contributors; CC BY-SA 4.0;
#   https://github.com/UniversalDependencies/UD_English-ESLSpok
# - GUMReddit: annotations CC BY 4.0 (underlying Reddit texts per
#   the GUM site); https://github.com/UniversalDependencies/UD_English-GUMReddit
# - ParTUT: CC BY-NC-SA 2.0;
#   https://github.com/UniversalDependencies/UD_English-ParTUT
# - Atis: UD_English-Atis contributors; CC BY-SA 4.0;
#   https://github.com/UniversalDependencies/UD_English-Atis
# - CHILDES: UD_English-CHILDES contributors; CC BY-SA 4.0;
#   https://github.com/UniversalDependencies/UD_English-CHILDES
# - GENTLE / Pronouns / LittlePrince / CTeTex (test-only evals):
#   CC BY-NC-SA 4.0 / CC BY-SA 4.0 / CC BY-SA 4.0 / CC BY-SA 4.0.
#
# Deliberately NOT fetched (verified 2026-10-11):
# - ESL: every FORM is `_` (FCE text stripped — needs a separate
#   FCE download + merge); mechanically unusable for word features.
# - PCEDT: stub repo (WSJ text is LDC-licensed, no redistribution).
# - UniDive: stub repo (no data files).
# - Code-switched (HIENCS/Miami/BUTR/TECT) and Old English
#   (Cairo/OEDT/TueCL): wrong language mix for an English tagger.
set -euo pipefail

DIR="/tmp/ud"
PRESET="ewt"
while [[ $# -gt 0 ]]; do
  case "$1" in
  --dir)
    DIR="$2"
    shift 2
    ;;
  ewt | ewt+gum | ewt+lines | all | eg+lines | eg+clean | eg+diverse | eg+child | eg+all)
    PRESET="$1"
    shift
    ;;
  *)
    echo "usage: fetch-ud.sh [--dir DIR] [ewt|ewt+gum|ewt+lines|all|eg+lines|eg+clean|eg+diverse|eg+child|eg+all]" >&2
    exit 2
    ;;
  esac
done

EWT_SHA="4a4d77f599ea53cc405f85d0cec4b2f14f81d42b"
GUM_SHA="1fe635509c649e376dfb449d528424ab78f4eaee"
LINES_SHA="f1e4f1d6d6dd03ee45176d6b6a13f694c7efab0"
ESLSPOK_SHA="3feb27b9759454c4cf02263b371963137bee0d2b"
GUMREDDIT_SHA="b23cca5d4f8732b35cbbec070bb2b1d7abe750e1"
PARTUT_SHA="5552572ac2c5aac1538e43edb7f7a8d2224f12de"
ATIS_SHA="3ba83a5ded2c2b6e69ea2fa862806a892ae62a53"
CHILDES_SHA="92d3cf4c7c567bdebfe412573fecb7ec93332c30"
GENTLE_SHA="fd7a1bfc82896e362c66f59492b5525940f52fa7"
PRONOUNS_SHA="6c5934b7330dd0302723178d266f8326bfa358df"
LITTLEPRINCE_SHA="60d7a34bcae4d3a09f11dd9fd47e743375e3b9c4"
CTETEX_SHA="3d2bda424dcdedb8889aeec9f81ee6401994f7f2"
# PUD has no train/dev splits (test only, 1,000 sents); it is an
# eval corpus, never training input — fetched to cache, no preset.
PUD_TAG="r2.18"

CACHE="$DIR/.cache"
mkdir -p "$CACHE"

fetch() { # repo sha remote_prefix local_prefix
  local repo="$1" sha="$2" remote="$3" local="$4"
  for split in train dev test; do
    local out="$CACHE/${local}-${split}.conllu"
    if [[ ! -f $out ]]; then
      curl -fSL -o "$out" \
        "https://raw.githubusercontent.com/UniversalDependencies/${repo}/${sha}/${remote}-${split}.conllu"
    fi
  done
}

fetch UD_English-EWT "$EWT_SHA" en_ewt-ud en_ewt-ud
fetch UD_English-GUM "$GUM_SHA" en_gum-ud en_gum-ud
fetch UD_English-LinES "$LINES_SHA" en_lines-ud en_lines-ud
fetch UD_English-ESLSpok "$ESLSPOK_SHA" en_eslspok-ud en_eslspok-ud
fetch UD_English-GUMReddit "$GUMREDDIT_SHA" en_gumreddit-ud en_gumreddit-ud
fetch UD_English-ParTUT "$PARTUT_SHA" en_partut-ud en_partut-ud
fetch UD_English-Atis "$ATIS_SHA" en_atis-ud en_atis-ud
fetch UD_English-CHILDES "$CHILDES_SHA" en_childes-ud en_childes-ud
# Test-only eval corpora (no train splits upstream): fetched to
# cache, no preset — same discipline as PUD below.
fetch_test() { # repo sha remote_file local_file
  local repo="$1" sha="$2" remote="$3" local="$4"
  local out="$CACHE/${local}"
  if [[ ! -f $out ]]; then
    curl -fSL -o "$out" \
      "https://raw.githubusercontent.com/UniversalDependencies/${repo}/${sha}/${remote}"
  fi
}
fetch_test UD_English-GENTLE "$GENTLE_SHA" en_gentle-ud-test.conllu en_gentle-ud-test.conllu
fetch_test UD_English-Pronouns "$PRONOUNS_SHA" en_pronouns-ud-test.conllu en_pronouns-ud-test.conllu
fetch_test UD_English-LittlePrince "$LITTLEPRINCE_SHA" en_littleprince-ud-test.conllu en_littleprince-ud-test.conllu
fetch_test UD_English-CTeTex "$CTETEX_SHA" en_ctetex-ud-test.conllu en_ctetex-ud-test.conllu
# PUD by release tag (immutable, same pinning discipline as SHAs).
curl -fSL -o "$CACHE/en_pud-ud-test.conllu" \
  "https://raw.githubusercontent.com/UniversalDependencies/UD_English-PUD/${PUD_TAG}/en_pud-ud-test.conllu"

OUT="$DIR/$PRESET"
mkdir -p "$OUT"
case "$PRESET" in
ewt)
  TRAINS="en_ewt-ud"
  EV="en_ewt-ud"
  ;;
ewt+gum)
  TRAINS="en_ewt-ud en_gum-ud"
  EV="en_ewt-ud"
  ;;
ewt+lines)
  TRAINS="en_ewt-ud en_lines-ud"
  EV="en_ewt-ud"
  ;;
all)
  TRAINS="en_ewt-ud en_gum-ud en_lines-ud"
  EV="en_ewt-ud"
  ;;
eg+lines)
  TRAINS="en_ewt-ud en_gum-ud en_lines-ud"
  EV="en_ewt-ud"
  ;;
eg+clean)
  TRAINS="en_ewt-ud en_gum-ud en_gumreddit-ud en_partut-ud"
  EV="en_ewt-ud"
  ;;
eg+diverse)
  TRAINS="en_ewt-ud en_gum-ud en_eslspok-ud en_atis-ud"
  EV="en_ewt-ud"
  ;;
eg+child)
  TRAINS="en_ewt-ud en_gum-ud en_childes-ud"
  EV="en_ewt-ud"
  ;;
eg+all)
  TRAINS="en_ewt-ud en_gum-ud en_lines-ud en_gumreddit-ud en_partut-ud en_eslspok-ud en_atis-ud en_childes-ud"
  EV="en_ewt-ud"
  ;;
esac

# Train split: concatenation (CoNLL-U sentences are blank-line
# separated, so plain cat preserves sentence boundaries).
: >"$OUT/en_ewt-ud-train.conllu"
for t in $TRAINS; do
  cat "$CACHE/${t}-train.conllu" >>"$OUT/en_ewt-ud-train.conllu"
done
cp "$CACHE/${EV}-dev.conllu" "$OUT/en_ewt-ud-dev.conllu"
cp "$CACHE/${EV}-test.conllu" "$OUT/en_ewt-ud-test.conllu"
echo "wrote $OUT (preset $PRESET)"
