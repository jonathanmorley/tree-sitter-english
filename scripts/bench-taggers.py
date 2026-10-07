#!/usr/bin/env python3
"""Rerunnable POS-tagger shootout (DEV-TIME ONLY).

Compares this repo's tagger against external systems on the same
data: exact-UPOS accuracy on EWT test gold words plus tagger-only
speed on Moby-Dick body words. Nothing here is a runtime dep;
every competitor is optional and skipped with an install hint
when absent (corpora and models stay out of the repo, same as
scripts/fetch-ud.sh and scripts/sent-diff.py).

  ours   : committed weights via
           cargo run --release -p english-pos --example tag_tokens
           --sentences (one decode per sentence, history resets as
           production tag_sentence does; flat whole-file decoding
           leaks cross-sentence history and scores ~0.5 worse).
  nltk   : NLTK averaged perceptron (WSJ-trained, Penn tags) —
           coarse universal-12 only (out-of-domain by construction).
  rdr    : RDRPOSTagger with a UPOS-EWT .rdr model — exact UPOS.
  spacy  : en_core_web_sm (token.pos_) — exact UPOS; sentences
           whose tokenization drifts from gold are excluded and
           the coverage is reported.
  treetagger : TreeTagger binary + english.par (Penn tags) —
           coarse universal-12 only; research-licensed, never
           vendored (see https://www.cis.uni-muenchen.de/~schmid/
           tools/TreeTagger/ and the license terms there).

Usage:
  scripts/fetch-ud.sh --dir /tmp/ud ewt   # EWT test conllu
  # Moby-Dick Gutenberg 2701 to /tmp/moby.txt (out-of-repo, as usual)
  python3 scripts/bench-taggers.py \
    --conllu /tmp/ud/ewt/en_ewt-ud-test.conllu --moby /tmp/moby.txt

  Optional: --rdr-model PATH --treetagger BIN --tt-params english.par
            --tsv /tmp/shootout.tsv
  Run inside `nix develop` so cargo is present for the ours leg.
"""

import shutil
import subprocess
import sys
import tempfile
import time

# Penn Treebank -> Petrov universal-12 (Petrov et al. 2012, Table 1;
# reimplemented from the published table, not copied code).
PTB_TO_UNI = {
    "CC": "CONJ", "CD": "NUM", "DT": "DET", "EX": "DET",
    "IN": "ADP", "JJ": "ADJ", "JJR": "ADJ", "JJS": "ADJ",
    "LS": "X", "MD": "VERB", "NN": "NOUN", "NNS": "NOUN",
    "NNP": "NOUN", "NNPS": "NOUN", "PDT": "DET", "POS": "PRT",
    "PRP": "PRON", "PRP$": "PRON", "RB": "ADV", "RBR": "ADV",
    "RBS": "ADV", "RP": "PRT", "TO": "PRT", "UH": "X",
    "VB": "VERB", "VBD": "VERB", "VBG": "VERB", "VBN": "VERB",
    "VBP": "VERB", "VBZ": "VERB", "WDT": "DET", "WP": "PRON",
    "WP$": "PRON", "WRB": "ADV", "FW": "X", "SYM": ".",
    ".": ".", ",": ".", ":": ".", "``": ".", "''": ".",
    "-LRB-": ".", "-RRB-": ".", "HYPH": ".", "NFP": ".",
    "SENT": ".",
}

# english.par as shipped (2026-10-07) emits a Penn/CLAWS mix on EWT
# test input (PP/NP/VV*-family alongside VBZ/NN/JJ). Mapped from the
# observed 25,094-word inventory, same universal-12 targets.
TT_EXTRA = {
    "NP": "NOUN", "NPS": "NOUN",
    "PP": "PRON", "PP$": "PRON",
    "VV": "VERB", "VVD": "VERB", "VVG": "VERB", "VVN": "VERB",
    "VVP": "VERB", "VVZ": "VERB",
    "VH": "VERB", "VHD": "VERB", "VHG": "VERB", "VHN": "VERB",
    "VHP": "VERB", "VHZ": "VERB",
    "IN/that": "CONJ",
    ",": ".", ":": ".", "''": ".", "``": ".", "(": ".", ")": ".",
    "#": ".", "$": ".",
}
PTB_TO_UNI = {**PTB_TO_UNI, **TT_EXTRA}


