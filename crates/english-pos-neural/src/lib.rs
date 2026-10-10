//! Batch/save-pass neural POS tagger (BiLSTM over words + char-BiLSTM).
//!
//! Never the keystroke path — the averaged perceptron in `english-pos`
//! keeps that. This crate decodes whole sentences through dense
//! contextual representations (the R3-1 screen: dev 93.86 / test
//! 94.19 with zero tuning) and reports margins for the same gated
//! correction discipline.
//!
//! Inference backend is a trait: [`HandRolled`] (zero-dep f32, ships
//! first) implements [`RecurrentBackend`]; a candle backend may
//! implement it later behind a feature flag. Weights are Tier-1 lazy
//! assets (gitignored when admitted); the trainer stays /tmp Python.

use english_pos::Tag;
use std::collections::HashMap;

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Dot product: runtime-detected AVX 8-wide path on x86_64 (this
/// box has AVX but not AVX2 — `std::simd` is unstable and AVX2
/// intrinsics would be dead weight here), portable chunked path
/// everywhere else. Both orders re-gate the 0-diff parity test.
/// Public for `english-joint` (shared encoder math, no duplication).
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        use std::sync::OnceLock;
        static HAS_AVX: OnceLock<bool> = OnceLock::new();
        if *HAS_AVX.get_or_init(|| std::arch::is_x86_feature_detected!("avx")) {
            return unsafe { dot_avx(a, b) };
        }
    }
    dot_scalar(a, b)
}

/// Portable fallback (also the aarch64 path): `chunks_exact` (no
/// bounds checks) + four accumulators.
fn dot_scalar(a: &[f32], b: &[f32]) -> f32 {
    // `chunks_exact` (no bounds checks) + four accumulators.
    let mut acc = [0.0f32; 4];
    for (x, y) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        acc[0] += x[0] * y[0];
        acc[1] += x[1] * y[1];
        acc[2] += x[2] * y[2];
        acc[3] += x[3] * y[3];
    }
    let mut s = acc[0] + acc[1] + acc[2] + acc[3];
    let cut = a.len() - a.len() % 4;
    for (x, y) in a[cut..].iter().zip(b[cut..].iter()) {
        s += x * y;
    }
    s
}

/// AVX 8-wide dot (x86_64 with runtime detection; single accumulator
/// pair keeps the summation shape close to scalar).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn dot_avx(a: &[f32], b: &[f32]) -> f32 {
    use core::arch::x86_64::*;
    unsafe {
        let mut acc = _mm256_setzero_ps();
        let mut i = 0;
        while i + 8 <= a.len() {
            let x = _mm256_loadu_ps(a.as_ptr().add(i));
            let y = _mm256_loadu_ps(b.as_ptr().add(i));
            acc = _mm256_add_ps(acc, _mm256_mul_ps(x, y));
            i += 8;
        }
        let mut buf = [0.0f32; 8];
        _mm256_storeu_ps(buf.as_mut_ptr(), acc);
        let mut s: f32 = buf.iter().sum();
        while i < a.len() {
            s += a[i] * b[i];
            i += 1;
        }
        s
    }
}

/// One directed single-layer LSTM pass over `xs` (T × input dim).
/// Weight layout matches torch `nn.LSTM`: `w_ih` is (4H × in),
/// `w_hh` (4H × H), biases length 4H, gates ordered i,f,g,o.
fn lstm_dir(xs: &[Vec<f32>], p: &LstmParams<'_>) -> Vec<Vec<f32>> {
    let hidden = p.hidden;
    let input = xs[0].len();
    let mut h = vec![0.0f32; hidden];
    let mut c = vec![0.0f32; hidden];
    let mut out = vec![vec![0.0f32; hidden]; xs.len()];
    let mut gates = vec![0.0f32; 4 * hidden];
    // No index Vec: forward walks up, reverse walks down — same
    // visit order as before, so parity-neutral by construction.
    let mut t = if p.reverse { xs.len() - 1 } else { 0 };
    loop {
        for g in 0..4 * hidden {
            let row_ih = &p.w_ih[g * input..(g + 1) * input];
            let row_hh = &p.w_hh[g * hidden..(g + 1) * hidden];
            // f64 accumulation: oneDNN blocks/fuses f32 sums in an
            // order we cannot replicate, so sum wide and cast once —
            // near-tie logits then flip far less often.
            let mut s = p.b_ih[g] + p.b_hh[g];
            s += dot(row_ih, &xs[t]);
            s += dot(row_hh, &h);
            gates[g] = s;
        }
        for j in 0..hidden {
            let i = sigmoid(gates[j]);
            let f = sigmoid(gates[hidden + j]);
            let g = gates[2 * hidden + j].tanh();
            let o = sigmoid(gates[3 * hidden + j]);
            c[j] = f * c[j] + i * g;
            h[j] = o * c[j].tanh();
        }
        out[t] = h.clone();
        if p.reverse {
            if t == 0 {
                break;
            }
            t -= 1;
        } else {
            t += 1;
            if t == xs.len() {
                break;
            }
        }
    }
    out
}

