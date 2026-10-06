#!/usr/bin/env python3
"""Word-class clustering for the tagger's cluster backoff (DEV-TIME ONLY).

Trains skip-gram embeddings (gensim) on public-domain book prose,
k-means them (numpy Lloyd's, fixed seed) into 256 classes, and emits
a word→u8 map plus a purity report. The map ships in
`crates/english-pos/lexicon/clusters.txt`; the tagger consumes it as
ONE dense feature (cluster id), never as labels.

Corpus discipline: PUBLIC-DOMAIN books only (Moby-Dick, Austen P&P,
Doyle Adventures, Stevenson TI — same /tmp texts as the sweep), so
the map carries zero UD licensing weight (unlike EWT-derived lists).
Purity gate: for map words present in EWT train, the report shows
per-cluster majority-tag share — below ~0.6 the map is noise and
must not be wired in (saves a retrain).

Determinism: gensim seed + workers=1, numpy seed; reruns are
identical (verify by md5 before committing the map).

Usage:
  oracle-python scripts/cluster-books.py <file>... [--min-count 5] [--clusters 256] [--out clusters.txt]
"""

import os
import re
import sys

WORD = re.compile(r"[a-z]+")


def read_sentences(paths):
    sents = []
    for path in paths:
        text = open(path, encoding="utf-8").read().lower()
        for chunk in re.split(r"[.!?]+", text):
            toks = WORD.findall(chunk)
            if len(toks) >= 3:
                sents.append(toks)
    return sents


def main():
    argv = sys.argv[1:]
    args = []
    skip_next = False
    for a in argv:
        if skip_next:
            skip_next = False
            continue
        if a in ("--min-count", "--clusters", "--out", "--ud"):
            skip_next = True
            continue
        if a.startswith("--"):
            continue
        args.append(a)
    _val = lambda flag, default: (
        argv[argv.index(flag) + 1] if flag in argv else default
    )
    min_count = int(_val("--min-count", 5))
    k = int(_val("--clusters", 256))
    out = _val("--out", None)

    import numpy as np
    from gensim.models import Word2Vec

    sents = read_sentences(args)
    ntok = sum(len(s) for s in sents)
    print(f"# sentences={len(sents)} tokens={ntok} min_count={min_count} k={k}")
    model = Word2Vec(
        sentences=sents, vector_size=50, window=5, min_count=min_count,
        workers=1, epochs=10, seed=42, sg=1, negative=5,
    )
    vocab = sorted(model.wv.key_to_index)
    print(f"# vocab={len(vocab)}")
    X = np.stack([model.wv[w] for w in vocab]).astype(float)
    Xn = X / np.linalg.norm(X, axis=1, keepdims=True)

    rng = np.random.default_rng(42)
    centers = Xn[rng.choice(len(Xn), size=k, replace=False)]
    assign = np.zeros(len(Xn), dtype=int)
    for it in range(60):
        d = ((Xn[:, None, :] - centers[None, :, :]) ** 2).sum(axis=2)
        new = d.argmin(axis=1)
        if (new == assign).all():
            print(f"# k-means converged at iter {it}")
            break
        assign = new
        for c in range(k):
            members = Xn[assign == c]
            if len(members):
                center = members.mean(axis=0)
                centers[c] = center / np.linalg.norm(center)
    else:
        print("# k-means hit iter cap (60)")

    labels = {w: int(c) for w, c in zip(vocab, assign)}
    sizes = {}
    for c in assign:
        sizes[int(c)] = sizes.get(int(c), 0) + 1
    used = sorted(sizes)
    print(f"# clusters used: {len(used)}/{k}; size p50={sorted(sizes.values())[len(sizes)//2]}")

    # Purity gate against EWT train tags (needs --ud <dir> with the
    # fetched train split; skipped when absent).
    if "--ud" in sys.argv:
        ud = sys.argv[sys.argv.index("--ud") + 1]
        gold = {}
        for split in ("train",):
            with open(os.path.join(ud, f"en_ewt-ud-{split}.conllu"), encoding="utf-8") as f:
                for line in f:
                    line = line.rstrip("\n")
                    if not line or line.startswith("#"):
                        continue
                    cols = line.split("\t")
                    if len(cols) < 5 or "-" in cols[0] or "." in cols[0]:
                        continue
                    gold.setdefault(cols[1].lower(), {}).setdefault(cols[3], 0)
                    gold[cols[1].lower()][cols[3]] += 1
        tot = 0
        pure = 0
        per = []
        by_cluster = {}
        for w, c in labels.items():
            by_cluster.setdefault(c, []).append(w)
        for c, ws in by_cluster.items():
            votes: dict = {}
            n = 0
            for w in ws:
                for t, cnt in gold.get(w, {}).items():
                    votes[t] = votes.get(t, 0) + cnt
                    n += cnt
            if n == 0:
                continue
            best = max(votes.values())
            tot += n
            pure += best
            per.append(best / n)
        per.sort()
        print(f"# purity: eval-tokens={tot} majority-share={pure/max(1,tot):.3f} "
              f"median-cluster={per[len(per)//2]:.3f} p10={per[len(per)//10]:.3f}")
        # Show the 6 least-pure populous clusters for eyeballing.
        scored = []
        for c, ws in by_cluster.items():
            votes = {}
            n = 0
            for w in ws:
                for t, cnt in gold.get(w, {}).items():
                    votes[t] = votes.get(t, 0) + cnt
                    n += cnt
            if n >= 30:
                scored.append((max(votes.values()) / n, c, ws[:12]))
        scored.sort()
        for p, c, ws in scored[:6]:
            print(f"#   impure {p:.2f} c={c}: {' '.join(ws)}")

    if out:
        with open(out, "w", encoding="utf-8") as f:
            f.write("# word→cluster map for the tagger cluster backoff.\n")
            f.write("# Public-domain books only (Moby/Austen/Doyle/Stevenson);\n")
            f.write("# gensim skip-gram (seed 42) + numpy k-means k=256.\n")
            for w in vocab:
                f.write(f"{w}\t{labels[w]}\n")
        print(f"# wrote {out} ({len(vocab)} entries)")


if __name__ == "__main__":
    main()
