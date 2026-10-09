"""Entrywise mean or median of member tagger weight JSONs (same math
as the `ensemble` example's average step, kept as the reproducible
recipe step: members from `ensemble.rs` → this script →
prune-weights.py). Deterministic: fixed member order, fixed
summation order, compact serde-compatible encoding (whole numbers
as ints); tagdicts asserted equal (order-independent by
construction). Re-derivation is md5-identical (verified, both
modes). Median ignores outlier members like a vote does, in one
map (admitted 2026-10-09 over the mean: dev +102 / test +94 at
1.87 MB vs 4.53 MB). Missing codes count as 0.0 votes. Usage:
average-members.py [--median] <out.json> <m1.json> ..."""

import json
import sys
from collections import defaultdict

args = sys.argv[1:]
median = args[0] == "--median"
if median:
    args = args[1:]
outp, paths = args[0], args[1:]
vals = [json.load(open(p)) for p in paths]
d0 = vals[0]["tagdict"]
for v in vals[1:]:
    assert v["tagdict"] == d0, "tagdict mismatch"
k = len(vals)
if not median:
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
else:
    acc = defaultdict(lambda: defaultdict(list))
    for v in vals:
        for fid, row in v["weights"].items():
            e = acc[fid]
            for code, num in row.items():
                e[code].append(float(num))
    weights = {}
    for fid, row in acc.items():
        o = {}
        for code, xs in row.items():
            while len(xs) < k:
                xs.append(0.0)
            xs.sort()
            m = xs[k // 2]
            if m != 0.0:
                o[code] = int(m) if float(m).is_integer() else m
        if o:
            weights[fid] = o
json.dump({"tagdict": d0, "weights": weights}, open(outp, "w"), separators=(",", ":"))
print("wrote", outp)