/// Directed LSTM layer parameters (borrows the weight tables).
/// Bundled so the [`RecurrentBackend`] method stays lean — and so a
/// future quantized backend can carry scales alongside the tables.
pub struct LstmParams<'a> {
    pub w_ih: &'a [f32],
    pub w_hh: &'a [f32],
    pub b_ih: &'a [f32],
    pub b_hh: &'a [f32],
    pub hidden: usize,
    pub reverse: bool,
}

/// Recurrent inference backend (f32). One implementor ships
/// ([`HandRolled`]); candle may follow behind a feature flag.
pub trait RecurrentBackend {
    /// Directed LSTM layer; direction rides in `params`.
    fn lstm_layer(&self, xs: &[Vec<f32>], params: &LstmParams<'_>) -> Vec<Vec<f32>>;
    /// Directed LSTM layer over int8 weights with per-row scales;
    /// activations quantize per vector (Stage 3). Falls back to the
    /// f32 path by default; [`HandRolled`] implements it natively.
    fn lstm_layer_q(&self, xs: &[Vec<f32>], params: &QLstmParams<'_>) -> Vec<Vec<f32>> {
        let _ = (xs, params);
        unimplemented!("quantized forward not implemented for this backend");
    }
}

/// Zero-dependency f32 backend (ships first).
pub struct HandRolled;

impl RecurrentBackend for HandRolled {
    fn lstm_layer(&self, xs: &[Vec<f32>], params: &LstmParams<'_>) -> Vec<Vec<f32>> {
        lstm_dir(xs, params)
    }

    fn lstm_layer_q(&self, xs: &[Vec<f32>], params: &QLstmParams<'_>) -> Vec<Vec<f32>> {
        qlstm_dir(xs, params)
    }
}

/// Symmetric per-vector int8 quantization (absmax/127); zero vectors
/// take scale epsilon so the dequant stays finite.
fn quantize_row(x: &[f32]) -> (Vec<i8>, f32) {
    let mut mx = 0.0f32;
    for v in x {
        let a = v.abs();
        if a > mx {
            mx = a;
        }
    }
    let s = (mx / 127.0).max(1e-9);
    (
        x.iter()
            .map(|v| (v / s).round().clamp(-128.0, 127.0) as i8)
            .collect(),
        s,
    )
}

/// int8×int8 dot with i32 accumulation (autovec-friendly narrow lane
/// density: 16 int8 lanes where f32 gets 4 under SSE2).
fn qdot(w: &[i8], sw: f32, x: &[i8], sx: f32) -> f32 {
    let mut acc = [0i32; 4];
    for (a, b) in w.chunks_exact(4).zip(x.chunks_exact(4)) {
        acc[0] += a[0] as i32 * b[0] as i32;
        acc[1] += a[1] as i32 * b[1] as i32;
        acc[2] += a[2] as i32 * b[2] as i32;
        acc[3] += a[3] as i32 * b[3] as i32;
    }
    let mut s = acc[0] + acc[1] + acc[2] + acc[3];
    let cut = w.len() - w.len() % 4;
    for (a, b) in w[cut..].iter().zip(x[cut..].iter()) {
        s += *a as i32 * *b as i32;
    }
    s as f32 * sw * sx
}

/// Directed LSTM layer over per-row-quantized weights (Stage 3).
/// Input/hidden vectors quantize once per step and are reused across
/// all 4H gate rows; states stay f32.
fn qlstm_dir(xs: &[Vec<f32>], p: &QLstmParams<'_>) -> Vec<Vec<f32>> {
    let hidden = p.hidden;
    let input = xs[0].len();
    let mut h = vec![0.0f32; hidden];
    let mut c = vec![0.0f32; hidden];
    let mut out = vec![vec![0.0f32; hidden]; xs.len()];
    let mut gates = vec![0.0f32; 4 * hidden];
    let mut t = if p.reverse { xs.len() - 1 } else { 0 };
    loop {
        let (xq, sx) = quantize_row(&xs[t]);
        let (hq, sh) = quantize_row(&h);
        for g in 0..4 * hidden {
            let base_ih = g * input;
            let base_hh = g * hidden;
            let mut s = p.b_ih[g] + p.b_hh[g];
            s += qdot(&p.w_ih[base_ih..base_ih + input], p.s_ih[g], &xq, sx);
            s += qdot(&p.w_hh[base_hh..base_hh + hidden], p.s_hh[g], &hq, sh);
            gates[g] = s;
        }
        for j in 0..hidden {
            let i = sigmoid(gates[j]);
            let f = sigmoid(gates[hidden + j]);
            let g = gates[2 * hidden + j].tanh();
            let o = sigmoid(gates[3 * hidden + j]);
            c[j] = f * c[j] + i * g;
            h[j] = o * c[j].tanh();
        }
        out[t] = h.clone();
        if p.reverse {
            if t == 0 {
                break;
            }
            t -= 1;
        } else {
            t += 1;
            if t == xs.len() {
                break;
            }
        }
    }
    out
}

/// Directed quantized-LSTM parameters: int8 tables with per-row f32
/// scales (offline per-row absmax, same recipe as the preview).
pub struct QLstmParams<'a> {
    pub w_ih: &'a [i8],
    pub s_ih: &'a [f32],
    pub w_hh: &'a [i8],
    pub s_hh: &'a [f32],
    pub b_ih: &'a [f32],
    pub b_hh: &'a [f32],
    pub hidden: usize,
    pub reverse: bool,
}

