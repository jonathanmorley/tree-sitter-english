//! Joint neural tagger-parser (R3-5 Stage 1): shared word+char BiLSTM
//! encoder (R3-1 screen dims) with UPOS + biaffine arc/label heads,
//! Chu-Liu-Edmonds MST at decode. Batch/save-pass only — the
//! perceptron keeps keystroke, the arc-eager decoders keep their
//! committed behavior; this crate is the addressed-to-cause cascade
//! answer, admitted only on its staged bars (`docs/joint-neural-scope.md`).
//!
//! Encoder math reuses `english-pos-neural` (`HandRolled` backend,
//! `LstmParams`, `dot`, `WordCache`); orchestration + heads + MST
//! live here so the admitted tagger crate stays frozen. Weights are
//! Tier-1 (/tmp until admission), score-gated never md5-gated.

use english_pos::Tag;
use english_pos_neural::{HandRolled, LstmParams, RecurrentBackend, WordCache, dot};
use std::collections::HashMap;

/// Maximum arborescence over `scores[dep][head]` (n+1 nodes, `root`
/// excluded from incoming). Standard Chu-Liu/Edmonds with
/// contraction. Ported from the screen probe; the unit test checks
/// it against brute force on seeded random graphs.
pub fn chu_liu_edmonds(scores: &[Vec<f32>], root: usize) -> Vec<i32> {
    let n = scores.len() - 1;
    let mut pre = best_in(scores, root);
    // Find a cycle in the greedy picks.
    let mut vis = vec![-1i32; n + 1];
    let mut cyc: Vec<usize> = Vec::new();
    for s in 0..=n {
        if s == root {
            continue;
        }
        let mut u = s as i32;
        while u != root as i32 && vis[u as usize] == -1 {
            vis[u as usize] = s as i32;
            u = pre[u as usize];
        }
        if u != root as i32 && vis[u as usize] == s as i32 {
            let mut v = u as usize;
            loop {
                cyc.push(v);
                v = pre[v] as usize;
                if v == u as usize {
                    break;
                }
            }
            break;
        }
    }
    if cyc.is_empty() {
        return pre;
    }
    let in_c: std::collections::HashSet<usize> = cyc.iter().copied().collect();
    let rest: Vec<usize> = (0..=n).filter(|i| !in_c.contains(i)).collect();
    let mut new_id = vec![0usize; n + 1];
    for (k, i) in rest.iter().enumerate() {
        new_id[*i] = k;
    }
    let cn = rest.len();
    let m = rest.len();
    let new_root = new_id[root];
    let mut ns = vec![vec![f32::NEG_INFINITY; m + 1]; m + 1];
    let mut edge = HashMap::new();
    for i in 0..=n {
        for j in 0..=n {
            if i == j {
                continue;
            }
            let ci = if in_c.contains(&i) { cn } else { new_id[i] };
            let cj = if in_c.contains(&j) { cn } else { new_id[j] };
            if ci == cj {
                continue;
            }
            let mut w = scores[i][j];
            if in_c.contains(&i) {
                w -= best_in_w(scores, i);
            }
            if w > ns[ci][cj] {
                ns[ci][cj] = w;
                edge.insert((ci, cj), (i, j));
            }
        }
    }
    let sub = chu_liu_edmonds(&ns, new_root);
    let mut out = vec![-1i32; n + 1];
    for ci in 0..=m {
        let pj = sub[ci];
        if pj < 0 || ci == pj as usize {
            continue;
        }
        let (i, j) = edge[&(ci, pj as usize)];
        out[i] = j as i32;
    }
    for i in cyc {
        if out[i] == -1 {
            out[i] = pre[i];
        }
    }
    out[root] = -1;
    out
}

fn best_in(scores: &[Vec<f32>], root: usize) -> Vec<i32> {
    let mut pre = vec![-1i32; scores.len()];
    for i in 0..scores.len() {
        if i == root {
            continue;
        }
        let mut b = f32::NEG_INFINITY;
        let mut bj = -1i32;
        for j in 0..scores.len() {
            if j != i && scores[i][j] > b {
                b = scores[i][j];
                bj = j as i32;
            }
        }
        pre[i] = bj;
    }
    pre
}

