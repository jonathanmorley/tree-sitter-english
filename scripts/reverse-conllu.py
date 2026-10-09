"""Reverse token order per sentence in a CoNLL-U file (mirror-decoder
training input: a model trained on reversed sentences with the same
templates learns original next-word/next-tags in its prev-word/prev-tag
slots — item 4, measured and rejected 2026-10-09).

Original `#` comments are dropped; each sentence is emitted with a
fresh `# sent_id = rev-<name>-<i>` line, token lines reversed,
blank-line separated. Multiword/empty-node lines are kept (the trainer
skips them the same way either direction).

Regenerates the item-4 inputs byte-identically (verified by diff):
  reverse-conllu.py <ud/ewt/en_ewt-ud-train.conllu> <out> train
  ... dev, test ...
  cargo run --release -p english-pos-train -- \
    --corpus <revdir> --iters 20 --min-count 1
yields mirror md5 `50be014f0c2b67001c5bd76c4620f886` (1.9 MB,
dev 92.29 / test 92.81 greedy on the reversed splits, i.e. mapped
back — the 2026-10-09 run record; retrain determinism inherits
from the trainer's fixed-order passes). Artifact stays /tmp-ephemeral (Tier-1 pair never vendored);
this script is the kept half of the recipe.

Usage: reverse-conllu.py <in.conllu> <out.conllu> <name>
"""

import sys

src, dst, name = sys.argv[1], sys.argv[2], sys.argv[3]
sents, cur = [], []
for line in open(src):
    if line.startswith("#"):
        continue
    if line.strip() == "":
        if cur:
            sents.append(cur)
            cur = []
        continue
    cur.append(line)
if cur:
    sents.append(cur)
n_tok = 0
with open(dst, "w") as f:
    for i, toks in enumerate(sents):
        f.write(f"# sent_id = rev-{name}-{i}\n")
        for t in reversed(toks):
            f.write(t)
        f.write("\n")
        n_tok += len(toks)
print(f"{name} sents: {len(sents)} toks: {n_tok} -> {dst}")