fn get_f32(params: &HashMap<String, Vec<f32>>, name: &str) -> Result<Vec<f32>, String> {
    params
        .get(name)
        .cloned()
        .ok_or_else(|| format!("missing param {name}"))
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

/// Cache of word input vectors (word-embed + char-BiLSTM) by surface
/// form. The char encoder is ~1/5 of the flops and book text repeats
/// itself; callers that tag whole documents keep one across sentences
/// (the TagCache precedent). Passed `&mut` so `Model` stays `Sync`
/// for scoped-thread batch tagging.
#[derive(Default)]
pub struct WordCache {
    map: HashMap<String, Vec<f32>>,
}

impl WordCache {
    pub fn new() -> Self {
        WordCache {
            map: HashMap::new(),
        }
    }

    /// Cached word vector, if present (shared with `english-joint`).
    pub fn map_get(&self, word: &str) -> Option<Vec<f32>> {
        self.map.get(word).cloned()
    }

    /// Store a word vector.
    pub fn map_put(&mut self, word: &str, vec: Vec<f32>) {
        self.map.insert(word.to_string(), vec);
    }
}

/// BiLSTM tagger. Load from the export JSON
/// (`scripts`-side recipe, /tmp until admission); weights are
/// Tier-1 and score-gated, never md5-gated.
pub struct Model<B = HandRolled> {
    backend: B,
    tags: Vec<Tag>,
    words: HashMap<String, usize>,
    chars: HashMap<char, usize>,
    wemb: Vec<f32>,
    wemb_dim: usize,
    cemb: Vec<f32>,
    cemb_dim: usize,
    cenc_ih_f: Vec<f32>,
    cenc_hh_f: Vec<f32>,
    cenc_bih_f: Vec<f32>,
    cenc_bhh_f: Vec<f32>,
    cenc_ih_r: Vec<f32>,
    cenc_hh_r: Vec<f32>,
    cenc_bih_r: Vec<f32>,
    cenc_bhh_r: Vec<f32>,
    cenc_h: usize,
    wenc_ih_f: Vec<f32>,
    wenc_hh_f: Vec<f32>,
    wenc_bih_f: Vec<f32>,
    wenc_bhh_f: Vec<f32>,
    wenc_ih_r: Vec<f32>,
    wenc_hh_r: Vec<f32>,
    wenc_bih_r: Vec<f32>,
    wenc_bhh_r: Vec<f32>,
    wenc_h: usize,
    out_w: Vec<f32>,
    out_b: Vec<f32>,
    n_tags: usize,
    max_chars: usize,
}

impl Model<HandRolled> {
    /// Load from the export JSON text.
    pub fn from_json(text: &str) -> Result<Self, String> {
        Self::from_json_with(text, HandRolled)
    }
}

impl<B: RecurrentBackend> Model<B> {
    pub fn from_json_with(text: &str, backend: B) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let tags: Vec<Tag> = v["tags"]
            .as_array()
            .ok_or("missing tags")?
            .iter()
            .map(|t| Tag::from_upos(t.as_str().unwrap_or("")).ok_or_else(|| format!("bad tag {t}")))
            .collect::<Result<_, _>>()?;
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
        let arch = &v["arch"];
        let wemb_dim = arch["wemb"].as_u64().ok_or("arch.wemb")? as usize;
        let cemb_dim = arch["cemb"].as_u64().ok_or("arch.cemb")? as usize;
        let cenc_h = arch["cenc"].as_u64().ok_or("arch.cenc")? as usize;
        let wenc_h = arch["wenc"].as_u64().ok_or("arch.wenc")? as usize;
        let n_tags = tags.len();
        let wemb = get_f32(&params, "wemb.weight")?;
        let cemb = get_f32(&params, "cemb.weight")?;
        let out_w = get_f32(&params, "out.weight")?;
        let out_b = get_f32(&params, "out.bias")?;
        Ok(Model {
            backend,
            tags,
            words,
            chars,
            wemb,
            wemb_dim,
            cemb,
            cemb_dim,
            cenc_ih_f: get_f32(&params, "cenc.weight_ih_l0")?,
            cenc_hh_f: get_f32(&params, "cenc.weight_hh_l0")?,
            cenc_bih_f: get_f32(&params, "cenc.bias_ih_l0")?,
            cenc_bhh_f: get_f32(&params, "cenc.bias_hh_l0")?,
            cenc_ih_r: get_f32(&params, "cenc.weight_ih_l0_reverse")?,
            cenc_hh_r: get_f32(&params, "cenc.weight_hh_l0_reverse")?,
            cenc_bih_r: get_f32(&params, "cenc.bias_ih_l0_reverse")?,
            cenc_bhh_r: get_f32(&params, "cenc.bias_hh_l0_reverse")?,
            cenc_h,
            wenc_ih_f: get_f32(&params, "wenc.weight_ih_l0")?,
            wenc_hh_f: get_f32(&params, "wenc.weight_hh_l0")?,
            wenc_bih_f: get_f32(&params, "wenc.bias_ih_l0")?,
            wenc_bhh_f: get_f32(&params, "wenc.bias_hh_l0")?,
            wenc_ih_r: get_f32(&params, "wenc.weight_ih_l0_reverse")?,
            wenc_hh_r: get_f32(&params, "wenc.weight_hh_l0_reverse")?,
            wenc_bih_r: get_f32(&params, "wenc.bias_ih_l0_reverse")?,
            wenc_bhh_r: get_f32(&params, "wenc.bias_hh_l0_reverse")?,
            wenc_h,
            out_w,
            out_b,
            n_tags,
            max_chars: 24,
        })
    }

    fn embed_row(table: &[f32], dim: usize, id: usize) -> Vec<f32> {
        table[id * dim..(id + 1) * dim].to_vec()
    }

    /// Logits per token (T × 17).
    pub fn logits(&self, words: &[&str]) -> Vec<Vec<f32>> {
        self.logits_cached(&mut WordCache::new(), words)
    }

    /// Logits with a caller-kept [`WordCache`] across sentences.
    pub fn logits_cached(&self, cache: &mut WordCache, words: &[&str]) -> Vec<Vec<f32>> {
        // Char vectors: truncate like training, pad per-sentence to max
        // (torch ran padded PAD steps — replicate, don't skip).
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
            if let Some(hit) = cache.map.get(*w) {
                word_vecs.push(hit.clone());
                continue;
            }
            let lower = w.to_lowercase();
            let wid = *self.words.get(&lower).unwrap_or(&1);
            let cseq: Vec<Vec<f32>> = cids
                .iter()
                .map(|&c| Self::embed_row(&self.cemb, self.cemb_dim, c))
                .collect();
            let f = self.backend.lstm_layer(
                &cseq,
                &LstmParams {
                    w_ih: &self.cenc_ih_f,
                    w_hh: &self.cenc_hh_f,
                    b_ih: &self.cenc_bih_f,
                    b_hh: &self.cenc_bhh_f,
                    hidden: self.cenc_h,
                    reverse: false,
                },
            );
            let r = self.backend.lstm_layer(
                &cseq,
                &LstmParams {
                    w_ih: &self.cenc_ih_r,
                    w_hh: &self.cenc_hh_r,
                    b_ih: &self.cenc_bih_r,
                    b_hh: &self.cenc_bhh_r,
                    hidden: self.cenc_h,
                    reverse: true,
                },
            );
            let mut cv = f[clen - 1].clone();
            cv.extend_from_slice(&r[0]);
            let mut wv = Self::embed_row(&self.wemb, self.wemb_dim, wid);
            wv.extend(cv);
            cache.map.insert((*w).to_string(), wv.clone());
            word_vecs.push(wv);
        }
        let hf = self.backend.lstm_layer(
            &word_vecs,
            &LstmParams {
                w_ih: &self.wenc_ih_f,
                w_hh: &self.wenc_hh_f,
                b_ih: &self.wenc_bih_f,
                b_hh: &self.wenc_bhh_f,
                hidden: self.wenc_h,
                reverse: false,
            },
        );
        let hr = self.backend.lstm_layer(
            &word_vecs,
            &LstmParams {
                w_ih: &self.wenc_ih_r,
                w_hh: &self.wenc_hh_r,
                b_ih: &self.wenc_bih_r,
                b_hh: &self.wenc_bhh_r,
                hidden: self.wenc_h,
                reverse: true,
            },
        );
        let width = 2 * self.wenc_h;
        word_vecs
            .iter()
            .enumerate()
            .map(|(t, _)| {
                let mut h = hf[t].clone();
                h.extend_from_slice(&hr[t]);
                (0..self.n_tags)
                    .map(|k| {
                        let row = &self.out_w[k * width..(k + 1) * width];
                        dot(row, &h) + self.out_b[k]
                    })
                    .collect()
            })
            .collect()
    }

    fn best_two(logits: &[f32]) -> (usize, f32) {
        let mut b0 = 0usize;
        let mut b1 = 1usize;
        if logits[1] > logits[0] {
            b0 = 1;
            b1 = 0;
        }
        for (i, &s) in logits.iter().enumerate().skip(2) {
            if s > logits[b0] {
                b1 = b0;
                b0 = i;
            } else if s > logits[b1] {
                b1 = i;
            }
        }
        (b0, logits[b0] - logits[b1])
    }

    /// Greedy tags.
    pub fn tag(&self, words: &[&str]) -> Vec<Tag> {
        self.tag_cached(&mut WordCache::new(), words)
    }

    /// Greedy tags with a caller-kept [`WordCache`].
    pub fn tag_cached(&self, cache: &mut WordCache, words: &[&str]) -> Vec<Tag> {
        self.logits_cached(cache, words)
            .iter()
            .map(|l| self.tags[Self::best_two(l).0])
            .collect()
    }

    /// Greedy tags with best-minus-runner-up margins (feeds the
    /// same gated correction discipline as `english-pos`).
    pub fn tag_margins(&self, words: &[&str]) -> Vec<(Tag, f32)> {
        self.logits(words)
            .iter()
            .map(|l| {
                let (b, m) = Self::best_two(l);
                (self.tags[b], m)
            })
            .collect()
    }
}

