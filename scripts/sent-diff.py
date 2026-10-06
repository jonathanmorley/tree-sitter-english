#!/usr/bin/env python3
"""Differential sentence-boundary oracle (DEV-TIME ONLY).

Runs two external segmenters over the same inputs the grammar parses
and diffs their sentence boundaries against this grammar's, hunting
for sentence-splitting disagreements. The oracles are never runtime
deps (Python, heavy, non-incremental — they would break the reparse
and dependency budgets); they only produce candidate lists for human
triage here.

  oracles : NLTK Punkt (unsupervised, WSJ-trained) + spaCy
            `en_core_web_sm` parser sentences.
  grammar : TSV from the throwaway helper
            `cargo run -p english --example sent_bounds -- <files>`
            (path, start byte, end byte per sentence).

Inputs: `examples/*.txt` + Moby-Dick (Gutenberg 2701) measured from
`CHAPTER 1. Loomings.` onward (same convention as the audit/bench
examples). Corpora stay out of the repo (see scripts/fetch-ud.sh
for why); NLTK data + venv live under /tmp.

Usage:
  cargo run -p english --example sent_bounds -- examples/*.txt /tmp/moby.txt > /tmp/bounds.tsv
  python3 scripts/sent-diff.py /tmp/bounds.tsv --moby /tmp/moby.txt --examples examples/

Output: per-file boundary-agreement counts for grammar-vs-Punkt and
grammar-vs-spaCy, then every delta with context, heuristically
tagged (abbrev / dialogue-dash / quote / paren / numeric /
transcription) for triage into grammar-miss / oracle-miss /
transcription. License: dev-time use only — nothing learned here
is copied into the repo (oracle parameters stay out; only
human-curated corpus tests or constrained scanner fixes land,
per the item's acceptance).

Requires: nltk (punkt), spacy + en_core_web_sm.
"""

import json
import os
import re
import sys

ABBREV = re.compile(r"\b([A-Z][a-z]{1,4}|[A-Z]\.|No|Mr|Mrs|Ms|Dr|St)\.\s*$")
LEAD_DASH = re.compile(r"(^|\n)\s*[—–-]\s*\S")
NUMERIC = re.compile(r"\d[:.,]\d")
TRANSCRIPTION = re.compile(r"[_*\[\]{}^]")


def load_bounds(path):
    out = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            p, s, e, _err, _ctx = line.split("\t", 4)
            out.setdefault(p, []).append((int(s), int(e)))
    return out


def byte_to_char(text):
    """Map every byte offset to its char offset (grammar bounds are
    byte offsets from tree-sitter; Punkt/spaCy yield char offsets).
    Linear pre-pass per file; boundaries are char-aligned, so an
    exact lookup holds."""
    import bisect

    starts = []
    b = 0
    for ch in text:
        starts.append(b)
        b += len(ch.encode("utf-8"))

    def conv(byte):
        i = bisect.bisect_right(starts, byte) - 1
        return max(0, i)

    return conv


def norm_end(text, start, end):
    while end > start and text[end - 1] in " \t\n\r":
        end -= 1
    return end


def boundaries(text, spans):
    ends = set()
    for s, e in spans:
        e = norm_end(text, s, e)
        if e > s:
            ends.add(e)
    return ends


def tag_ctx(text, pos):
    lo = max(0, pos - 100)
    hi = min(len(text), pos + 60)
    ctx = text[lo:hi].replace("\n", "\\n")
    tags = []
    if ABBREV.search(text[max(0, pos - 12):pos]):
        tags.append("abbrev?")
    if LEAD_DASH.search(text[lo:hi]):
        tags.append("dialogue-dash?")
    if '"' in text[lo:hi] or "“" in text[lo:hi]:
        tags.append("quote?")
    if "(" in text[lo:hi] or ")" in text[lo:hi]:
        tags.append("paren?")
    if NUMERIC.search(text[lo:hi]):
        tags.append("numeric?")
    if TRANSCRIPTION.search(text[lo:hi]):
        tags.append("transcription?")
    return (", ".join(tags) or "plain"), ctx


def main():
    if len(sys.argv) < 2 or "--moby" not in sys.argv:
        sys.exit("usage: sent-diff.py /tmp/bounds.tsv --moby /tmp/moby.txt [--examples examples/]")
    bounds_path = sys.argv[1]
    moby = sys.argv[sys.argv.index("--moby") + 1]
    exdir = sys.argv[sys.argv.index("--examples") + 1] if "--examples" in sys.argv else "examples"

    import nltk

    nltk.data.path.insert(0, "/tmp/opencode/nltk_data")
    from nltk.tokenize.punkt import PunktTokenizer

    try:
        punkt = PunktTokenizer("english")
    except LookupError:
        nltk.download("punkt_tab", download_dir="/tmp/opencode/nltk_data")
        punkt = PunktTokenizer("english")

    import spacy

    try:
        nlp = spacy.load("en_core_web_sm", disable=["ner", "lemmatizer"])
    except OSError:
        sys.exit("spaCy model missing: run `python -m spacy download en_core_web_sm` in the venv")
    nlp.max_length = max(nlp.max_length, 3_000_000)

    bounds = load_bounds(bounds_path)
    files = sorted([os.path.join(exdir, f) for f in os.listdir(exdir) if f.endswith(".txt")
                    and not f.endswith(".parse.txt")] + [moby])
    print(f"files: {len(files)}")
    total = {"gp": [0, 0], "gs": [0, 0]}  # [agree, delta] per pair
    deltas = []
    for path in files:
        key = next((k for k in bounds if k.endswith(os.path.basename(path))), None)
        if key is None:
            print(f"  SKIP {path} (no grammar bounds)")
            continue
        with open(path, encoding="utf-8") as f:
            text = f.read()
        conv = byte_to_char(text)
        gspans = [(conv(s), conv(e)) for (s, e) in bounds[key]]
        g = boundaries(text, gspans)
        p = boundaries(text, list(punkt.span_tokenize(text)))
        s = boundaries(text, [(s.start_char, s.end_char) for s in nlp(text).sents])
        for name, other in (("punkt", p), ("spacy", s)):
            agree = len(g & other)
            only_g = sorted(other - g)  # oracle ends where grammar continues
            only_o = sorted(g - other)  # grammar ends where oracle continues
            k = "gp" if name == "punkt" else "gs"
            total[k][0] += agree
            total[k][1] += len(only_g) + len(only_o)
            for pos in only_g:
                tags, ctx = tag_ctx(text, pos)
                deltas.append((path, name, "oracle-splits", pos, tags, ctx))
            for pos in only_o:
                tags, ctx = tag_ctx(text, pos)
                deltas.append((path, name, "grammar-splits", pos, tags, ctx))
        print(f"  {os.path.basename(path)}: grammar bounds={len(g)} "
              f"punkt Δ={len(p - g) + len(g - p)} spacy Δ={len(s - g) + len(g - s)}")
    for k, name in (("gp", "grammar-vs-punkt"), ("gs", "grammar-vs-spacy")):
        a, d = total[k]
        print(f"{name}: agree={a} delta={d}")
    print(f"--- deltas ({len(deltas)}) ---")
    for path, name, direction, pos, tags, ctx in sorted(deltas):
        print(f"[{os.path.basename(path)}] [{name}] [{direction}] @{pos} [{tags}] {ctx}")


if __name__ == "__main__":
    main()