fn best_in_w(scores: &[Vec<f32>], i: usize) -> f32 {
    let mut b = f32::NEG_INFINITY;
    for j in 0..scores.len() {
        if j != i && scores[i][j] > b {
            b = scores[i][j];
        }
    }
    b
}

fn json_to_f32(v: &serde_json::Value, out: &mut Vec<f32>) {
    match v {
        serde_json::Value::Number(n) => out.push(n.as_f64().unwrap_or(0.0) as f32),
        serde_json::Value::Array(a) => {
            for x in a {
                json_to_f32(x, out);
            }
        }
        _ => {}
    }
}

fn get(params: &HashMap<String, Vec<f32>>, name: &str) -> Result<Vec<f32>, String> {
    params
        .get(name)
        .cloned()
        .ok_or_else(|| format!("missing param {name}"))
}

/// Joint tagger-parser. Load from the export JSON; weights Tier-1.
pub struct JointModel {
    backend: HandRolled,
    tags: Vec<Tag>,
    labels: Vec<String>,
    words: HashMap<String, usize>,
    chars: HashMap<char, usize>,
    wemb: Vec<f32>,
    cemb: Vec<f32>,
    cenc: [Dir; 2],
    wenc: [Dir; 2],
    tag_w: Vec<f32>,
    tag_b: Vec<f32>,
    arc_hw: Vec<f32>,
    arc_hb: Vec<f32>,
    arc_dw: Vec<f32>,
    arc_db: Vec<f32>,
    arc_u: Vec<f32>,
    arc_b: f32,
    arc_root: Vec<f32>,
    lab_hw: Vec<f32>,
    lab_hb: Vec<f32>,
    lab_dw: Vec<f32>,
    lab_db: Vec<f32>,
    lab_u: Vec<f32>,
    lab_b: Vec<f32>,
    lab_root: Vec<f32>,
    n_lab: usize,
    max_chars: usize,
}

/// One direction's LSTM tables.
struct Dir {
    w_ih: Vec<f32>,
    w_hh: Vec<f32>,
    b_ih: Vec<f32>,
    b_hh: Vec<f32>,
    hidden: usize,
}

