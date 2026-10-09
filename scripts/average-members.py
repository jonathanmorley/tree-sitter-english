"""Entrywise mean of member tagger weight JSONs (same math as the
`ensemble` example's average step, kept as the reproducible recipe
step: members from `ensemble.rs` → this script → prune-weights.py).
Deterministic: fixed member order, fixed summation order, compact
serde-compatible encoding (whole numbers as ints); tagdicts asserted
equal (order-independent by construction). Re-derivation is
md5-identical (verified). Usage:
average-members.py <out.json> <m1.json> ..."""

import json
import sys
from collections import defaultdict

outp, paths = sys.argv[1], sys.argv[2:]
vals = [json.load(open(p)) for p in paths]
d0 = vals[0]["tagdict"]
for v in vals[1:]:
    assert v["tagdict"] == d0, "tagdict mismatch"
k = len(vals)
acc = defaultdict(lambda: defaultdict(float))
for v in vals:
    for fid, row in v["weights"].items():
        e = acc[fid]
        for code, num in row.items():
            e[code] += float(num)
weights = {}
for fid, row in acc.items():
    o = {}
    for code, s in row.items():
        m = s / k
        if m != 0.0:
            o[code] = int(m) if float(m).is_integer() else m
    if o:
        weights[fid] = o
json.dump({"tagdict": d0, "weights": weights}, open(outp, "w"), separators=(",", ":"))
print("wrote", outp)
