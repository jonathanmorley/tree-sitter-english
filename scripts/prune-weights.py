"""Prune small-magnitude weights from an averaged tagger map.
Kept infrastructure (reproduces the committed pruned weights byte
for byte): entrywise |w| < EPS dropped, serde-compatible compact
encoding (whole numbers as ints), tagdict untouched.
Usage: prune-weights.py <in.json> <out.json> <eps>."""

import json
import sys

inp, outp, eps = sys.argv[1], sys.argv[2], float(sys.argv[3])
d = json.load(open(inp))
n_drop = n_keep = 0
out = {"tagdict": d["tagdict"], "weights": {}}
for fid, row in d["weights"].items():
    o = {}
    for code, v in row.items():
        x = float(v)
        if abs(x) < eps:
            n_drop += 1
            continue
        n_keep += 1
        o[code] = int(x) if x.is_integer() else x
    if o:
        out["weights"][fid] = o
json.dump(out, open(outp, "w"), separators=(",", ":"))
print(f"eps={eps}: keep={n_keep} drop={n_drop}")
