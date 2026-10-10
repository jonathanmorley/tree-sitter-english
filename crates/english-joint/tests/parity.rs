//! Stage-1 joint gates: torch parity on the pinned 50-sent dev sample
//! (tags + heads + rels, zero diffs) and MST-vs-brute-force on seeded
//! random graphs. Weights/vectors /tmp-only (EWT text never vendored).

use english_joint::{JointModel, chu_liu_edmonds};

/// Stage-1 parity bar: Rust joint decode identical to torch on all
/// three outputs. Zero diffs allowed.
#[test]
fn joint_torch_parity() {
    let weights = match std::fs::read_to_string("/tmp/opencode/round3/joint.json") {
        Ok(w) => w,
        Err(_) => {
            eprintln!("skip: /tmp joint weights absent");
            return;
        }
    };
    let vectors = match std::fs::read_to_string("/tmp/opencode/round3/joint-parity.json") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skip: /tmp joint parity vectors absent");
            return;
        }
    };
    let model = JointModel::from_json(&weights).expect("weights load");
    let rows: Vec<serde_json::Value> = serde_json::from_str(&vectors).expect("vectors parse");
    assert_eq!(rows.len(), 50);
    let (mut dt, mut dh, mut dr) = (0, 0, 0);
    for r in &rows {
        let words: Vec<&str> = r["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_str().unwrap())
            .collect();
        let want_t: Vec<&str> = r["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        let want_h: Vec<i64> = r["heads"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h.as_i64().unwrap())
            .collect();
        let want_r: Vec<&str> = r["rels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        let p = model.parse(&words);
        for (g, w) in p.tags.iter().zip(want_t.iter()) {
            dt += (g.upos() != *w) as usize;
        }
        for (g, w) in p.heads.iter().zip(want_h.iter()) {
            dh += (*g as i64 != *w) as usize;
        }
        for (g, w) in p.rels.iter().zip(want_r.iter()) {
            dr += (*g != *w) as usize;
        }
    }
    eprintln!("joint parity diffs: tags {dt} heads {dh} rels {dr}");
    assert_eq!((dt, dh, dr), (0, 0, 0), "joint parity diffs");
}

/// MST vs brute-force arborescence on seeded random graphs
/// (independent implementation — checks contraction, not just paths).
#[test]
fn mst_matches_brute_force() {
    let mut state: u64 = 0x12345678;
    let mut rnd = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) as f32) / (u32::MAX as f32) * 10.0 - 5.0
    };
    for trial in 0..30 {
        let n = 2 + (trial % 4); // 2..5 deps
        let mut s = vec![vec![0.0f32; n + 1]; n + 1];
        for i in 0..=n {
            for j in 0..=n {
                s[i][j] = if i == j { f32::NEG_INFINITY } else { rnd() };
            }
        }
        let got = chu_liu_edmonds(&s, 0);
        let want = brute(&s);
        assert_eq!(got, want, "trial {trial}");
    }
}

/// Brute-force max arborescence (root 0): enumerate all head
/// assignments, keep the best well-formed tree. Exponential — tiny
/// graphs only, independent of the contraction code path.
fn brute(scores: &[Vec<f32>]) -> Vec<i32> {
    let n = scores.len() - 1;
    let mut best = vec![-1i32; n + 1];
    let mut best_w = f32::NEG_INFINITY;
    let mut assign = vec![0i32; n + 1];
    fn rec(
        scores: &[Vec<f32>],
        n: usize,
        d: usize,
        assign: &mut [i32],
        best: &mut Vec<i32>,
        best_w: &mut f32,
    ) {
        if d > n {
            // well-formed: every node reaches root, no cycles.
            for i in 1..=n {
                let mut u = i as i32;
                let mut seen = vec![false; n + 1];
                while u != 0 {
                    if u < 0 || u as usize > n || seen[u as usize] {
                        return;
                    }
                    seen[u as usize] = true;
                    u = assign[u as usize];
                }
            }
            let w: f32 = (1..=n).map(|i| scores[i][assign[i] as usize]).sum();
            if w > *best_w {
                *best_w = w;
                best.copy_from_slice(assign);
            }
            return;
        }
        for h in 0..=n as i32 {
            if h as usize == d {
                continue;
            }
            assign[d] = h;
            rec(scores, n, d + 1, assign, best, best_w);
        }
    }
    rec(scores, n, 1, &mut assign, &mut best, &mut best_w);
    best[0] = -1;
    best
}
