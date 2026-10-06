#!/usr/bin/env python3
"""Unsupervised abbreviation discovery (DEV-TIME ONLY).

Trains NLTK's Punkt trainer (Kiss & Strunk 2006) on literary prose
and prints Dunning log-likelihood-ranked abbreviation candidates
with scores, our-list membership, corpus counts, and example
contexts — for human curation into the abbreviation-harvest item
(standing rule: no large auto-imported list; EWT-majority checks;
over-listing wrongly-open breaks are worse than misses).

Offline oracle use: NLTK stays out of the repo (heavy,
non-incremental, dependency-free runtime). Nothing here ships as
runtime tables. Why direct NLTK use instead of a from-scratch
port: the reference trainer is installed in this env, so calling
it yields Kiss & Strunk's numbers with zero reimplementation risk;
no NLTK regexes or wordlists are copied (Apache-2.0 respected by
non-copy). Script pins: nltk version printed in the header.

Our-list membership reads the single source of truth,
`bindings/rust/scanner.rs` (`ABBREVIATIONS` array), so the report
never drifts from the grammar.

Usage:
  oracle-python scripts/punkt-harvest.py /tmp/opencode/moby-ch1.txt [--top 60]

Output: one line per candidate: LL score, type, ours[y/n],
count-with-period, count-bare, up-to-two contexts. Above-threshold
(≥ 0.3) first, then below, both by score.
"""

import os
import re
import sys


def our_list(repo):
    src = open(os.path.join(repo, "bindings/rust/scanner.rs"), encoding="utf-8").read()
    m = re.search(r"const ABBREVIATIONS: &\[&str\] = &\[([^\]]*)\]", src)
    if not m:
        sys.exit("ABBREVIATIONS array not found in scanner.rs")
    return set(re.findall(r'"([a-z]+)"', m.group(1)))


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: punkt-harvest.py <text> [--top N]")
    path = sys.argv[1]
    top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 60
    repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    import nltk

    print(f"# nltk {nltk.__version__} Punkt harvest on {path}")
    from nltk.tokenize.punkt import PunktTrainer

    text = open(path, encoding="utf-8").read()
    trainer = PunktTrainer()
    tokens = list(trainer._tokenize_words(text))
    # Populate type frequencies exactly as train() would (without
    # finalizing — we only want the abbreviation ranking).
    for aug_tok in tokens:
        trainer._type_fdist[aug_tok.type] += 1
        if aug_tok.period_final:
            trainer._num_period_toks += 1
    scored = list(trainer._reclassify_abbrev_types(trainer._unique_types(tokens)))

    ours = our_list(repo)
    rows = []
    for abbr, score, is_add in scored:
        if not is_add:
            continue
        with_p = trainer._type_fdist[abbr + "."]
        bare = trainer._type_fdist[abbr]
        rows.append((score, abbr, with_p, bare))
    rows.sort(reverse=True)

    print(f"# {len(rows)} period-final types scored; ours={sorted(ours)}")
    shown = 0
    for score, abbr, with_p, bare in rows:
        if shown >= top:
            break
        shown += 1
        flag = "ours" if abbr.lower() in ours else "NEW"
        ctxs = []
        start = 0
        for _ in range(2):
            i = text.find(abbr + ".", start)
            if i < 0:
                break
            ctxs.append(text[max(0, i - 45):i + len(abbr) + 25].replace("\n", "\\n"))
            start = i + 1
        print(f"{score:7.4f} {abbr!r:18} {flag:4} with={with_p:<5} bare={bare:<5} | {' // '.join(ctxs)}")


if __name__ == "__main__":
    main()