def coarse_upos(tag):
    """UPOS -> universal-12, same projection as PTB_TO_UNI targets."""
    return {
        "PROPN": "NOUN", "AUX": "VERB", "CCONJ": "CONJ",
        "SCONJ": "CONJ", "PART": "PRT", "PUNCT": ".",
        "SYM": ".", "INTJ": "X",
    }.get(tag, tag)


def read_conllu(path):
    """Return [(words, upos)] skipping MWTs and empty nodes."""
    sents, words, tags = [], [], []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                if words:
                    sents.append((words, tags))
                    words, tags = [], []
                continue
            if line.startswith("#"):
                continue
            cols = line.split("\t")
            if "-" in cols[0] or "." in cols[0]:
                continue
            words.append(cols[1])  # FORM
            tags.append(cols[3])  # UPOS (cols[2] is LEMMA)
    if words:
        sents.append((words, tags))
    return sents


def moby_words(path):
    """Body words from CHAPTER 1 onward (scale-corpus convention)."""
    with open(path, encoding="utf-8") as f:
        text = f.read()
    mark = "CHAPTER 1. Loomings."
    i = text.find(mark)
    body = text[i:] if i >= 0 else text
    return body.split()


def run_ours(sents):
    """Tag EWT test sentences with committed weights, one sentence per
    decode (history resets, as production tag_sentence does). Flat
    whole-file decoding leaks cross-sentence history and scores ~0.5
    worse — never use it for accuracy."""
    if shutil.which("cargo") is None:
        return None, "skip (no cargo; run inside nix develop)"
    with tempfile.NamedTemporaryFile(
            "w", suffix=".txt", delete=False) as f:
        f.write("\n\n".join("\n".join(w) for w, _ in sents) + "\n")
        tokfile = f.name

    def run(extra):
        try:
            out = subprocess.run(
                ["cargo", "run", "--quiet", "--release",
                 "-p", "english-pos", "--example", "tag_tokens",
                 "--", tokfile, "--sentences"] + extra,
                capture_output=True, text=True, check=True).stdout
        except subprocess.CalledProcessError as e:
            return None, f"skip (tag_tokens failed: {e.stderr[-200:]})"
        pred = [ln.split("\t")[1] for ln in out.splitlines() if "\t" in ln]
        if len(pred) != n:
            return None, (f"skip (tag count {len(pred)} != "
                           f"word count {n})")
        return pred, None

    n = sum(len(w) for w, _ in sents)
    t = time.perf_counter()
    (greedy, err) = run([])
    if err:
        return None, err
    (prod, err) = run(["--production"])
    dt = time.perf_counter() - t
    if err:
        return None, err
    return ((greedy, prod), dt), None


def score_exact(pred, gold):
    return sum(p == g for p, g in zip(pred, gold))


def score_coarse(pred, gold, to_uni):
    n = sum(to_uni(p) == coarse_upos(g)
            for p, g in zip(pred, gold) if to_uni(p) is not None)
    d = sum(1 for p in pred if to_uni(p) is not None)
    return n, d


def leg_nltk(sents):
    try:
        from nltk.tag import PerceptronTagger
    except ImportError:
        return None, "skip (no nltk; nixpkgs#python3Packages.nltk)"
    try:
        tagger = PerceptronTagger()
    except LookupError:
        return None, ("skip (averaged_perceptron_tagger data missing; "
                       "nltk.download it to /tmp/opencode/nltk_data)")
    pred, t = [], time.perf_counter()
    for words, _ in sents:
        pred.extend(t for _, t in tagger.tag(words))
    dt = time.perf_counter() - t
    return (pred, dt), None