fn json_to_i8(v: &serde_json::Value, out: &mut Vec<i8>) {
    match v {
        serde_json::Value::Number(n) => out.push(n.as_i64().unwrap_or(0) as i8),
        serde_json::Value::Array(a) => {
            for x in a {
                json_to_i8(x, out);
            }
        }
        _ => {}
    }
}

/// Quantized BiLSTM tagger (Stage 3): per-row int8 weights with f32
/// scales (offline absmax recipe), f32 biases and states, per-vector
/// activation quantization. Same API as [`Model`]; accuracy re-gates
/// every change (dev/test within run-wobble, sweep ≥ 0.87).
pub struct QModel<B = HandRolled> {
    backend: B,
    tags: Vec<Tag>,
    words: HashMap<String, usize>,
    chars: HashMap<char, usize>,
    wemb_q: Vec<i8>,
    wemb_s: Vec<f32>,
    wemb_dim: usize,
    cemb_q: Vec<i8>,
    cemb_s: Vec<f32>,
    cemb_dim: usize,
    cenc: [QDir; 2],
    wenc: [QDir; 2],
    cenc_h: usize,
    wenc_h: usize,
    out_q: Vec<i8>,
    out_s: Vec<f32>,
    out_b: Vec<f32>,
    n_tags: usize,
    max_chars: usize,
}

