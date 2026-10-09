//! TEMPORARY Stage-1 probe (delete after use): k-best second-order
//! Viterbi over the public scoring API + gold-in-k measurement.
//! Usage: `kbest -- <words> <gold> <k> [brute|oracle]`
//!   default: report 1-best accuracy + gold-in-k exact-path rate.
//!   `brute`: verify top-k against exhaustive enumeration on
//!   sentences with <= 4 tokens (score multisets must match
//!   exactly; each DP path rescored).
//!   `oracle`: min-token-loss path among the k (perfect-selection
//!   headroom for reranking, in tokens vs 1-best).

use english_pos::{Model, START1, START2, TAGS, features};

const HS: usize = 19;
const NST: usize = HS * HS;

fn hs(k: usize) -> &'static str {
    if k < 17 {
        TAGS[k]
    } else if k == 17 {
        START1
    } else {
        START2
    }
}

fn lower_of(words: &[String]) -> Vec<String> {
    words.iter().map(|w| w.to_lowercase()).collect()
}

fn acc_for(
    model: &Model,
    words: &[String],
    lower: &[String],
    i: usize,
    p1: usize,
    p2: usize,
) -> [f32; 17] {
    let mut feats = Vec::with_capacity(20);
    features(words, lower, i, hs(p1), hs(p2), &mut feats);
    let mut acc = [0.0f32; 17];
    for f in feats.iter() {
        if let Some(arr) = model.feature_weights(*f) {
            for (a, w) in acc.iter_mut().zip(arr.iter()) {
                *a += *w;
            }
        }
    }
    acc
}

fn path_score(model: &Model, words: &[String], tags: &[usize]) -> f32 {
    let lower = lower_of(words);
    let mut total = 0.0f32;
    for i in 0..words.len() {
        let (p1, p2) = (
            if i >= 1 { tags[i - 1] } else { 17 },
            if i >= 2 {
                tags[i - 2]
            } else if i == 1 {
                17
            } else {
                18
            },
        );
        total += acc_for(model, words, &lower, i, p1, p2)[tags[i]];
    }
    total
}

/// k-best full tag paths (deduped by tag sequence). Returns
/// (tags, score) sorted by score desc, ties by first-seen order.
fn kbest(model: &Model, words: &[String], k: usize) -> Vec<(Vec<usize>, f32)> {
    let n = words.len();
    let lower = lower_of(words);
    // dp[state] = top-k (score, prev_state, prev_rank)
    let mut dp: Vec<Vec<(f32, u16, u8)>> = vec![vec![]; NST];
    dp[17 * HS + 18].push((0.0, u16::MAX, 0));
    // choice[i][newstate*k + rank] = (prev_state, prev_rank)
    let mut choice: Vec<Vec<(u16, u8)>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut ndp: Vec<Vec<(f32, u16, u8)>> = vec![vec![]; NST];
        for p1 in 0..HS {
            for p2 in 0..HS {
                let st = p1 * HS + p2;
                if dp[st].is_empty() {
                    continue;
                }
                let acc = acc_for(model, words, &lower, i, p1, p2);
                for (r, e) in dp[st].iter().enumerate() {
                    for b in 0..17 {
                        ndp[b * HS + p1].push((e.0 + acc[b], st as u16, r as u8));
                    }
                }
            }
        }
        let mut ch = vec![(u16::MAX, 0u8); NST * k];
        for (ns, v) in ndp.iter_mut().enumerate() {
            // stable desc sort: ties keep (state-asc, rank-asc, tag-asc)
            // insertion order (states/ ranks/tags all iterated ascending).
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            v.truncate(k);
            for (r, e) in v.iter().enumerate() {
                ch[ns * k + r] = (e.1, e.2);
            }
        }
        choice.push(ch);
        dp = ndp;
    }
    // top-k over final states
    let mut finals: Vec<(f32, usize, usize)> = vec![];
    for (ns, v) in dp.iter().enumerate() {
        for (r, e) in v.iter().enumerate() {
            finals.push((e.0, ns, r));
        }
    }
    finals.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out = vec![];
    let mut seen = std::collections::HashSet::new();
    for (s, ns, r) in finals {
        if out.len() >= k {
            break;
        }
        // backtrack: state (b, p1) = (t{i}, t{i-1}); the choice
        // table recovers the previous state (p1, p2) + its rank.
        let mut tags = vec![0usize; n];
        let (mut bcur, mut pcur, mut rcur) = (ns / HS, ns % HS, r);
        for i in (0..n).rev() {
            tags[i] = bcur.min(16);
            let (ps, pr) = choice[i][(bcur.min(16) * HS + pcur.min(16)) * k + rcur.min(k - 1)];
            (bcur, pcur, rcur) = ((ps as usize) / HS, (ps as usize) % HS, pr as usize);
        }
        let _ = s;
        if seen.insert(tags.clone()) {
            let sc = path_score(model, words, &tags);
            out.push((tags, sc));
        }
    }
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out.truncate(k);
    out
}

