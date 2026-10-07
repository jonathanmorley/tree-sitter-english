//! Oracle tests: the static oracle walk reproduces gold trees,
//! including the projectivity guard (a popped dependent must not
//! strand right children) and Reduce timing.

use english_dep::{Action, Config, oracle};

// `Time flies like an arrow`: heads[1]=2 (nsubj), heads[2]=0
// (root), heads[3]=2 (prep... simplified: 3->2), heads[4]=5 (det),
// heads[5]=3 (pobj). Index 0 unused.
fn arrow_heads() -> Vec<usize> {
    vec![0, 2, 0, 2, 5, 3]
}

fn walk(words: usize, gold: &[usize]) -> Vec<usize> {
    let mut cfg = Config::initial(words);
    let mut guard = 0;
    while !cfg.is_terminal(words) {
        guard += 1;
        assert!(guard < 100, "oracle walk did not terminate");
        cfg.apply(oracle(gold, &cfg));
    }
    cfg.heads
}

#[test]
fn oracle_reproduces_gold_tree() {
    // heads: 0:dummy 1->2 2->0 3->2 4->5 5->3
    let gold = arrow_heads();
    let heads = walk(5, &gold);
    assert_eq!(&heads[1..], &gold[1..]);
}

#[test]
fn left_arc_waits_for_dependents() {
    // `big dogs bark`: 1->2 (amod), 2->3 (nsubj), 3->0 (root).
    // When stack=[0,1], buf=2: gold head of 1 is 2, but the oracle
    // must Shift first only if... here 1 has no children, so Left
    // fires immediately: heads[1]=2, pop. Then stack=[0], buf=2:
    // Right(0->2)? gold head of 2 is 3, not 0 — Shift. Then
    // stack=[0,2], buf=3: gold head of 2 is 3 → Left? No: Left
    // needs gold_heads[s0]==buf → 3==3 ✓, and 2's children (1, via
    // gold 1->2) attached ✓ → Left: heads[2]=3. Then [0],3:
    // gold head of 3 is 0... Right needs gold_heads[buf]==s0:
    // buf=4 > n=3. legal: shift=F. stack=[0]: s0=0, attached[0]
    // false → reduce=F. left: s0==0 → F. right: buf>n → F. Only
    // the deadlock fallback... stack.len()==1 so even fallback is
    // off — but is_terminal(3): buf=4>3 && len==1 ✓ terminal.
    // Walk it through the harness instead of hand-simulating.
    let gold = vec![0, 2, 3, 0];
    let heads = walk(3, &gold);
    assert_eq!(&heads[1..], &gold[1..]);
}

#[test]
fn projectivity_guard_blocks_early_pop() {
    // Constructed: gold 1->3 (dep), 2->1 (dep), 3->0 (root), i.e.
    // heads = [_, 3, 1, 0]. Tokens: 1 has child 2; 1's head is 3.
    // At stack=[0,1], buf=2: gold_heads[1]==3 != 2, no Right
    // (gold_heads[2]==1 == s0=1 → Right fires: heads[2]=1).
    // At stack=[0,1,2], buf=3: gold_heads[1]==3==buf BUT 1 still
    // has unattached child 2... wait 2 just attached (heads[2]=1,
    // attached[2]=true) → guard passes → Left: heads[1]=3. Good.
    // Now the guard-negative case: reorder so the child comes
    // later is impossible in projective order — instead assert the
    // guard directly: at stack=[0,1], buf=3 with 2 unattached and
    // gold 1->3, 2->1: oracle must NOT Left (would strand 2).
    let gold = vec![0, 3, 1, 0];
    // Drive to stack=[0,1], buf=3 manually: Shift, Shift would
    // attach nothing; use oracle steps: [0],1 → gold_heads[1]=3≠1?
    // Step 1: stack=[0], buf=1: gold_heads[0]... s0=0.
    // oracle: s0=0; Left needs gold_heads[0]==1: gold[0]=0 ≠ 1.
    // Right needs gold_heads[1]==0: no (3). Reduce: s0==0 → no.
    // → Shift → stack=[0,1], buf=2.
    // Step 2: s0=1: gold_heads[1]=3 ≠ 2. Right: gold_heads[2]=1
    // == s0 ✓ → Right: heads[2]=1, stack=[0,1,2], buf=3.
    // Step 3: s0=2: gold_heads[2]=1 ≠ 3. Right: buf=3, gold_heads
    // [3]=0 ≠ 2. Reduce: attached[2] ✓ → Reduce → [0,1],3.
    // Step 4: s0=1, buf=3: gold_heads[1]=3==buf; children of 1:
    // {2}, attached ✓ → Left: heads[1]=3. Then [0],3... buf=4>n:
    // terminal check needs stack [0]: stack=[0,1] — Reduce? s0=1
    // attached ✓ → Reduce → [0], buf=4 → terminal.
    let heads = walk(3, &gold);
    assert_eq!(&heads[1..], &gold[1..]);

    // Guard-negative probe: fresh config forced to the dangerous
    // state must refuse Left.
    let mut cfg2 = Config::initial(3);
    cfg2.stack = vec![0, 1];
    cfg2.buf = 3;
    // 2 unattached, gold child of 1.
    assert_eq!(oracle(&gold, &cfg2), Action::Shift);
}