/// One direction's quantized tables: int8 weights + per-row scales +
/// f32 biases.
pub struct QDir {
    pub w_ih: Vec<i8>,
    pub s_ih: Vec<f32>,
    pub w_hh: Vec<i8>,
    pub s_hh: Vec<f32>,
    pub b_ih: Vec<f32>,
    pub b_hh: Vec<f32>,
}

fn get_q(
    qp: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<(Vec<i8>, Vec<f32>), String> {
    let e = qp
        .get(name)
        .ok_or_else(|| format!("missing qparam {name}"))?;
    let mut q = Vec::new();
    json_to_i8(&e["q"], &mut q);
    let mut s = Vec::new();
    json_to_f32(&e["s"], &mut s);
    Ok((q, s))
}

impl QModel<HandRolled> {
    /// Load from the quantized export JSON text.
    pub fn from_json(text: &str) -> Result<Self, String> {
        Self::from_json_with(text, HandRolled)
    }
}

impl<B: RecurrentBackend> QModel<B> {
    pub fn from_json_with(text: &str, backend: B) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let tags: Vec<Tag> = v["tags"]
            .as_array()
            .ok_or("missing tags")?
            .iter()
            .map(|t| Tag::from_upos(t.as_str().unwrap_or("")).ok_or_else(|| format!("bad tag {t}")))
            .collect::<Result<_, _>>()?;
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
        let qp = v["qparams"].as_object().ok_or("missing qparams")?;
        let bias = v["bias"].as_object().ok_or("missing bias")?;
        let arch = &v["arch"];
        let wemb_dim = arch["wemb"].as_u64().ok_or("arch.wemb")? as usize;
        let cemb_dim = arch["cemb"].as_u64().ok_or("arch.cemb")? as usize;
        let cenc_h = arch["cenc"].as_u64().ok_or("arch.cenc")? as usize;
        let wenc_h = arch["wenc"].as_u64().ok_or("arch.wenc")? as usize;
        // Torch names reverse-direction tables
        // `{base}.weight_ih_l0_reverse` (flat, not nested).
        let dir = |base: &str, rev: &str| -> Result<QDir, String> {
            let (w_ih, s_ih) = get_q(qp, &format!("{base}.weight_ih_l0{rev}"))?;
            let (w_hh, s_hh) = get_q(qp, &format!("{base}.weight_hh_l0{rev}"))?;
            let mut bih = Vec::new();
            let mut bhh = Vec::new();
            json_to_f32(
                bias.get(&format!("{base}.bias_ih_l0{rev}"))
                    .ok_or("bias_ih")?,
                &mut bih,
            );
            json_to_f32(
                bias.get(&format!("{base}.bias_hh_l0{rev}"))
                    .ok_or("bias_hh")?,
                &mut bhh,
            );
            Ok(QDir {
                w_ih,
                s_ih,
                w_hh,
                s_hh,
                b_ih: bih,
                b_hh: bhh,
            })
        };
        let (out_q, out_s) = get_q(qp, "out.weight")?;
        let mut out_b = Vec::new();
        json_to_f32(bias.get("out.bias").ok_or("out.bias")?, &mut out_b);
        let n_tags = tags.len();
        let (wemb_q, wemb_s) = get_q(qp, "wemb.weight")?;
        let (cemb_q, cemb_s) = get_q(qp, "cemb.weight")?;
        Ok(QModel {
            backend,
            tags,
            words,
            chars,
            wemb_q,
            wemb_s,
            wemb_dim,
            cemb_q,
            cemb_s,
            cemb_dim,
            cenc: [dir("cenc", "")?, dir("cenc", "_reverse")?],
            wenc: [dir("wenc", "")?, dir("wenc", "_reverse")?],
            cenc_h,
            wenc_h,
            out_q,
            out_s,
            out_b,
            n_tags,
            max_chars: 24,
        })
    }

    fn embed_row(table: &[i8], scales: &[f32], dim: usize, id: usize) -> Vec<f32> {
        let s = scales[id];
        table[id * dim..(id + 1) * dim]
            .iter()
            .map(|q| *q as f32 * s)
            .collect()
    }

    fn qparams<'s>(d: &'s QDir, hidden: usize, reverse: bool) -> QLstmParams<'s> {
        QLstmParams {
            w_ih: &d.w_ih,
            s_ih: &d.s_ih,
            w_hh: &d.w_hh,
            s_hh: &d.s_hh,
            b_ih: &d.b_ih,
            b_hh: &d.b_hh,
            hidden,
            reverse,
        }
    }

    /// Logits per token (T × 17).
    pub fn logits(&self, words: &[&str]) -> Vec<Vec<f32>> {
        self.logits_cached(&mut WordCache::new(), words)
    }

    /// Logits with a caller-kept [`WordCache`] across sentences.
    pub fn logits_cached(&self, cache: &mut WordCache, words: &[&str]) -> Vec<Vec<f32>> {
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
            if let Some(hit) = cache.map.get(*w) {
                word_vecs.push(hit.clone());
                continue;
            }
            let lower = w.to_lowercase();
            let wid = *self.words.get(&lower).unwrap_or(&1);
            let cseq: Vec<Vec<f32>> = cids
                .iter()
                .map(|&c| Self::embed_row(&self.cemb_q, &self.cemb_s, self.cemb_dim, c))
                .collect();
            let f = self
                .backend
                .lstm_layer_q(&cseq, &Self::qparams(&self.cenc[0], self.cenc_h, false));
            let r = self
                .backend
                .lstm_layer_q(&cseq, &Self::qparams(&self.cenc[1], self.cenc_h, true));
            let mut cv = f[clen - 1].clone();
            cv.extend_from_slice(&r[0]);
            let mut wv = Self::embed_row(&self.wemb_q, &self.wemb_s, self.wemb_dim, wid);
            wv.extend(cv);
            cache.map.insert((*w).to_string(), wv.clone());
            word_vecs.push(wv);
        }
        let hf = self.backend.lstm_layer_q(
            &word_vecs,
            &Self::qparams(&self.wenc[0], self.wenc_h, false),
        );
        let hr = self
            .backend
            .lstm_layer_q(&word_vecs, &Self::qparams(&self.wenc[1], self.wenc_h, true));
        let width = 2 * self.wenc_h;
        word_vecs
            .iter()
            .enumerate()
            .map(|(t, _)| {
                let mut h = hf[t].clone();
                h.extend_from_slice(&hr[t]);
                let (hq, sh) = quantize_row(&h);
                (0..self.n_tags)
                    .map(|k| {
                        let base = k * width;
                        qdot(&self.out_q[base..base + width], self.out_s[k], &hq, sh)
                            + self.out_b[k]
                    })
                    .collect()
            })
            .collect()
    }

    fn best_two(logits: &[f32]) -> (usize, f32) {
        let mut b0 = 0usize;
        let mut b1 = 1usize;
        if logits[1] > logits[0] {
            b0 = 1;
            b1 = 0;
        }
        for (i, &s) in logits.iter().enumerate().skip(2) {
            if s > logits[b0] {
                b1 = b0;
                b0 = i;
            } else if s > logits[b1] {
                b1 = i;
            }
        }
        (b0, logits[b0] - logits[b1])
    }

    /// Greedy tags.
    pub fn tag(&self, words: &[&str]) -> Vec<Tag> {
        self.tag_cached(&mut WordCache::new(), words)
    }

    /// Greedy tags with a caller-kept [`WordCache`].
    pub fn tag_cached(&self, cache: &mut WordCache, words: &[&str]) -> Vec<Tag> {
        self.logits_cached(cache, words)
            .iter()
            .map(|l| self.tags[Self::best_two(l).0])
            .collect()
    }

    /// Greedy tags with best-minus-runner-up margins.
    pub fn tag_margins(&self, words: &[&str]) -> Vec<(Tag, f32)> {
        self.logits(words)
            .iter()
            .map(|l| {
                let (b, m) = Self::best_two(l);
                (self.tags[b], m)
            })
            .collect()
    }
}
