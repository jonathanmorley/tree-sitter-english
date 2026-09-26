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
set -euo pipefail

DIR="/tmp/ud"
PRESET="ewt"
while [[ $# -gt 0 ]]; do
  case "$1" in
  --dir)
    DIR="$2"
    shift 2
    ;;
  ewt | ewt+gum | ewt+lines | all)
    PRESET="$1"
    shift
    ;;
  *)
    echo "usage: fetch-ud.sh [--dir DIR] [ewt|ewt+gum|ewt+lines|all]" >&2
    exit 2
    ;;
  esac
done

EWT_SHA="4a4d77f599ea53cc405f85d0cec4b2f14f81d42b"
GUM_SHA="1fe635509c649e376dfb449d528424ab78f4eaee"
LINES_SHA="f1e4f1d6d6dd03ee4e5176d6b6a13f694c7efab0"

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