def leg_rdr(sents, model):
    if model is None:
        return None, "skip (--rdr-model not given)"
    try:
        from RDRPOSTagger.pSCRDRtagger import RDRPOSTagger
    except ImportError:
        return None, "skip (no rdrpostagger package)"
    r = RDRPOSTagger()
    try:
        r.loadFromFile(model)
    except Exception as e:  # noqa: BLE001 - path/parse errors surface as-is
        return None, f"skip (RDR model load failed: {e})"
    pred, t = [], time.perf_counter()
    for words, _ in sents:
        tagged = r.tagRawSentence(" ".join(words))
        pred.extend(tok.rsplit("/", 1)[1]
                    for tok in tagged.split(" "))
    dt = time.perf_counter() - t
    return (pred, dt), None


def leg_spacy(sents):
    try:
        import spacy
    except ImportError:
        return None, "skip (no spacy)", None
    try:
        nlp = spacy.load("en_core_web_sm")
    except OSError:
        return None, ("skip (en_core_web_sm missing; "
                      "python -m spacy download en_core_web_sm)"), None
    pred, gkept, t = [], [], time.perf_counter()
    for words, tags in sents:
        doc = nlp(" ".join(words))
        if len(doc) != len(words):
            continue
        pred.extend(tok.pos_ for tok in doc)
        gkept.extend(tags)
    dt = time.perf_counter() - t
    cov = len(gkept) / max(1, sum(len(w) for w, _ in sents))
    note = (None if cov == 1.0 else f"aligned {cov:.3f} of gold words")
    return (pred, dt, gkept), None, note


def leg_treetagger(sents, binary, params):
    if binary is None or params is None:
        return None, "skip (--treetagger/--tt-params not given)"
    if shutil.which(binary) is None:
        return None, f"skip ({binary} not installed; see TreeTagger page)"
    with tempfile.NamedTemporaryFile(
            "w", suffix=".txt", delete=False) as f:
        for words, _ in sents:
            f.write("\n".join(words) + "\n\n")
        infile = f.name
    t = time.perf_counter()
    try:
        out = subprocess.run(
            [binary, "-token", "-lemma", params, infile],
            capture_output=True, text=True, check=True).stdout
    except subprocess.CalledProcessError as e:
        return None, f"skip (treetagger failed: {e.stderr[-200:]})"
    dt = time.perf_counter() - t
    pred = [ln.split("\t")[1] for ln in out.splitlines()
            if ln.count("\t") >= 2]
    if len(pred) != sum(len(w) for w, _ in sents):
        return None, ("skip (treetagger token count drifted; "
                       "likely SGML/encoding in input)")
    return (pred, dt), None


def speed_each(name, words, fn):
    t = time.perf_counter()
    fn(words)
    dt = time.perf_counter() - t
    return len(words) / max(dt, 1e-9)


