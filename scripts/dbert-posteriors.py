"""DBERT UPOS posteriors for offline oracle probes (soft-distillation
Stage 0, item 7). Kept as PROCEDURE, not bytes: the 34 MB train
posterior file stays /tmp-ephemeral (it carries EWT surface text,
which this repo never vendors — same tags-only discipline as the
Spark row — and its item is closed). Regeneration is what ships.

Backend is transformers-torch fp32 on the pinned model id (no
/tmp-only ONNX export chain involved; matches the kept
`bench-taggers.py --distilbert` leg). First-subword alignment,
batches pad to longest, distributions rounded to 6dp (1e-6 noise,
4 orders below any update scale). Verified: evals mode is
argmax-identical with the Stage-0 ORT record on 2079 sweep
tokens with entropy within 4e-5.

Usage:
  dbert-posteriors.py --words <blocks.txt> <out.jsonl>
    (one word per line, blank-separated sentences; e.g. eval sets)
  dbert-posteriors.py --conllu <in.conllu> <out.jsonl> [start] [count]
    (UD sentences; slice args support foreground chunking —
    background jobs die between turns here, so run slices that
    finish inside one turn. Full EWT train ≈ 12 min.)
Output: one JSON object per line: {"w": [...], "q": [[17 floats]...]}
in input order (consumers assert word alignment, fail loudly).
"""

import json
import sys

import numpy as np
import torch
from transformers import AutoModelForTokenClassification, AutoTokenizer

REPO = "Basengalenga/destilbert-part-of-speech-partial-fine-tuning"
LABELS = [
    "ADJ", "ADP", "ADV", "AUX", "CCONJ", "DET", "INTJ", "NOUN",
    "NUM", "PART", "PRON", "PROPN", "PUNCT", "SCONJ", "SYM",
    "VERB", "X",
]


def read_blocks(path):
    out, cur = [], []
    for line in open(path):
        s = line.rstrip("\n")
        if not s.strip():
            if cur:
                out.append(cur)
                cur = []
            continue
        cur.append(s)
    if cur:
        out.append(cur)
    return out


def read_conllu(path):
    sents = []
    words = []
    for line in open(path):
        s = line.strip()
        if not s or s.startswith("#"):
            if words:
                sents.append(words)
                words = []
            continue
        c = s.split("\t")
        if len(c) < 5 or "-" in c[0] or "." in c[0]:
            continue
        words.append(c[1])
    if words:
        sents.append(words)
    return sents


@torch.no_grad()
def posteriors(tok, model, sents, out):
    B = 64
    done = 0
    for b in range(0, len(sents), B):
        chunk = sents[b:b + B]
        enc = tok(
            chunk, is_split_into_words=True, return_tensors="pt",
            padding="longest", truncation=True, max_length=512,
        )
        enc.pop("token_type_ids", None)
        logits = model(**enc).logits
        for k, w in enumerate(chunk):
            wid = enc.word_ids(k)
            if hasattr(wid, "tolist"):
                wid = wid.tolist()
            order = []
            seen = set()
            for j, wi in enumerate(wid):
                if wi is not None and wi not in seen:
                    seen.add(wi)
                    order.append(j)
            assert len(order) == len(w), (k, len(order), len(w))
            z = logits[k][order].double().numpy()
            z -= z.max(axis=1, keepdims=True)
            ex = np.exp(z)
            q = ex / ex.sum(axis=1, keepdims=True)
            qs = [[round(float(v), 6) for v in row] for row in q]
            out.write(json.dumps({"w": w, "q": qs}) + "\n")
        done += len(chunk)
        print(f"  {done}/{len(sents)}", flush=True)


def main():
    mode = sys.argv[1]
    if mode == "--words":
        sents = read_blocks(sys.argv[2])
        outp = sys.argv[3]
        start, count = 0, len(sents)
    elif mode == "--conllu":
        alls = read_conllu(sys.argv[2])
        outp = sys.argv[3]
        start = int(sys.argv[4]) if len(sys.argv) > 4 else 0
        count = int(sys.argv[5]) if len(sys.argv) > 5 else len(alls)
        sents = alls[start:start + count]
    else:
        sys.exit("usage: dbert-posteriors.py (--words <blocks> | --conllu <conllu>) <out> [start] [count]")
    print(f"sents: {len(sents)}", flush=True)
    tok = AutoTokenizer.from_pretrained(REPO)
    model = AutoModelForTokenClassification.from_pretrained(REPO).eval()
    assert [model.config.id2label[i] for i in range(17)] == LABELS, "head must be UPOS-direct"
    with open(outp, "w" if start == 0 else "a") as out:
        posteriors(tok, model, sents, out)
    print(f"wrote {outp}", flush=True)


main()
