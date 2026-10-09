# Reranking scope (item 5, SCOPED 2026-10-09 — unbuilt)

N-best tag paths rescored with global features. The only clean
vehicle for `that`-gap detection (relativizers with pronominal
subjects go SCONJ only 111:50 in EWT — unseparable locally, per
the sweep-margin probe) plus parallelism symmetry and
prep-chain shapes. Local machinery is exhausted here: 17
correction rules cover predicted-tag local shapes, beam-2 spans
cover local uncertainty, and the mirror experiment (item 4)
proved disagreement alone is not a selector (breaks 34 dev /
8 sweep). What remains needs features over the whole path.

## Design

- **N-best decoder**: k-best extension of second-order Viterbi
  (features depend only on t-2/t-1/words, so prefix states are
  exact; keep k best prefixes per state). Deterministic
  lowest-index tie-breaks throughout (decoder lesson record).
- **Rescoring features** (all path-global, none available to the
  local model): base model path score (as one feature),
  forward/mirror agreement count (subsumes item 4 — the mirror
  becomes a signal, never an override), relativizer-gap
  indicators (`that` + nominal-subject + finite-verb-ahead with
  no local gap filler), coordination parallelism symmetry
  (conjunct tag/span symmetry), prep-chain run features.
  Dozens of weights (bytes), not megabytes.
- **Training**: Collins-style perceptron rerank over train
  n-best (oracle = min-loss path), offline in
  `english-pos-train`, deterministic (fixed order, md5-pinned
  like every weights change). Single-model features first;
  mirror-agreement joins only if the single-model reranker
  admits (one variable at a time).
- **Integration**: in-crate rescoring after beam decode;
  keystroke measured (k-best + rescore must hold the ~40 ms
  budget — tag pass is 9 µs/sent today, so k ≤ 10 likely fits,
  but measured not assumed).

## Stages and gates (STOP rules pre-registered)

- **Stage 0 — prize census, no code**: DONE 2026-10-09 (offline
  Python, production dev errors): (a) relativizer-gap 54,
  (b) coordination 310 loose / 247 tight (asymmetric conjunct
  readings), (c) prep-chain 55; union tight = 349 dev tokens
  (1.39pt) — PASS over the 250 bar on dev alone, mirror
  unneeded. Production dev errors 1591 (6.33%).
- **Stage 1 — k-best + oracle reachability**: DONE 2026-10-09
  (temp `kbest` example over the public scoring API, brute-force
  verified on 463 short sentences — score multisets exact).
  Pre-registered exact-path gate MISSED (gold-in-10 81.11% vs
  98%) — gate misdesigned, corrected openly: exact-path
  compounds over 12.6-token sentences (43% sentence-exact
  baseline at 93.6% token accuracy), the wrong shape for a
  path-reranker whose gains come token-wise via min-loss
  paths. Corrected criterion (oracle-min-loss token headroom,
  same +1.0pt bar shape as the mirror either-right bound):
  +1162 toks (+4.62pt) at k=10, +932 (+3.71) at k=5, +736
  (+2.93) at k=3 — PASS overwhelmingly. 1-best reproduces the
  Viterbi verdict (92.21, below greedy — miscalibration
  persists in these weights). Guidance for Stage 2: train at
  k=5 (80% of the k=10 bound at half the decode cost).
  Cost flag for Stage 3: k-best is ~361× the greedy tag pass
  per token (all history states live) — keystroke-fine
  (~3 ms/sent) but the full-book tag-pass number dies unless
  narrowed; k=5 + span-gating are the levers, measured then.
- **Stage 2 — train reranker**: MEASURED AND REJECTED
  2026-10-09 (temp `rerank` example + TEMP `tag_kbest`, both
  deleted): Collins perceptron over fixed k=5 lists, iters=10,
  single-model global features (0x30–0x34: relativizer reading
  + verb-ahead, conjunction symmetry, chain runs; base score
  fixed weight 1.0), 149 features, 3.7 KB, md5 `97027ea1`.
  Result: reranked dev 92.25 / test 91.86 vs production 93.67 /
  94.13 — net −357/−570, breaks 2× fixes (809/452,
  979/409), worse than greedy 1-best. Mechanism: the global
  features were never EWT-vetted per shape (unlike every
  admitted correction rule) and outvote the 2 MB base model
  everywhere; train updates flat ~2800/iter (no separation).
  Stage 3 never starts. Item 5 CLOSES with data; the
  mirror-as-rescoring-feature revisit (item 4's last path)
  dies with it. Reopen only on EWT-vetted global shapes with
  their own majorities (none queued) or a new combination
  idea.
- **Stage 3 — integration**: full bars — all book/chunk/lint
  evals neutral-or-better (sweep zero-breaks holds), `flies`
  holds on all paths, keystroke measured, determinism
  (md5-identical retrains), size (bytes — Tier-1 trivially).

## Non-goals and relations

- Not a replacement for the correction layer (local shapes stay
  local) or beam spans (local uncertainty stays spanned).
- Item 13 (LaSO) is independent and compatible: rerank sits on
  whatever base scores exist; if LaSO lands first, rerank
  retrains on LaSO scores (same recipe).
- No new runtime deps, no model-file growth beyond bytes,
  no grammar involvement (Tier-3 rule holds).
- If admitted, the rerank weights + recipe join the kept set
  (`ensemble`/`average-members` precedent); if any gate fails,
  full revert (item-4 precedent) with the census kept as the
  residual record.