impl JointModel {
    /// Load from the export JSON text.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let tags: Vec<Tag> = v["tags"]
            .as_array()
            .ok_or("missing tags")?
            .iter()
            .map(|t| Tag::from_upos(t.as_str().unwrap_or("")).ok_or_else(|| format!("bad tag {t}")))
            .collect::<Result<_, _>>()?;
        let labels: Vec<String> = v["labels"]
            .as_array()
            .ok_or("missing labels")?
            .iter()
            .map(|t| Ok(t.as_str().unwrap_or("").to_string()))
            .collect::<Result<_, String>>()?;
        let words: HashMap<String, usize> = v["words"]
            .as_object()
            .ok_or("missing words")?
            .iter()
            .map(|(k, n)| (k.clone(), n.as_u64().unwrap_or(1) as usize))
            .collect();
        let chars: HashMap<char, usize> = v["chars"]
            .as_object()
            .ok_or("missing chars")?
            .iter()
            .filter_map(|(k, n)| {
                k.chars()
                    .next()
                    .map(|c| (c, n.as_u64().unwrap_or(1) as usize))
            })
            .collect();
        let mut params: HashMap<String, Vec<f32>> = HashMap::new();
        for (k, val) in v["params"].as_object().ok_or("missing params")? {
            let mut flat = Vec::new();
            json_to_f32(val, &mut flat);
            params.insert(k.clone(), flat);
        }
        let dir = |base: &str, rev: &str, hidden: usize| -> Result<Dir, String> {
            Ok(Dir {
                w_ih: get(&params, &format!("{base}.weight_ih_l0{rev}"))?,
                w_hh: get(&params, &format!("{base}.weight_hh_l0{rev}"))?,
                b_ih: get(&params, &format!("{base}.bias_ih_l0{rev}"))?,
                b_hh: get(&params, &format!("{base}.bias_hh_l0{rev}"))?,
                hidden,
            })
        };
        let n_lab = labels.len();
        Ok(JointModel {
            backend: HandRolled,
            tags,
            labels: labels.clone(),
            words,
            chars,
            wemb: get(&params, "wemb.weight")?,
            cemb: get(&params, "cemb.weight")?,
            cenc: [dir("cenc", "", 32)?, dir("cenc", "_reverse", 32)?],
            wenc: [dir("wenc", "", 128)?, dir("wenc", "_reverse", 128)?],
            tag_w: get(&params, "tag_out.weight")?,
            tag_b: get(&params, "tag_out.bias")?,
            arc_hw: get(&params, "arc_h.weight")?,
            arc_hb: get(&params, "arc_h.bias")?,
            arc_dw: get(&params, "arc_d.weight")?,
            arc_db: get(&params, "arc_d.bias")?,
            arc_u: get(&params, "arc_U")?,
            arc_b: get(&params, "arc_b")?[0],
            arc_root: get(&params, "arc_root")?,
            lab_hw: get(&params, "lab_h.weight")?,
            lab_hb: get(&params, "lab_h.bias")?,
            lab_dw: get(&params, "lab_d.weight")?,
            lab_db: get(&params, "lab_d.bias")?,
            lab_u: get(&params, "lab_U")?,
            lab_b: get(&params, "lab_b")?,
            lab_root: get(&params, "lab_root")?,
            n_lab: labels.len(),
            max_chars: 24,
        })
    }

    fn affine(w: &[f32], b: &[f32], x: &[f32], out_dim: usize) -> Vec<f32> {
        let width = x.len();
        (0..out_dim)
            .map(|k| dot(&w[k * width..(k + 1) * width], x) + b[k])
            .collect()
    }

    fn lstm(&self, xs: &[Vec<f32>], d: &Dir, reverse: bool) -> Vec<Vec<f32>> {
        self.backend.lstm_layer(
            xs,
            &LstmParams {
                w_ih: &d.w_ih,
                w_hh: &d.w_hh,
                b_ih: &d.b_ih,
                b_hh: &d.b_hh,
                hidden: d.hidden,
                reverse,
            },
        )
    }

    fn embed_row(table: &[f32], dim: usize, id: usize) -> Vec<f32> {
        table[id * dim..(id + 1) * dim].to_vec()
    }

    /// Shared encoder states (T × 256), with caller-kept cache.
    fn encode(&self, cache: &mut WordCache, words: &[&str]) -> Vec<Vec<f32>> {
        let mut ch_ids: Vec<Vec<usize>> = words
            .iter()
            .map(|w| {
                let mut ids: Vec<usize> = w
                    .chars()
                    .take(self.max_chars)
                    .map(|c| *self.chars.get(&c).unwrap_or(&1))
                    .collect();
                if ids.is_empty() {
                    ids.push(0);
                }
                ids
            })
            .collect();
        let clen = ch_ids.iter().map(Vec::len).max().unwrap_or(1);
        for ids in &mut ch_ids {
            ids.resize(clen, 0);
        }
        let mut word_vecs = Vec::with_capacity(words.len());
        for (w, cids) in words.iter().zip(ch_ids.iter()) {
            if let Some(hit) = cache.map_get(w) {
                word_vecs.push(hit);
                continue;
            }
            let lower = w.to_lowercase();
            let wid = *self.words.get(&lower).unwrap_or(&1);
            let cseq: Vec<Vec<f32>> = cids
                .iter()
                .map(|&c| Self::embed_row(&self.cemb, 16, c))
                .collect();
            let f = self.lstm(&cseq, &self.cenc[0], false);
            let r = self.lstm(&cseq, &self.cenc[1], true);
            let mut cv = f[clen - 1].clone();
            cv.extend_from_slice(&r[0]);
            let mut wv = Self::embed_row(&self.wemb, 64, wid);
            wv.extend(cv);
            cache.map_put(w, wv.clone());
            word_vecs.push(wv);
        }
        let hf = self.lstm(&word_vecs, &self.wenc[0], false);
        let hr = self.lstm(&word_vecs, &self.wenc[1], true);
        hf.iter()
            .zip(hr.iter())
            .map(|(a, b)| {
                let mut h = a.clone();
                h.extend_from_slice(b);
                h
            })
            .collect()
    }

    /// Arc score matrix (T+1 slots, slot 0 = root; self-loops -inf).
    /// Factored: U·head precomputed per slot (same math, reordered
    /// sums — the parity test re-gates the order).
    fn arc_scores(&self, states: &[Vec<f32>]) -> Vec<Vec<f32>> {
        let t = states.len();
        // Head reps: slot 0 = learned root vector, used RAW (torch
        // prepends arc_root before... precisely, torch never runs it
        // through the head MLP — mirror that exactly).
        let mut heads: Vec<Vec<f32>> = Vec::with_capacity(t + 1);
        heads.push(self.arc_root.clone());
        for h in states {
            heads.push(Self::affine(&self.arc_hw, &self.arc_hb, h, 128));
        }
        // U·head per slot.
        let uh: Vec<Vec<f32>> = heads
            .iter()
            .map(|hh| {
                (0..128)
                    .map(|j| dot(&self.arc_u[j * 128..(j + 1) * 128], hh))
                    .collect()
            })
            .collect();
        let mut s = vec![vec![0.0f32; t + 1]; t + 1];
        for (d, h) in states.iter().enumerate() {
            let dep = Self::affine(&self.arc_dw, &self.arc_db, h, 128);
            for hh in 0..=t {
                s[d + 1][hh] = if hh == d + 1 {
                    f32::NEG_INFINITY
                } else {
                    dot(&dep, &uh[hh]) + self.arc_b
                };
            }
        }
        s
    }

    /// Relation logits for dependent `ld` (post-MLP) over head `lh`
    /// (post-MLP, or raw lab_root for slot 0 — torch never runs the
    /// root through the MLP on either side).
    fn lab_scores(&self, ld: &[f32], lh: &[f32]) -> Vec<f32> {
        (0..self.n_lab)
            .map(|l| {
                let base = l * 32 * 32;
                let mut tmp = vec![0.0f32; 32];
                for j in 0..32 {
                    tmp[j] = dot(&self.lab_u[base + j * 32..base + (j + 1) * 32], lh);
                }
                dot(ld, &tmp) + self.lab_b[l]
            })
            .collect()
    }

    /// Full decode: tags, head indices (-1 = root), relation labels.
    pub fn parse(&self, words: &[&str]) -> Parsed {
        self.parse_cached(&mut WordCache::new(), words)
    }

    /// Decode with a caller-kept [`WordCache`].
    pub fn parse_cached(&self, cache: &mut WordCache, words: &[&str]) -> Parsed {
        let states = self.encode(cache, words);
        let t = states.len();
        let tags: Vec<Tag> = states
            .iter()
            .map(|h| {
                let mut best = 0;
                let mut bs = f32::NEG_INFINITY;
                for k in 0..self.tags.len() {
                    let s = dot(&self.tag_w[k * 256..(k + 1) * 256], h) + self.tag_b[k];
                    if s > bs {
                        bs = s;
                        best = k;
                    }
                }
                self.tags[best]
            })
            .collect();
        let scores = self.arc_scores(&states);
        let heads_mst = chu_liu_edmonds(&scores, 0);
        // Label-side head reps: slot 0 = lab_root RAW (never through
        // the MLP, exactly like the arc side); token slots affine.
        let mut lh_slots: Vec<Vec<f32>> = Vec::with_capacity(t + 1);
        lh_slots.push(self.lab_root.clone());
        for h in &states {
            lh_slots.push(Self::affine(&self.lab_hw, &self.lab_hb, h, 32));
        }
        let mut head_states: Vec<Vec<f32>> = Vec::with_capacity(t + 1);
        head_states.push(self.lab_root.clone());
        head_states.extend(states.iter().cloned());
        let mut heads = Vec::with_capacity(t);
        let mut rels = Vec::with_capacity(t);
        for d in 0..t {
            let hh = heads_mst[d + 1] as usize;
            heads.push(hh as i32 - 1);
            let ld = Self::affine(&self.lab_dw, &self.lab_db, &states[d], 32);
            let ls = self.lab_scores(&ld, &lh_slots[hh]);
            let mut bl = 0;
            let mut bs = f32::NEG_INFINITY;
            for (k, s) in ls.iter().enumerate() {
                if *s > bs {
                    bs = *s;
                    bl = k;
                }
            }
            rels.push(self.labels[bl].clone());
        }
        Parsed { tags, heads, rels }
    }
}

/// One decoded sentence: tags, head indices (-1 = root), UD labels.
pub struct Parsed {
    pub tags: Vec<Tag>,
    pub heads: Vec<i32>,
    pub rels: Vec<String>,
}