def main(argv):
    import argparse
    ap = argparse.ArgumentParser(description="rerunnable tagger shootout")
    ap.add_argument("--conllu", required=True)
    ap.add_argument("--moby", required=True)
    ap.add_argument("--rdr-model", default=None)
    ap.add_argument("--treetagger", default=None)
    ap.add_argument("--tt-params", default=None)
    ap.add_argument("--tsv", default=None)
    a = ap.parse_args(argv)

    sents = read_conllu(a.conllu)
    gold = [g for _, tags in sents for g in tags]
    words = [w for wds, _ in sents for w in wds]
    print(f"test: {len(sents)} sents, {len(gold)} gold words from {a.conllu}")
    mwords = moby_words(a.moby)
    print(f"speed: {len(mwords)} body words from {a.moby}")

    rows = []  # (system, exact, exact_n, coarse, coarse_d, tok/s, note)

    # Ours: greedy + production (exact UPOS + coarse).
    (res, err) = run_ours(sents)
    if err:
        rows.append(("ours-greedy", None, 0, None, 0, None, err))
        rows.append(("ours-prod", None, 0, None, 0, None, err))
    else:
        (greedy, prod), dt = res
        ex = score_exact(greedy, gold)
        cn, cd = score_coarse(greedy, gold, coarse_upos)
        # Tagger-only speed on the shared Moby word list (greedy path).
        with tempfile.NamedTemporaryFile(
                "w", suffix=".txt", delete=False) as f:
            f.write("\n".join(mwords) + "\n")
            mf = f.name
        t = time.perf_counter()
        subprocess.run(
            ["cargo", "run", "--quiet", "--release",
             "-p", "english-pos", "--example", "tag_tokens",
             "--", mf],
            capture_output=True, text=True, check=True)
        sdt = time.perf_counter() - t
        rows.append(("ours-greedy", ex / len(gold), len(gold),
                     cn / cd, cd, len(mwords) / sdt,
                     "per-sentence decode"))
        ex = score_exact(prod, gold)
        cn, cd = score_coarse(prod, gold, coarse_upos)
        rows.append(("ours-prod", ex / len(gold), len(gold),
                     cn / cd, cd, None, "beam-2 + 14 rules"))

    # NLTK perceptron (coarse only).
    (res, err) = leg_nltk(sents)
    if err:
        rows.append(("nltk-perceptron", None, 0, None, 0, None, err))
    else:
        pred, _ = res
        cn, cd = score_coarse(pred, gold, PTB_TO_UNI.get)
        try:
            from nltk.tag import PerceptronTagger as _P
            _tagger = _P()
            r = speed_each("nltk", mwords, _tagger.tag)
        except Exception:  # noqa: BLE001 - speed is best-effort
            r = None
        rows.append(("nltk-perceptron", None, 0, cn / cd, cd, r,
                     "WSJ Penn tags; domain gap conflated"))

    # RDRPOSTagger (exact UPOS).
    (res, err) = leg_rdr(sents, a.rdr_model)
    if err:
        rows.append(("rdr", None, 0, None, 0, None, err))
    else:
        pred, _ = res
        ex = score_exact(pred, gold)
        cn, cd = score_coarse(pred, gold, coarse_upos)
        rows.append(("rdr", ex / len(gold), len(gold),
                     cn / cd, cd, None, "speed: rerun tagRawSentence timed"))

    # spaCy (exact UPOS on aligned subset).
    (res, skip, note) = leg_spacy(sents)
    if skip:
        rows.append(("spacy-sm", None, 0, None, 0, None, skip))
    else:
        pred, _, gkept = res
        ex = score_exact(pred, gkept)
        cn, cd = score_coarse(pred, gkept, coarse_upos)
        rows.append(("spacy-sm", ex / len(gkept), len(gkept),
                     cn / cd, cd, None, note or "aligned subset"))

    # TreeTagger (coarse only).
    (res, err) = leg_treetagger(sents, a.treetagger, a.tt_params)
    if err:
        rows.append(("treetagger", None, 0, None, 0, None, err))
    else:
        pred, _ = res
        cn, cd = score_coarse(pred, gold, PTB_TO_UNI.get)
        rows.append(("treetagger", None, 0, cn / cd, cd, None,
                     "Penn tags; research license, never vendored"))

    print(f"{'system':<16}{'exact-UPOS':>12}{'n':>7}"
          f"{'coarse-12':>11}{'tok/s':>12}  note")
    for name, ex, n, co, _cd, rate, note in rows:
        exs = f"{ex:.4f}" if ex is not None else "      -"
        cos = f"{co:.4f}" if co is not None else "      -"
        rs = f"{rate:,.0f}" if rate else "       -"
        print(f"{name:<16}{exs:>12}{n:>7}{cos:>11}{rs:>12}  {note}")
    if a.tsv:
        with open(a.tsv, "w", encoding="utf-8") as f:
            f.write("system\texact\tn\tcoarse\ttok_per_s\tnote\n")
            for name, ex, n, co, _cd, rate, note in rows:
                f.write(f"{name}\t{ex}\t{n}\t{co}\t{rate}\t{note}\n")
        print(f"wrote {a.tsv}")


if __name__ == "__main__":
    main(sys.argv[1:])
