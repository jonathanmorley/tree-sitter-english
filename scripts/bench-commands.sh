#!/usr/bin/env bash
# Outer wall-clock comparison via hyperfine (DEV-TIME ONLY).
#
# bench-taggers.py owns accuracy + in-process tagger-only breakdowns
# (spawn excluded, stage splits); this script owns command-level
# wall clock with statistics (--warmup, --runs, --export-markdown
# straight into the shootout record). The two agree at book scale
# and diverge on tiny inputs where spawn dominates — that is
# expected, not a bug (see bench-taggers.py header).
#
# Usage (inside `nix develop`, hyperfine now in the devShell):
#   scripts/bench-commands.sh --moby /tmp/moby.txt [--out /tmp/hf.md]
#
# Optional competitor commands are appended only when their
# launchers exist in the env, mirroring bench-taggers.py skips:
#   BENCH_VENV=/tmp/opencode/benchvenv  (python with nltk/rdr/spacy)
#   TREETAGGER_BIN + TREETAGGER_PARAMS  (binary + english.par)
#
# Moby words file: whitespace-split body from CHAPTER 1 onward
# (same definition bench-taggers.py uses; shell approximation).
set -euo pipefail

MOBY=""
OUT="/tmp/hyperfine.md"
while [[ $# -gt 0 ]]; do
  case "$1" in
  --moby)
    MOBY="$2"
    shift 2
    ;;
  --out)
    OUT="$2"
    shift 2
    ;;
  *)
    echo "usage: bench-commands.sh --moby FILE [--out FILE]" >&2
    exit 2
    ;;
  esac
done
[[ -n $MOBY ]] || {
  echo "--moby required" >&2
  exit 2
}

command -v hyperfine >/dev/null || {
  echo "hyperfine missing (nix develop should provide it)" >&2
  exit 1
}

# Release binaries once: hyperfine must time the binary, never the build.
cargo build --quiet --release -p english-pos --example bench --example tag_tokens
BENCH_BIN="target/release/examples/bench"
TAGS_BIN="target/release/examples/tag_tokens"

WORDS="$(mktemp -t moby-words.XXXXXX)"
awk '/CHAPTER 1\. Loomings\./{f=1} f' "$MOBY" | grep -o '[^[:space:]]*' >"$WORDS"
echo "words: $(wc -l <"$WORDS") from $MOBY"

CMDS=(--command-name "ours end-to-end" "$BENCH_BIN $MOBY 5"
  --command-name "ours tag_tokens" "$TAGS_BIN $WORDS")

if [[ -n ${BENCH_VENV:-} && -x $BENCH_VENV/bin/python ]]; then
  PY="$BENCH_VENV/bin/python"
  export NLTK_DATA="${NLTK_DATA:-/tmp/opencode/nltk_data}"
  $PY -c "import nltk.tag" 2>/dev/null &&
    CMDS+=(--command-name "nltk perceptron"
      "$PY -c \"from nltk.tag import PerceptronTagger; t=PerceptronTagger(); t.tag([l.strip() for l in open('$WORDS')])\"")
  $PY -c "import RDRPOSTagger" 2>/dev/null &&
    [[ -n ${RDR_MODEL:-} ]] &&
    CMDS+=(--command-name "rdr"
      "$PY -c \"from RDRPOSTagger.pSCRDRtagger import RDRPOSTagger; r=RDRPOSTagger(); r.loadFromFile('$RDR_MODEL'); r.tagRawSentence(open('$WORDS').read().replace(chr(10),' '))\"")
  $PY -c "import spacy" 2>/dev/null &&
    $PY -c "import spacy; spacy.load('en_core_web_sm')" 2>/dev/null &&
    CMDS+=(--command-name "spacy sm"
      "$PY -c \"import spacy; n=spacy.load('en_core_web_sm'); list(n.pipe([open('$WORDS').read()], batch_size=1000))\"")
else
  echo "BENCH_VENV unset: competitor legs skipped (see bench-taggers.py)"
fi

if [[ -n ${TREETAGGER_BIN:-} && -n ${TREETAGGER_PARAMS:-} ]]; then
  TT_IN="$(mktemp -t tt-in.XXXXXX)"
  cp "$WORDS" "$TT_IN"
  CMDS+=(--command-name "treetagger"
    "$TREETAGGER_BIN -token -lemma $TREETAGGER_PARAMS $TT_IN")
fi

hyperfine --warmup 2 --runs 5 --export-markdown "$OUT" "${CMDS[@]}"
echo "wrote $OUT"
