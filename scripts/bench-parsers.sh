#!/usr/bin/env bash
# Parser shootout rerun: MaltParser 1.9.2 (BSD license, jar lives in
# $WORK, never vendored — same shelf as TreeTagger) vs english-dep
# on EWT test. Regenerates every Malt number in the front-page
# parser table from scratch; ours rows are committed gates
# (english-dep-train depbench + ud release tests).
#
# Needs: nix (openjdk_headless, ad-hoc shell — no flake change),
# python3 (stdlib only), this repo checkout, EWT conllu files.
# Usage: EWT=/path/to/ewt WORK=/tmp/malt [PY=python3] scripts/bench-parsers.sh
set -euo pipefail
PY=${PY:-python3}
command -v "$PY" >/dev/null || {
  echo "bench-parsers needs a stdlib-only python3: PY=/path/to/python3 $0"
  exit 2
}
EWT=${EWT:-/tmp/opencode/ud/ewt}
WORK=${WORK:-/tmp/opencode/malt}
MALT_VER=maltparser-1.9.2
REPO=$(cd -- "$(dirname -- "$0")" && git rev-parse --show-toplevel)
mkdir -p "$WORK"
cd "$WORK"

if [ ! -f "$MALT_VER/maltparser-1.9.2.jar" ]; then
  curl -sL -o maltparser-1.9.2.tar.gz \
    https://maltparser.org/dist/maltparser-1.9.2.tar.gz
  tar xzf maltparser-1.9.2.tar.gz
fi
JAVA="nix shell nixpkgs#openjdk_headless --command java -Xmx4g -jar $MALT_VER/maltparser-1.9.2.jar"

# Lexical input: LEMMA/FEATS blanked, XPOS slot <- UPOS (gold or
# predicted). Column-mapping note: MaltParser's default features
# read POSTAG (= XPOS), NOT CPOSTAG — training with XPOS blanked
# yields a FORM-only model whose outputs are exactly
# tag-invariant (measured: 0/25094 tokens flip between gold and
# predicted tags). The XPOS <- UPOS mapping is load-bearing.
# Splice hygiene: strip the tag field (a stray newline corrupts
# every downstream column; caught by the POSTAG-symbol error).
if [ ! -f ewt-test.tokens ]; then
  "$PY" - "$EWT" <<"PYEOF"
import sys
ewt = sys.argv[1]


def load(path):
  sents, cur = [], []
  for line in open(path):
    s = line.strip()
    if not s:
      if cur:
        sents.append(cur)
        cur = []
      continue
    if s.startswith("#"):
      continue
    c = s.split("\t")
    if len(c) < 10 or "-" in c[0] or "." in c[0]:
      continue
    cur.append(c)
  if cur:
    sents.append(cur)
  return sents


train = load(f"{ewt}/en_ewt-ud-train.conllu")
test = load(f"{ewt}/en_ewt-ud-test.conllu")
with open("ewt-train-lex2.conllu", "w") as f:
  for s in train:
    for c in s:
      c = list(c)
      c[2] = "_"
      c[5] = "_"
      c[4] = c[3]
      f.write("\t".join(c) + "\n")
    f.write("\n")
with open("ewt-test-lex2-gold.conllu", "w") as f:
  for s in test:
    for c in s:
      c = list(c)
      c[2] = "_"
      c[5] = "_"
      c[4] = c[3]
      f.write("\t".join(c) + "\n")
    f.write("\n")
with open("ewt-test.tokens", "w") as f:
  for s in test:
    for c in s:
      f.write(c[1] + "\n")
    f.write("\n")
print(f"lex2 built: {len(train)} train / {len(test)} test sents")
PYEOF
fi

# Greedy predicted tags (matches the dep eval's +tagger regime, which
# uses Model::tag, not the production path). nix develop runs in the
# repo checkout (this script's home), not $WORK.
if [ ! -f ewt-test.predg ]; then
  nix develop "$REPO" --command bash -c \
    'cd "$0" || exit 1; export PATH=$HOME/.cargo/bin:$PATH; export RUSTUP_TOOLCHAIN=stable;
     cargo run -q --release -p english-pos --example tag_tokens -- \
     "$1/ewt-test.tokens" --sentences' "$REPO" "$WORK" >ewt-test.predg
fi

if [ ! -f ewt-test-lex2-predg.conllu ]; then
  "$PY" - <<"PYEOF"
pred = [l.strip().split("\t")[1] for l in open("ewt-test.predg") if l.strip()]
out, i = [], 0
for line in open("ewt-test-lex2-gold.conllu"):
  s = line.strip()
  if not s or s.startswith("#"):
    out.append(line)
    continue
  cols = s.split("\t")
  cols[3] = pred[i]
  cols[4] = pred[i]
  i += 1
  out.append("\t".join(cols) + "\n")
assert i == len(pred), (i, len(pred))
open("ewt-test-lex2-predg.conllu", "w").writelines(out)
print(f"spliced {i} greedy pred tags")
PYEOF
fi

$JAVA -c ewtfull -i "$EWT/en_ewt-ud-train.conllu" \
  -m learn -l liblinear -a nivreeager 2>&1 | tail -1
$JAVA -c ewtlex2 -i ewt-train-lex2.conllu \
  -m learn -l liblinear -a nivreeager 2>&1 | tail -1
$JAVA -c ewtfull -i "$EWT/en_ewt-ud-test.conllu" \
  -o malt-full-gold.conllu -m parse 2>&1 | tail -1
$JAVA -c ewtlex2 -i ewt-test-lex2-gold.conllu \
  -o malt-lex2-gold.conllu -m parse 2>&1 | tail -1
$JAVA -c ewtlex2 -i ewt-test-lex2-predg.conllu \
  -o malt-lex2-predg.conllu -m parse 2>&1 | tail -1

# Score: UAS + full-label LAS over all integer-ID tokens (punct
# included — the repo's own denominator; 25,094 test tokens both
# sides, multiword/empty nodes skipped both sides).
GOLD="$EWT/en_ewt-ud-test.conllu" "$PY" - <<"PYEOF"
import os


def load(path):
  sents, cur = [], []
  for line in open(path):
    line = line.strip()
    if not line:
      if cur:
        sents.append(cur)
        cur = []
      continue
    if line.startswith("#"):
      continue
    c = line.split("\t")
    if len(c) < 8 or "-" in c[0] or "." in c[0]:
      continue
    cur.append((c[1], c[6], c[7]))
  if cur:
    sents.append(cur)
  return sents


gold = load(os.environ["GOLD"])
for name in ["malt-full-gold", "malt-lex2-gold", "malt-lex2-predg"]:
  pred = load(f"{name}.conllu")
  assert len(pred) == len(gold), (name, len(pred), len(gold))
  ok_u = ok_l = tot = 0
  for g, p in zip(gold, pred):
    assert len(g) == len(p), (len(g), len(p))
    for (gf, gh, gr), (pf, ph, pr) in zip(g, p):
      assert gf == pf, (gf, pf)
      tot += 1
      if ph == gh:
        ok_u += 1
        if pr == gr:
          ok_l += 1
  print(f"{name}: UAS {ok_u}/{tot} = {ok_u/tot:.4f}  LAS {ok_l}/{tot} = {ok_l/tot:.4f}")
PYEOF
