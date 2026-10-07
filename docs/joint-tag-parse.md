# Joint tag-parse: scope (no code)

The cascade is the largest remaining measured loss in the pipeline
(UAS gold 83.9 → pipeline 77.8; LAS gold 80.2 → pipeline 71.1) and
the only one twice confirmed structural: training downstream stages
on predicted tags trades clean-input quality ~1.5:1 against pipeline
gains (labeler +0.5/−0.7, parser +1.1/−0.8 — both rejected under the
pipeline-moves-AND-gold-holds bar). Tagger misses destroy head
evidence; nothing below the tagger can train around that. This doc
scopes the only addressed-to-cause lever: coupling tagging and
parsing. Status: design only. No code lands without clearing the
bars in §5 in order.

## 1. The distinction that matters

Stacked *training* failed (it adapts weights to noise). Joint work
must therefore couple at *inference* — searching tag×parse space
with clean weights — not in weights. Any proposal that retrains on
predicted tags is presumed guilty and must re-clear the gold-hold
bar from scratch. The pipeline-moves-AND-gold-holds discipline that
rejected both pred-tags treatments applies unchanged:
gold UAS ≥ 83.7, gold LAS ≥ 80.0 (dev, beam4, current banked numbers
83.9/80.2).

## 2. Options

### A. Tag-uncertainty features (probe first, cheapest)

Feed the tagger's uncertainty (margins, 2-best readings — both
already exist: `tag_margins`, beam re-decode) into arc and label
features: conjunctions with low-margin flags, alternative-tag
readings at uncertain positions. Features-only change; LaSO
retrains as usual; decode unchanged (beam4).

Risk: margin semantics differ between gold-tag training and
predicted-tag inference. If the model learns "low margin ⇒ back
off" on gold-tag margins (which still resolve correctly), the
signal misfires in the pipeline. Needs pred-tags alignment, which
re-invokes the 1.5:1 tradeoff — measure, don't assume.

First: Stage-0 probe (§4) decides whether the signal separates.

### B. Two-pass feedback (bounded, reuses shipped pieces)

Pass 1: greedy tags → beam4 parse (today's pipeline). Pass 2:
retag with head-agreement features (does this tag agree with what
its decoded head expects? e.g. NOUN-headed-by-ADP vs VERB readings),
then reparse once. Bounded at two passes; per-pass deltas must
diminish or it stops at one (no oscillation budget — no
convergence proof is offered, so iteration count is a hard cap,
not a fixpoint).

Risk: feedback amplifies confident errors (parse built on a wrong
tag retags toward the wrong tag). The diminishing-delta gate is
the tripwire. Uses only shipped pieces (`tag_sentence`,
`parse_beam`, one small agreement pass) — no new search.

### C. Full joint beam (most principled, most expensive)

Product search: tag beam × parse beam under jointly-trained
scoring (joint oracle = tag oracle composed with parse oracle;
joint LaSO over transition+tag actions). New training machinery —
the biggest build here by far.

Cost before code: extrapolate Moby wall-time (beam4 dep ≈ 4 s/book;
the product must stay single-digit seconds or the widths narrow
first). Tier-1 batch allows seconds; keystroke never sees this.

Only on two clean failures (A and B both measured and refused).
The cost demands a proven need, not a hoped gain.

### D. Unified transition system (rejected upfront)

One model with joint tag+parse actions (Zhang & Clark / Bohnet
style) replaces two measured stages with one unmeasured monolith
and breaks incremental shipping (a tag improvement would need full
parse re-validation). Recorded so it stays dead without new
evidence that A–C are structurally insufficient (not merely
unbuilt).

## 3. Staged plan (gated, in order)

- **Stage 0 — probes, no model change.**
  (i) Margin separability: distribution of `tag_margins` on gold
  vs predicted tags (dev). If the predicted-tag low-margin mass
  doesn't separate from gold-tag behavior, option A has no signal
  — skip to B.
  (ii) Feedback headroom (oracle experiment): correct tag errors
  at low-margin positions only, reparse, measure pipeline UAS
  delta. This bounds the ADDRESSABLE cascade. If headroom <
  +1.0 pipeline UAS, STOP the whole program (joint can't pay for
  itself; bank the pipeline numbers and move on).
- **Stage 1 — uncertainty features (A)**, iff Stage 0 passes.
  Gold-tags LaSO first; pred-tags alignment only under the
  gold-hold bar. Verdict against §5.
- **Stage 2 — two-pass feedback (B)**, iff Stage 1 insufficient.
  Hard cap two passes, diminishing-delta gate.
- **Stage 3 — joint beam (C)**, iff two clean failures.

## 4. Bars (set now, before any code)

- Pipeline UAS ≥ 78.3 (+0.5 over banked 77.8), pipeline LAS ≥ 71.8
  (+0.5 over banked 71.3), dev and test.
- Gold holds: UAS ≥ 83.7, LAS ≥ 80.0.
- Stage-0 headroom ≥ +1.0 pipeline UAS, else stop.
- Standing gates ride along: EWT dev/test ≥ 0 on every change,
  evals + bench + `flies` vetoes, weights md5 hygiene, deterministic
  retrains. A bar moves deliberately with a recorded reason, or the
  change is reverted (workflow §1).

## 5. Budgets

- Tier-1 batch: full-book seconds acceptable; the keystroke path
  (greedy tag + ... ) is untouched — dep never runs per keystroke
  and joint work doubly doesn't.
- Beam widths need a Moby extrapolation before Stage 3 commits.
- Artifacts stay gitignored pairs (parser + labeler + whatever
  joint weights); nothing vendored without single-digit-MB
  justification and its own gate clearance.

## 6. Explicitly out

- Runtime backfeed into Tier-0 grammar (the Tier-3 rule covers
  tag→grammar direction too — argument structure never enters
  segmentation).
- Transformer/BERT joint models (evicted class; offline oracles
  only, per the standing neural-runtimes decision).
- Changing the TAGGER under joint pretext without its own full
  clearance (EWT dev/test + moby/genre/hard/sweep + bench +
  `flies` veto). Joint work consumes the shipped 92.05 tagger;
  it doesn't risk it. Any tagger-side change re-clears
  everything, same as always.