#[test]
fn reduce_needs_a_head() {
    // stack=[0,1] with 1 unattached and buf past a non-head:
    // must Shift, never Reduce-or-pop root.
    let gold = vec![0, 0, 1];
    // stack=[0], buf=1: gold_heads[0]=0≠1; Right: gold_heads[1]=0...
    // s0=0: Right needs gold_heads[buf]==s0 → gold_heads[1]=0==0 ✓
    // → Right: heads[1]=0, stack=[0,1], buf=2.
    // stack=[0,1], buf=2: Left? gold_heads[1]=0≠2. Right?
    // gold_heads[2]=1==s0 ✓ → Right: heads[2]=1.
    // stack=[0,1,2], buf=3>n: Reduce? attached[2] ✓ → pop...
    // walk through harness:
    let heads = walk(2, &gold);
    assert_eq!(&heads[1..], &gold[1..]);
}

#[test]
fn nonprojective_walk_terminates() {
    // Crossing arcs 1->3, 2->4: no oracle walk reproduces this
    // (static oracle is projective-only — the trainer filters such
    // sentences). The walk must still terminate via the Reduce
    // backstop instead of pushing buf past the sentence.
    let gold = vec![0, 3, 4, 0, 0];
    let heads = walk(4, &gold);
    assert_eq!(heads.len(), 5);
}

#[test]
fn beam_width_1_matches_greedy() {
    // Width 1 keeps only the argmax each step — the greedy path
    // exactly (same decisions, same margins). Guards the shared
    // scoring path against beam/greedy divergence.
    use english_dep::Model;
    let words = ["time", "flies", "like", "an", "arrow"];
    let tags = ["NOUN", "VERB", "ADP", "DET", "NOUN"];
    let model = Model::train(
        &[(
            words.iter().map(|s| s.to_string()).collect(),
            tags.iter().map(|s| s.to_string()).collect(),
            vec![0, 2, 0, 2, 5, 3],
        )],
        20,
        1,
    );
    let w: Vec<String> = words.iter().map(|s| s.to_string()).collect();
    let t: Vec<String> = tags.iter().map(|s| s.to_string()).collect();
    let (gheads, gmarg) = model.parse_margins(&w, &t);
    let (bheads, bmarg) = model.parse_beam(&w, &t, 1);
    assert_eq!(gheads, bheads);
    assert_eq!(gmarg, bmarg);
}

#[test]
fn action_codes_roundtrip() {
    use english_dep::Action;
    for a in Action::ALL {
        assert_eq!(Action::from_code(a.code()), Some(a));
    }
    assert_eq!(Action::from_code("BOGUS"), None);
}