fn blocks(path: &str) -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(path).expect("read");
    let mut out = vec![];
    let mut cur = vec![];
    for line in text.lines().chain([""]) {
        let line = line.trim_end();
        if line.is_empty() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        cur.push(line.to_string());
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = Model::from_json(include_str!("../weights/upos.json")).expect("weights");
    let sents = blocks(&args[1]);
    let golds = blocks(&args[2]);
    assert_eq!(sents.len(), golds.len());
    let k: usize = args[3].parse().expect("k");
    let brute = args.get(4).map(|s| s.as_str()) == Some("brute");
    if brute {
        let mut checked = 0usize;
        for (words, gold) in sents.iter().zip(golds.iter()) {
            if words.len() > 4 || words.is_empty() {
                continue;
            }
            let kb = kbest(&model, words, k.min(10));
            // exhaustive enumeration
            let mut all: Vec<(Vec<usize>, f32)> = vec![];
            let mut stack = vec![vec![]];
            for _ in 0..words.len() {
                let mut nx = vec![];
                for pre in stack.drain(..) {
                    for b in 0..17 {
                        let mut q = pre.clone();
                        q.push(b);
                        nx.push(q);
                    }
                }
                stack = nx;
            }
            for tags in stack.drain(..) {
                all.push((tags.clone(), path_score(&model, words, &tags)));
            }
            all.sort_by(|a, b| b.1.total_cmp(&a.1));
            let mut es: Vec<u32> = all
                .iter()
                .take(k.min(all.len()))
                .map(|e| e.1.to_bits())
                .collect();
            let mut ds: Vec<u32> = kb.iter().map(|e| e.1.to_bits()).collect();
            es.sort();
            ds.sort();
            assert_eq!(es, ds, "score multiset mismatch on {}", words.join(" "));
            // every DP path rescores to its claimed score
            for (tags, s) in kb.iter() {
                assert_eq!(*s, path_score(&model, words, tags), "rescore mismatch");
            }
            let _ = gold;
            checked += 1;
        }
        println!("brute-force verified on {checked} short sentences");
        return;
    }
    let gidx: Vec<Vec<usize>> = golds
        .iter()
        .map(|g| {
            g.iter()
                .map(|t| TAGS.iter().position(|c| *c == t.as_str()).unwrap())
                .collect()
        })
        .collect();
    let (mut ok1, mut n, mut in_k) = (0usize, 0usize, 0usize);
    let mut ok_oracle = 0usize;
    let oracle_mode = args.get(4).map(|s| s.as_str()) == Some("oracle");
    for (words, gold) in sents.iter().zip(gidx.iter()) {
        let kb = kbest(&model, words, k);
        n += words.len();
        for (j, g) in gold.iter().enumerate() {
            if kb[0].0[j] == *g {
                ok1 += 1;
            }
        }
        if kb.iter().any(|(t, _)| t == gold) {
            in_k += 1;
        }
        if oracle_mode {
            let best = kb
                .iter()
                .map(|(t, _)| t.iter().zip(gold.iter()).filter(|(a, b)| a == b).count())
                .max()
                .unwrap_or(0);
            ok_oracle += best;
        }
    }
    println!(
        "k={k}: 1-best {ok1}/{n} = {:.4}; gold-in-{k}: {in_k}/{} = {:.4}",
        ok1 as f64 / n as f64,
        sents.len(),
        in_k as f64 / sents.len() as f64
    );
    if oracle_mode {
        println!(
            "oracle-min-loss-{k}: {ok_oracle}/{n} = {:.4} (1-best {ok1}; delta {:+.4} = {:+} toks)",
            ok_oracle as f64 / n as f64,
            ok_oracle as f64 / n as f64 - ok1 as f64 / n as f64,
            ok_oracle as i64 - ok1 as i64,
        );
    }
}
