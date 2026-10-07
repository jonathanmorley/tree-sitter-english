//! Greedy arc-eager dependency parsing over tagged pieces.
//!
//! The fifth pipeline stage (never grammar — Tier 3 showed word
//! classes and phrase structure do not belong there, and argument
//! structure belongs there even less): given words plus predicted
//! UPOS tags, attach each token to its head, left to right, with a
//! perceptron over parser-configuration features. Unlabeled in
//! this stage (heads only — UAS); relation labels follow once the
//! arc machinery holds its bar.
//!
//! Design mirrors `english-pos`: 64-bit FNV-1a hashed features,
//! dense per-action rows, zero per-transition allocation (scratch
//! buffer reused), deterministic JSON weights. The static oracle
//! below is textbook Nivre arc-eager: Left-Arc fires only on the
//! gold arc *with all of the dependent's own gold children already
//! attached* (projectivity guard — without it a popped dependent
//! strands its right children); Reduce fires on an already-headed
//! stack top (which is why root, headless forever, is never
//! popped); otherwise Shift.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// FNV-1a 64-bit, mirroring `english-pos` (stable across processes,
/// so committed weight keys stay valid; separate hasher instance,
/// same constants).
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[derive(Default)]
struct U64Hasher(u64);

impl Hasher for U64Hasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut h = FNV_OFFSET_BASIS;
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
        self.0 = h;
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = v;
    }

    fn finish(&self) -> u64 {
        let mut z = self.0.wrapping_add(0x9e3779b97f4a7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

type U64Map<V> = HashMap<u64, V, BuildHasherDefault<U64Hasher>>;

/// Hash one feature template (same namespace discipline as the
/// tagger: discriminant + 0xff-separated parts).
pub fn hash_feature(discriminant: u8, parts: &[&str]) -> u64 {
    let mut h = FNV_OFFSET_BASIS;
    h ^= discriminant as u64;
    h = h.wrapping_mul(FNV_PRIME);
    for part in parts {
        for b in part.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
        h ^= 0xff;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Parser actions in fixed order (decoding tie-breaks and JSON
/// stability). Unlabeled: arc direction only, no relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Shift,
    Left,
    Right,
    Reduce,
}

impl Action {
    pub const ALL: [Action; 4] = [Action::Shift, Action::Left, Action::Right, Action::Reduce];

    pub fn code(self) -> &'static str {
        match self {
            Action::Shift => "SHIFT",
            Action::Left => "LEFT",
            Action::Right => "RIGHT",
            Action::Reduce => "REDUCE",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "SHIFT" => Action::Shift,
            "LEFT" => Action::Left,
            "RIGHT" => Action::Right,
            "REDUCE" => Action::Reduce,
            _ => return None,
        })
    }
}

/// Arc-eager parser configuration over 1-based token indices
/// (0 = root). `heads[i]` is the head of token `i`, or `usize::MAX`
/// while unattached (a fallback Reduce can strand a token after a
/// model mistake — MAX never matches gold, so UAS stays honest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub stack: Vec<usize>,
    pub buf: usize,
    pub heads: Vec<usize>,
    pub attached: Vec<bool>,
}

impl Config {
    /// Fresh configuration for an `n`-token sentence: `[0]` on the
    /// stack, buffer at 1, nothing attached.
    pub fn initial(n: usize) -> Self {
        Self {
            stack: vec![0],
            buf: 1,
            heads: vec![usize::MAX; n + 1],
            attached: vec![false; n + 1],
        }
    }

    /// Terminal: buffer exhausted and only root remains.
    pub fn is_terminal(&self, n: usize) -> bool {
        self.buf > n && self.stack.len() == 1
    }

    /// Legal actions in this configuration (grammar of the machine
    /// itself — independent of any model or gold tree). Includes a
    /// deadlock fallback: buffer exhausted with an unattached stack
    /// top otherwise strands the parse (reachable only off the
    /// oracle path, i.e. after a model mistake) — Reduce pops it,
    /// leaving it headless rather than looping forever.
    pub fn legal(&self, n: usize) -> [bool; 4] {
        let shift = self.buf <= n;
        let s0 = self.stack.last().copied().unwrap_or(0);
        let reduce = (s0 != 0 && self.attached[s0]) || (self.stack.len() > 1 && self.buf > n);
        let left = s0 != 0 && self.buf <= n;
        let right = !self.stack.is_empty() && self.buf <= n;
        [shift, left, right, reduce]
    }

    /// Apply an action (caller guarantees legality).
    pub fn apply(&mut self, a: Action) {
        match a {
            Action::Shift => {
                self.stack.push(self.buf);
                self.buf += 1;
            }
            Action::Left => {
                let s0 = self.stack.pop().expect("Left on empty stack");
                self.heads[s0] = self.buf;
                self.attached[s0] = true;
            }
            Action::Right => {
                let s0 = *self.stack.last().expect("Right on empty stack");
                self.heads[self.buf] = s0;
                self.attached[self.buf] = true;
                self.stack.push(self.buf);
                self.buf += 1;
            }
            Action::Reduce => {
                self.stack.pop().expect("Reduce on empty stack");
            }
        }
    }
}

/// Static arc-eager oracle: the gold action in this configuration.
/// Left-Arc fires only on the gold arc with the dependent's own
/// gold children already attached (the projectivity guard);
/// Reduce fires on an already-headed stack top whose own gold
/// children are also all attached (popping early would strand a
/// right dependent still in the buffer — same guard, both pops);
/// else Shift. Correct for projective trees: the walk reproduces
/// every gold arc.
pub fn oracle(gold_heads: &[usize], cfg: &Config) -> Action {
    let s0 = *cfg.stack.last().expect("oracle on empty stack");
    // All gold children of s0 already attached?
    let mut clear = true;
    for (k, &h) in gold_heads.iter().enumerate().skip(1) {
        if h == s0 && !cfg.attached[k] {
            clear = false;
            break;
        }
    }
    // Left-Arc: gold arc (buf -> s0), dependent's children done.
    if cfg.buf < gold_heads.len() && gold_heads[s0] == cfg.buf && clear {
        return Action::Left;
    }
    // Right-Arc: gold arc (s0 -> buf).
    if cfg.buf < gold_heads.len() && s0 < gold_heads.len() && gold_heads[cfg.buf] == s0 {
        return Action::Right;
    }
    // Reduce: headed top with no stranded children (root never:
    // headless forever, and the guard covers the rest).
    if s0 != 0 && cfg.attached[s0] && clear {
        return Action::Reduce;
    }
    // Fallthrough Shift needs a buffer token; past the end (only
    // reachable on non-projective gold, which the trainer filters,
    // or a hand-built test config) pop instead of pushing buf past
    // the sentence — stranding beats indexing out of bounds.
    if cfg.buf < gold_heads.len() {
        Action::Shift
    } else {
        Action::Reduce
    }
}

/// Feature templates over the configuration. Words are pre-lowered
/// by the caller (training reads UD FORM lowercased); tags are UPOS
/// codes. Same zero-alloc discipline as the tagger: text hashed in
/// place, scratch buffer reused. Discriminants: `0x30` s0 word,
/// `0x31` s0 tag, `0x32`/`0x33` s1 word/tag, `0x34` s2 tag,
/// `0x35`/`0x36` b0 word/tag, `0x37` b1 tag, `0x38`-`0x3b`
/// s0/s1 outer-dependent tags, `0x3c` s0+b0 tag bigram, `0x3d`
/// buffer distance bucket, `0x3e` b1 word, `0x3f`/`0x40` b2
/// word/tag, `0x41`/`0x42` s0 outer-dependent words,
/// `0x43`/`0x44` b0 left-dependent tag/word, `0x45` s0 word+b0
/// tag, `0x46` b0 word+s0 tag, `0x47` s1+s0 tag bigram,
/// `0x49`-`0x4b` valency buckets, `0x4c` s2 word, `0x4d`/`0x4e`
/// s0 head word/tag (when attached), `0x4f` s0 left-sibling tag,
/// `0x50` s0+b0 word bigram, `0x53`-`0x58` sentence neighbors of
/// the salient pair (MaltOptimizer step 4: predecessor/successor
/// word+tag for s0 and b0 — b0+1 rides the existing lookahead,
/// so the three added positions are s0−1, s0+1, b0−1).
pub fn features(words: &[String], tags: &[String], cfg: &Config, feats: &mut Vec<u64>) {
    feats.clear();
    let n = words.len();
    let word_at = |i: usize| -> &str {
        if i >= 1 && i <= n {
            words[i - 1].as_str()
        } else if i == 0 {
            // MEASURED 2026-10-07: no gain (dev 79.09→79.05, test
            // 79.34→79.47 — noise). Root/NULL aliasing is not the
            // binding constraint; the split stays for debuggability
            // (root states read distinctly in feature dumps), not
            // for accuracy.
            "<ROOT>"
        } else {
            "<NULL>"
        }
    };
    let tag_at = |i: usize| -> &str {
        if i >= 1 && i <= n {
            tags[i - 1].as_str()
        } else if i == 0 {
            "<ROOT>"
        } else {
            "<NULL>"
        }
    };
    // Outermost two dependent indices of a stack token on one
    // side (outermost first; Nones when childless there); linear
    // scan is O(n) worst-case per extract — the train loop, not
    // the hot path, pays it most. Callers read word/tag through
    // `word_at`/`tag_at`.
    let outer2 = |s: usize, left: bool| -> (Option<usize>, Option<usize>) {
        let mut best: Option<usize> = None;
        let mut second: Option<usize> = None;
        for k in 1..=n {
            if !cfg.attached[k] || cfg.heads[k] != s {
                continue;
            }
            let take = if left { k < s } else { k > s };
            if !take {
                continue;
            }
            let better = |k: usize, b: Option<usize>| -> bool {
                match b {
                    None => true,
                    Some(x) if left => k > x,
                    Some(x) => k < x,
                }
            };
            if better(k, best) {
                second = best;
                best = Some(k);
            } else if better(k, second) {
                second = Some(k);
            }
        }
        (best, second)
    };
    let outer_tag =
        |s: usize, left: bool| -> &str { outer2(s, left).0.map(tag_at).unwrap_or("<NULL>") };
    // Valency bucket of a stack token on one side (attached
    // children count): Shift/Reduce timing signal.
    let valence = |s: usize, left: bool| -> &str {
        let mut c = 0usize;
        for k in 1..=n {
            if !cfg.attached[k] || cfg.heads[k] != s {
                continue;
            }
            if (left && k < s) || (!left && k > s) {
                c += 1;
            }
        }
        match c {
            0 => "0",
            1 => "1",
            _ => "2+",
        }
    };
    let s0 = cfg.stack.last().copied().unwrap_or(0);
    let s1 = if cfg.stack.len() >= 2 {
        cfg.stack[cfg.stack.len() - 2]
    } else {
        0
    };
    let s1_some = cfg.stack.len() >= 2;
    let s2_some = cfg.stack.len() >= 3;
    let b0 = cfg.buf;
    feats.push(hash_feature(0x30, &[word_at(s0)]));
    feats.push(hash_feature(0x31, &[tag_at(s0)]));
    feats.push(hash_feature(
        0x32,
        &[if s1_some { word_at(s1) } else { "<NULL>" }],
    ));
    feats.push(hash_feature(
        0x33,
        &[if s1_some { tag_at(s1) } else { "<NULL>" }],
    ));
    feats.push(hash_feature(
        0x34,
        &[if s2_some {
            tag_at(cfg.stack[cfg.stack.len() - 3])
        } else {
            "<NULL>"
        }],
    ));
    feats.push(hash_feature(0x35, &[word_at(b0)]));
    feats.push(hash_feature(0x36, &[tag_at(b0)]));
    feats.push(hash_feature(0x37, &[tag_at(b0 + 1)]));
    feats.push(hash_feature(0x38, &[outer_tag(s0, true)]));
    feats.push(hash_feature(0x39, &[outer_tag(s0, false)]));
    if s1_some {
        feats.push(hash_feature(0x3a, &[outer_tag(s1, true)]));
        feats.push(hash_feature(0x3b, &[outer_tag(s1, false)]));
    }
    feats.push(hash_feature(0x3c, &[tag_at(s0), tag_at(b0)]));
    let d = b0.saturating_sub(s0);
    let bucket = match d {
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6..=10 => "6-10",
        _ => "11+",
    };
    feats.push(hash_feature(0x3d, &[bucket]));
    // v2: lookahead words, dependent words, conjunctions, valency
    // (MaltParser-class density — the v1 singles underfit at 63%
    // train UAS; sparse-conjunction caution from the tagger does
    // not transfer without measurement, features gate admission).
    feats.push(hash_feature(0x3e, &[word_at(b0 + 1)]));
    feats.push(hash_feature(0x3f, &[word_at(b0 + 2)]));
    feats.push(hash_feature(0x40, &[tag_at(b0 + 2)]));
    if let Some(k) = outer2(s0, true).0 {
        feats.push(hash_feature(0x41, &[word_at(k)]));
    }
    if let Some(k) = outer2(s0, false).0 {
        feats.push(hash_feature(0x42, &[word_at(k)]));
    }
    if let Some(k) = outer2(b0, true).0 {
        feats.push(hash_feature(0x43, &[tag_at(k)]));
        feats.push(hash_feature(0x44, &[word_at(k)]));
    }
    feats.push(hash_feature(0x45, &[word_at(s0), tag_at(b0)]));
    feats.push(hash_feature(0x46, &[word_at(b0), tag_at(s0)]));
    feats.push(hash_feature(
        0x47,
        &[if s1_some { tag_at(s1) } else { "<NULL>" }, tag_at(s0)],
    ));
    feats.push(hash_feature(0x49, &[valence(s0, true)]));
    feats.push(hash_feature(0x4a, &[valence(s0, false)]));
    feats.push(hash_feature(0x4b, &[valence(b0, true)]));
    if s2_some {
        feats.push(hash_feature(
            0x4c,
            &[word_at(cfg.stack[cfg.stack.len() - 3])],
        ));
    }
    // v3: head, sibling, word bigram (the MaltParser remainder —
    // what governs the governor, the inner dependent, and lexical
    // affinity across the prospective arc).
    if cfg.attached[s0] {
        let h = cfg.heads[s0];
        feats.push(hash_feature(0x4d, &[word_at(h)]));
        feats.push(hash_feature(0x4e, &[tag_at(h)]));
    }
    if let Some(k) = outer2(s0, true).1 {
        feats.push(hash_feature(0x4f, &[tag_at(k)]));
    }
    feats.push(hash_feature(0x50, &[word_at(s0), word_at(b0)]));
    // v4 (MaltOptimizer step 4): sentence neighbors of the salient
    // pair. s0±1 disambiguate the stack top's local context
    // (e.g. determiner-before-noun vs bare noun); b0−1 sees what
    // the buffer front follows. b0+1 already rides the lookahead
    // (0x37/0x3e), so only three positions are added.
    feats.push(hash_feature(0x53, &[word_at(s0.wrapping_sub(1))]));
    feats.push(hash_feature(0x54, &[tag_at(s0.wrapping_sub(1))]));
    feats.push(hash_feature(0x55, &[word_at(s0 + 1)]));
    feats.push(hash_feature(0x56, &[tag_at(s0 + 1)]));
    feats.push(hash_feature(0x57, &[word_at(b0.wrapping_sub(1))]));
    feats.push(hash_feature(0x58, &[tag_at(b0.wrapping_sub(1))]));
}

fn action_index(a: Action) -> usize {
    match a {
        Action::Shift => 0,
        Action::Left => 1,
        Action::Right => 2,
        Action::Reduce => 3,
    }
}

/// Best legal action index (strict `>` keeps fixed action order as
/// tie-break, mirroring the tagger). `legal()` never returns all
/// false (deadlock fallback), so this always finds one.
fn best_legal(acc: &[f32; 4], legal: &[bool; 4]) -> usize {
    let mut best: Option<usize> = None;
    for a in 0..4 {
        if !legal[a] {
            continue;
        }
        match best {
            None => best = Some(a),
            Some(b) if acc[a] > acc[b] => best = Some(a),
            _ => {}
        }
    }
    best.expect("legal() always admits an action")
}

/// Greedy arc-eager model: `weights[feature][action]`.
#[derive(Debug, Default, Clone)]
pub struct Model {
    weights: U64Map<[f32; 4]>,
}

impl Model {
    /// Greedy decode of `(words, tags)` into head indices
    /// (`heads[i]` = head of 1-based token `i`; `heads[0]` and any
    /// token stranded by the deadlock fallback stay `usize::MAX`).
    /// Illegal actions are masked (`f32::NEG_INFINITY`) so the
    /// machine can never deadlock, whatever the weights say.
    pub fn parse<S: AsRef<str>, T: AsRef<str>>(&self, words: &[S], tags: &[T]) -> Vec<usize> {
        self.decode(words, tags).0
    }

    /// [`Model::parse`] paired with per-token margins (best minus
    /// runner-up *legal* action score at the decision that attached
    /// the token — Shift/Reduce steps carry the running margin).
    /// Feeds the same gated-correction discipline as tag margins.
    pub fn parse_margins<S: AsRef<str>, T: AsRef<str>>(
        &self,
        words: &[S],
        tags: &[T],
    ) -> (Vec<usize>, Vec<f32>) {
        self.decode(words, tags)
    }

    fn decode<S: AsRef<str>, T: AsRef<str>>(
        &self,
        words: &[S],
        tags: &[T],
    ) -> (Vec<usize>, Vec<f32>) {
        let n = words.len();
        let w: Vec<String> = words.iter().map(|x| x.as_ref().to_lowercase()).collect();
        let t: Vec<String> = tags.iter().map(|x| x.as_ref().to_string()).collect();
        let mut cfg = Config::initial(n);
        let mut feats = Vec::with_capacity(32);
        let mut margins = vec![0.0f32; n + 1];
        let mut guard = 0usize;
        while !cfg.is_terminal(n) {
            guard += 1;
            debug_assert!(guard < 8 * (n + 1), "decoder looped without progress");
            if guard >= 8 * (n + 1) {
                break;
            }
            let legal = cfg.legal(n);
            let acc = self.action_scores(&w, &t, &cfg, &mut feats);
            let best = best_legal(&acc, &legal);
            Self::commit_margin(&acc, &legal, best, &mut margins, &cfg);
            cfg.apply(Action::ALL[best]);
        }
        (cfg.heads, margins)
    }

    /// Record the best-minus-runner-up legal gap at the token this
    /// decision attaches (Left→stack top, Right→buffer front);
    /// Shift/Reduce attach nothing and leave margins untouched (0.0
    /// carries no signal — same gate semantics as tag margins).
    fn commit_margin(
        acc: &[f32; 4],
        legal: &[bool; 4],
        best: usize,
        margins: &mut [f32],
        cfg: &Config,
    ) {
        let mut second = f32::NEG_INFINITY;
        for a in 0..4 {
            if legal[a] && a != best && acc[a] > second {
                second = acc[a];
            }
        }
        if second == f32::NEG_INFINITY {
            return;
        }
        let m = acc[best] - second;
        match Action::ALL[best] {
            Action::Left => {
                if let Some(&s0) = cfg.stack.last() {
                    margins[s0] = m;
                }
            }
            Action::Right if cfg.buf < margins.len() => {
                margins[cfg.buf] = m;
            }
            _ => {}
        }
    }
    /// Score the four actions in this configuration (shared by
    /// greedy and beam decoding — one scoring path, bit-identical).
    fn action_scores(
        &self,
        lower: &[String],
        tags: &[String],
        cfg: &Config,
        feats: &mut Vec<u64>,
    ) -> [f32; 4] {
        features(lower, tags, cfg, feats);
        let mut acc = [0.0f32; 4];
        for f in feats.iter() {
            if let Some(arr) = self.weights.get(f) {
                for (a, wgt) in acc.iter_mut().zip(arr.iter()) {
                    *a += *wgt;
                }
            }
        }
        acc
    }

    /// Width-`width` beam search over transition sequences. Each
    /// hypothesis carries its own config, cumulative score, and
    /// per-token margins (recorded at the attaching decision under
    /// that hypothesis's history — the tagger-beam analogue; 0.0
    /// where nothing attached). Returns the winning heads plus
    /// margins. Greedy is width 1 without the margin refinement;
    /// use this for measurement sweeps, not the keystroke path
    /// (cost is ~width×4 scorings per step).
    pub fn parse_beam<S: AsRef<str>, T: AsRef<str>>(
        &self,
        words: &[S],
        tags: &[T],
        width: usize,
    ) -> (Vec<usize>, Vec<f32>) {
        let n = words.len();
        let w: Vec<String> = words.iter().map(|x| x.as_ref().to_lowercase()).collect();
        let t: Vec<String> = tags.iter().map(|x| x.as_ref().to_string()).collect();
        let width = width.max(1);
        struct Hyp {
            cfg: Config,
            score: f32,
            margins: Vec<f32>,
        }
        let mut hyps = vec![Hyp {
            cfg: Config::initial(n),
            score: 0.0,
            margins: vec![0.0f32; n + 1],
        }];
        let mut feats = Vec::with_capacity(32);
        let mut guard = 0usize;
        while hyps.iter().any(|h| !h.cfg.is_terminal(n)) {
            guard += 1;
            if guard >= 4 * (n + 1).max(1) {
                break;
            }
            let mut cands: Vec<Hyp> = Vec::with_capacity(hyps.len() * 4);
            for h in &hyps {
                if h.cfg.is_terminal(n) {
                    cands.push(Hyp {
                        cfg: h.cfg.clone(),
                        score: h.score,
                        margins: h.margins.clone(),
                    });
                    continue;
                }
                let legal = h.cfg.legal(n);
                let acc = self.action_scores(&w, &t, &h.cfg, &mut feats);
                for a in 0..4 {
                    if !legal[a] {
                        continue;
                    }
                    let mut cfg = h.cfg.clone();
                    let mut margins = h.margins.clone();
                    // Local best-gap under this history, recorded at
                    // the token the action attaches (if any).
                    let mut second = f32::NEG_INFINITY;
                    for b in 0..4 {
                        if legal[b] && b != a && acc[b] > second {
                            second = acc[b];
                        }
                    }
                    let s0 = cfg.stack.last().copied().unwrap_or(0);
                    let b0 = cfg.buf;
                    if second != f32::NEG_INFINITY {
                        match Action::ALL[a] {
                            Action::Left => margins[s0] = acc[a] - second,
                            Action::Right => margins[b0] = acc[a] - second,
                            _ => {}
                        }
                    }
                    cfg.apply(Action::ALL[a]);
                    cands.push(Hyp {
                        cfg,
                        score: h.score + acc[a],
                        margins,
                    });
                }
            }
            cands.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            cands.truncate(width);
            hyps = cands;
        }
        let mut best = 0usize;
        for (k, h) in hyps.iter().enumerate() {
            // Prefer terminal hypotheses; among them (or if none),
            // highest score wins.
            let ka = (h.cfg.is_terminal(n) as u8, h.score);
            let kb = (hyps[best].cfg.is_terminal(n) as u8, hyps[best].score);
            if ka > kb {
                best = k;
            }
        }
        let winner = &hyps[best];
        (winner.cfg.heads.clone(), winner.margins.clone())
    }

    /// Train on gold `(words, tags, heads)` with the plain
    /// perceptron for `iters` passes (`min_count` drops rare
    /// features). Updates fire on oracle disagreements only;
    /// illegal actions are masked in both training and inference,
    /// so train/decode see the same action space.
    pub fn train(
        data: &[(Vec<String>, Vec<String>, Vec<usize>)],
        iters: usize,
        min_count: usize,
    ) -> Self {
        let mut counts: U64Map<usize> = U64Map::default();
        let mut feats = Vec::with_capacity(20);
        for (raw, tags, gold) in data {
            let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
            let mut cfg = Config::initial(raw.len());
            let mut guard = 0usize;
            while !cfg.is_terminal(raw.len()) {
                guard += 1;
                if guard >= 8 * (raw.len() + 1) {
                    break;
                }
                features(&lower, tags, &cfg, &mut feats);
                for f in &feats {
                    *counts.entry(*f).or_insert(0) += 1;
                }
                cfg.apply(oracle(gold, &cfg));
            }
        }
        let mut weights: U64Map<[f32; 4]> = U64Map::default();
        for _ in 0..iters {
            for (raw, tags, gold) in data {
                let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
                let mut cfg = Config::initial(raw.len());
                let mut guard = 0usize;
                while !cfg.is_terminal(raw.len()) {
                    guard += 1;
                    if guard >= 8 * (raw.len() + 1) {
                        break;
                    }
                    features(&lower, tags, &cfg, &mut feats);
                    let legal = cfg.legal(raw.len());
                    let mut acc = [0.0f32; 4];
                    for f in feats
                        .iter()
                        .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                    {
                        if let Some(arr) = weights.get(f) {
                            for (a, wgt) in acc.iter_mut().zip(arr.iter()) {
                                *a += *wgt;
                            }
                        }
                    }
                    let best = best_legal(&acc, &legal);
                    let want = oracle(gold, &cfg);
                    if best != action_index(want) {
                        for f in feats
                            .iter()
                            .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                        {
                            let arr = weights.entry(*f).or_insert([0.0; 4]);
                            arr[action_index(want)] += 1.0;
                            arr[best] -= 1.0;
                        }
                    }
                    cfg.apply(oracle(gold, &cfg));
                }
            }
        }
        weights.retain(|f, arr| {
            arr.iter().any(|w| *w != 0.0) && counts.get(f).is_some_and(|&c| c >= min_count)
        });
        Model { weights }
    }

    /// Train with Collins averaging: same perceptron updates as
    /// [`Model::train`], but decode with weights averaged over
    /// survival time (timestamped totals, sentence-granular steps).
    /// Averaging was falsified for the *tagger* on fast-converging
    /// data (33% vs 88% dev) — but the parser shows the opposite
    /// signature (train 82% vs dev 71%: oscillation on sparse
    /// shared features), so it is measured here, not assumed
    /// either way. Timestamp maps live in the trainer only and
    /// never ship.
    pub fn train_averaged(
        data: &[(Vec<String>, Vec<String>, Vec<usize>)],
        iters: usize,
        min_count: usize,
    ) -> Self {
        let mut counts: U64Map<usize> = U64Map::default();
        let mut feats = Vec::with_capacity(32);
        for (raw, tags, gold) in data {
            let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
            let mut cfg = Config::initial(raw.len());
            let mut guard = 0usize;
            while !cfg.is_terminal(raw.len()) {
                guard += 1;
                if guard >= 8 * (raw.len() + 1) {
                    break;
                }
                features(&lower, tags, &cfg, &mut feats);
                for f in &feats {
                    *counts.entry(*f).or_insert(0) += 1;
                }
                cfg.apply(oracle(gold, &cfg));
            }
        }
        let mut weights: U64Map<[f32; 4]> = U64Map::default();
        let mut totals: U64Map<[f64; 4]> = U64Map::default();
        let mut stamp: U64Map<usize> = U64Map::default();
        let mut step = 0usize;
        for _ in 0..iters {
            for (raw, tags, gold) in data {
                step += 1;
                let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
                let mut cfg = Config::initial(raw.len());
                let mut guard = 0usize;
                while !cfg.is_terminal(raw.len()) {
                    guard += 1;
                    if guard >= 8 * (raw.len() + 1) {
                        break;
                    }
                    features(&lower, tags, &cfg, &mut feats);
                    let legal = cfg.legal(raw.len());
                    let mut acc = [0.0f32; 4];
                    for f in feats
                        .iter()
                        .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                    {
                        if let Some(arr) = weights.get(f) {
                            for (a, wgt) in acc.iter_mut().zip(arr.iter()) {
                                *a += *wgt;
                            }
                        }
                    }
                    let best = best_legal(&acc, &legal);
                    let want = oracle(gold, &cfg);
                    if best != action_index(want) {
                        for f in feats
                            .iter()
                            .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                        {
                            let last = stamp.insert(*f, step).unwrap_or(0);
                            let tot = totals.entry(*f).or_insert([0.0; 4]);
                            if let Some(cur) = weights.get(f) {
                                for (t, c) in tot.iter_mut().zip(cur.iter()) {
                                    *t += *c as f64 * (step - last) as f64;
                                }
                            }
                            let arr = weights.entry(*f).or_insert([0.0; 4]);
                            arr[action_index(want)] += 1.0;
                            arr[best] -= 1.0;
                        }
                    }
                    cfg.apply(oracle(gold, &cfg));
                }
            }
        }
        let total_steps = step.max(1) as f64;
        let mut averaged: U64Map<[f32; 4]> = U64Map::default();
        for (f, arr) in &weights {
            if !arr.iter().any(|w| *w != 0.0) {
                continue;
            }
            if !counts.get(f).is_some_and(|&c| c >= min_count) {
                continue;
            }
            let last = stamp.get(f).copied().unwrap_or(0);
            let mut acc = totals.get(f).copied().unwrap_or([0.0; 4]);
            for (t, c) in acc.iter_mut().zip(arr.iter()) {
                *t += *c as f64 * (total_steps - last as f64);
            }
            let mut out = [0.0f32; 4];
            for (o, t) in out.iter_mut().zip(acc.iter()) {
                *o = (*t / total_steps) as f32;
            }
            if out.iter().any(|w| *w != 0.0) {
                averaged.insert(*f, out);
            }
        }
        Model { weights: averaged }
    }

    /// Train under the beam with early update (Collins & Roark
    /// 2004): the beam runs in lockstep with the oracle path, and
    /// whenever the gold configuration falls out of the beam the
    /// weights move +gold-prefix / −best-hyp-prefix and search
    /// restarts from gold. A final update fires if the winning
    /// hypothesis still differs from gold. This is the license for
    /// beam *decoding*: greedily-trained scores rank actions within
    /// a state but compare meaninglessly across paths (measured:
    /// beam-2 decode of greedy-trained weights lost 16 points), so
    /// a model meant to be decoded under a beam must be trained
    /// under one. Averaging included (same timestamp scheme as
    /// [`Model::train_averaged`], sentence-granular steps);
    /// weights shape unchanged, so artifacts stay interchangeable.
    pub fn train_beam(
        data: &[(Vec<String>, Vec<String>, Vec<usize>)],
        iters: usize,
        min_count: usize,
        width: usize,
    ) -> Self {
        let width = width.max(1);
        let mut counts: U64Map<usize> = U64Map::default();
        let mut feats = Vec::with_capacity(32);
        for (raw, tags, gold) in data {
            let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
            let mut cfg = Config::initial(raw.len());
            let mut guard = 0usize;
            while !cfg.is_terminal(raw.len()) {
                guard += 1;
                if guard >= 8 * (raw.len() + 1) {
                    break;
                }
                features(&lower, tags, &cfg, &mut feats);
                for f in &feats {
                    *counts.entry(*f).or_insert(0) += 1;
                }
                cfg.apply(oracle(gold, &cfg));
            }
        }
        // Averaged-update bookkeeping: touch accrues survival time
        // before each mutation (sentence-granular steps, mirroring
        // `train_averaged`).
        fn touch(
            weights: &mut U64Map<[f32; 4]>,
            totals: &mut U64Map<[f64; 4]>,
            stamp: &mut U64Map<usize>,
            step: usize,
            f: u64,
        ) {
            let last = stamp.insert(f, step).unwrap_or(0);
            let tot = totals.entry(f).or_insert([0.0; 4]);
            if let Some(cur) = weights.get(&f) {
                for (t, c) in tot.iter_mut().zip(cur.iter()) {
                    *t += *c as f64 * (step - last) as f64;
                }
            }
        }
        fn score(
            weights: &U64Map<[f32; 4]>,
            counts: &U64Map<usize>,
            min_count: usize,
            kept: &[u64],
        ) -> [f32; 4] {
            let mut acc = [0.0f32; 4];
            for f in kept
                .iter()
                .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
            {
                if let Some(arr) = weights.get(f) {
                    for (a, wgt) in acc.iter_mut().zip(arr.iter()) {
                        *a += *wgt;
                    }
                }
            }
            acc
        }
        struct BeamHyp {
            cfg: Config,
            score: f32,
            hist: Vec<(u64, usize)>,
        }
        let mut weights: U64Map<[f32; 4]> = U64Map::default();
        let mut totals: U64Map<[f64; 4]> = U64Map::default();
        let mut stamp: U64Map<usize> = U64Map::default();
        let mut step = 0usize;
        for _ in 0..iters {
            for (raw, tags, gold) in data {
                step += 1;
                let n = raw.len();
                let lower: Vec<String> = raw.iter().map(|w| w.to_lowercase()).collect();
                let mut gold_cfg = Config::initial(n);
                let mut gold_hist: Vec<(u64, usize)> = Vec::new();
                let mut beam = vec![BeamHyp {
                    cfg: Config::initial(n),
                    score: 0.0,
                    hist: Vec::new(),
                }];
                let mut guard = 0usize;
                while !gold_cfg.is_terminal(n) {
                    guard += 1;
                    if guard >= 8 * (n + 1) {
                        break;
                    }
                    // Gold step under the oracle; full-prefix history
                    // accrues (the + side of any early update).
                    let want = oracle(gold, &gold_cfg);
                    features(&lower, tags, &gold_cfg, &mut feats);
                    let kept: Vec<u64> = feats
                        .iter()
                        .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                        .copied()
                        .collect();
                    for f in &kept {
                        gold_hist.push((*f, action_index(want)));
                    }
                    // Beam advances one step from every hypothesis.
                    let mut cands: Vec<BeamHyp> = Vec::with_capacity(beam.len() * 4);
                    for h in &beam {
                        if h.cfg.is_terminal(n) {
                            cands.push(BeamHyp {
                                cfg: h.cfg.clone(),
                                score: h.score,
                                hist: h.hist.clone(),
                            });
                            continue;
                        }
                        let legal = h.cfg.legal(n);
                        features(&lower, tags, &h.cfg, &mut feats);
                        let hkept: Vec<u64> = feats
                            .iter()
                            .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                            .copied()
                            .collect();
                        let acc = score(&weights, &counts, min_count, &hkept);
                        for a in 0..4 {
                            if !legal[a] {
                                continue;
                            }
                            let mut cfg = h.cfg.clone();
                            cfg.apply(Action::ALL[a]);
                            let mut hist = h.hist.clone();
                            for f in &hkept {
                                hist.push((*f, a));
                            }
                            cands.push(BeamHyp {
                                cfg,
                                score: h.score + acc[a],
                                hist,
                            });
                        }
                    }
                    cands.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
                    cands.truncate(width);
                    beam = cands;
                    gold_cfg.apply(want);
                    if beam.iter().any(|h| h.cfg == gold_cfg) {
                        continue;
                    }
                    // Gold fell out: +gold prefix, −best hyp prefix,
                    // restart search from gold.
                    let best = beam
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap())
                        .map(|(k, _)| k)
                        .unwrap_or(0);
                    let (plus, minus) = (gold_hist.clone(), beam[best].hist.clone());
                    for (f, a) in &plus {
                        touch(&mut weights, &mut totals, &mut stamp, step, *f);
                        weights.entry(*f).or_insert([0.0; 4])[*a] += 1.0;
                    }
                    for (f, a) in &minus {
                        touch(&mut weights, &mut totals, &mut stamp, step, *f);
                        weights.entry(*f).or_insert([0.0; 4])[*a] -= 1.0;
                    }
                    beam = vec![BeamHyp {
                        cfg: gold_cfg.clone(),
                        score: 0.0,
                        hist: gold_hist.clone(),
                    }];
                }
                // Final update if the winner still differs from gold.
                let mut best = 0usize;
                for (k, h) in beam.iter().enumerate() {
                    let ka = (h.cfg.is_terminal(n) as u8, h.score);
                    let kb = (beam[best].cfg.is_terminal(n) as u8, beam[best].score);
                    if ka > kb {
                        best = k;
                    }
                }
                let bad = beam.is_empty()
                    || beam[best].cfg.heads.len() != gold.len()
                    || beam[best]
                        .cfg
                        .heads
                        .iter()
                        .zip(gold.iter())
                        .any(|(a, b)| a != b);
                if bad && !beam.is_empty() {
                    let (plus, minus) = (gold_hist.clone(), beam[best].hist.clone());
                    for (f, a) in &plus {
                        touch(&mut weights, &mut totals, &mut stamp, step, *f);
                        weights.entry(*f).or_insert([0.0; 4])[*a] += 1.0;
                    }
                    for (f, a) in &minus {
                        touch(&mut weights, &mut totals, &mut stamp, step, *f);
                        weights.entry(*f).or_insert([0.0; 4])[*a] -= 1.0;
                    }
                }
            }
        }
        let total_steps = step.max(1) as f64;
        let mut averaged: U64Map<[f32; 4]> = U64Map::default();
        for (f, arr) in &weights {
            if !arr.iter().any(|w| *w != 0.0) {
                continue;
            }
            if !counts.get(f).is_some_and(|&c| c >= min_count) {
                continue;
            }
            let last = stamp.get(f).copied().unwrap_or(0);
            let mut acc = totals.get(f).copied().unwrap_or([0.0; 4]);
            for (t, c) in acc.iter_mut().zip(arr.iter()) {
                *t += *c as f64 * (total_steps - last as f64);
            }
            let mut out = [0.0f32; 4];
            for (o, t) in out.iter_mut().zip(acc.iter()) {
                *o = (*t / total_steps) as f32;
            }
            if out.iter().any(|w| *w != 0.0) {
                averaged.insert(*f, out);
            }
        }
        Model { weights: averaged }
    }

    /// Deserialize weights written by the trainer. Unknown action
    /// codes are an error (fail loudly on a corrupt artifact).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let raw: std::collections::BTreeMap<u64, HashMap<String, f32>> =
            serde_json::from_str::<serde_json::Value>(json)
                .and_then(|v| serde_json::from_value(v["weights"].clone()))?;
        let mut weights: U64Map<[f32; 4]> = U64Map::default();
        for (f, per_action) in raw {
            let mut arr = [0.0f32; 4];
            for (code, w) in &per_action {
                match Action::from_code(code) {
                    Some(a) => arr[action_index(a)] = *w,
                    None => {
                        return Err(serde::de::Error::custom(format!(
                            "unknown action code {code:?}"
                        )));
                    }
                }
            }
            weights.insert(f, arr);
        }
        Ok(Model { weights })
    }

    /// Serialize weights for committing: compact JSON with sorted
    /// keys; whole-number weights serialize as integers; exact-zero
    /// rows omitted — same artifact discipline as `english-pos`.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let sorted: std::collections::BTreeMap<
            u64,
            std::collections::BTreeMap<String, serde_json::Value>,
        > = self
            .weights
            .iter()
            .map(|(f, arr)| {
                (
                    *f,
                    Action::ALL
                        .iter()
                        .enumerate()
                        .filter(|(t, _)| arr[*t] != 0.0)
                        .map(|(t, code)| {
                            let w = arr[t];
                            let v =
                                if w.fract() == 0.0 && w >= i32::MIN as f32 && w <= i32::MAX as f32
                                {
                                    serde_json::Value::from(w as i32)
                                } else {
                                    serde_json::Number::from_f64(w as f64)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null)
                                };
                            (code.code().to_string(), v)
                        })
                        .collect(),
                )
            })
            .collect();
        serde_json::to_string(&serde_json::json!({"weights": sorted}))
    }
}

/// Arc-label features for dependent `d` (1-based) with head `h`.
/// Words pre-lowered, tags UPOS; `heads` follows the crate
/// convention (index 0 dummy, `usize::MAX` = stranded). Children
/// come from the GIVEN tree (gold in training, decoded in
/// inference) — the classifier never moves heads, so no
/// oracle/projection machinery applies. Discriminants `0x60`
/// d word, `0x61` d tag, `0x62` h word, `0x63` h tag, `0x64`/`0x65`
/// d∓1 tags, `0x66`/`0x67` h∓1 tags, `0x68`/`0x69` d outer-dep
/// tags, `0x6a`/`0x6b` h outer-dep tags, `0x6c` d+h tag bigram,
/// `0x6d` d-word+h-tag, `0x6e` d-tag+h-word, `0x6f` direction,
/// `0x70` distance bucket, `0x71`/`0x72` d∓1 words.
pub fn label_features(
    words: &[String],
    tags: &[String],
    heads: &[usize],
    d: usize,
    feats: &mut Vec<u64>,
) {
    feats.clear();
    let n = words.len();
    let h = if d < heads.len() {
        heads[d]
    } else {
        usize::MAX
    };
    let word_at = |i: usize| -> &str {
        if i >= 1 && i <= n {
            words[i - 1].as_str()
        } else if i == 0 {
            "<ROOT>"
        } else {
            "<NULL>"
        }
    };
    let tag_at = |i: usize| -> &str {
        if i >= 1 && i <= n {
            tags[i - 1].as_str()
        } else if i == 0 {
            "<ROOT>"
        } else {
            "<NULL>"
        }
    };
    // Outermost dependent tag of token s on one side (NULL when
    // childless there); linear scan, train loop pays it most.
    let outer = |s: usize, left: bool| -> &str {
        let mut best: Option<usize> = None;
        for k in 1..=n {
            if k >= heads.len() || heads[k] != s {
                continue;
            }
            let take = if left { k < s } else { k > s };
            if !take {
                continue;
            }
            best = Some(match best {
                None => k,
                Some(b) if left => b.max(k),
                Some(b) => b.min(k),
            });
        }
        best.map(tag_at).unwrap_or("<NULL>")
    };
    feats.push(hash_feature(0x60, &[word_at(d)]));
    feats.push(hash_feature(0x61, &[tag_at(d)]));
    feats.push(hash_feature(0x62, &[word_at(h)]));
    feats.push(hash_feature(0x63, &[tag_at(h)]));
    feats.push(hash_feature(0x64, &[tag_at(d.wrapping_sub(1))]));
    feats.push(hash_feature(0x65, &[tag_at(d + 1)]));
    feats.push(hash_feature(0x66, &[tag_at(h.wrapping_sub(1))]));
    feats.push(hash_feature(0x67, &[tag_at(h + 1)]));
    feats.push(hash_feature(0x68, &[outer(d, true)]));
    feats.push(hash_feature(0x69, &[outer(d, false)]));
    feats.push(hash_feature(0x6a, &[outer(h, true)]));
    feats.push(hash_feature(0x6b, &[outer(h, false)]));
    feats.push(hash_feature(0x6c, &[tag_at(d), tag_at(h)]));
    feats.push(hash_feature(0x6d, &[word_at(d), tag_at(h)]));
    feats.push(hash_feature(0x6e, &[tag_at(d), word_at(h)]));
    feats.push(hash_feature(0x6f, &[if h < d { "LEFT" } else { "RIGHT" }]));
    let dist = d.abs_diff(h);
    let bucket = match dist {
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6..=10 => "6-10",
        _ => "11+",
    };
    feats.push(hash_feature(0x70, &[bucket]));
    feats.push(hash_feature(0x71, &[word_at(d.wrapping_sub(1))]));
    feats.push(hash_feature(0x72, &[word_at(d + 1)]));
}

/// Sparse labeler rows: feature hash -> per-label weights.
type LabelRows = U64Map<Vec<f32>>;

/// UD relation classifier over decoded (or gold) arcs: averaged
/// perceptron, same weight-row/JSON discipline as [`Model`]
/// (sorted label set for stable indices, integer-encoded wholes,
/// zero rows omitted). The label set is CLOSED from training —
/// prediction always lands in-set; stranded tokens (head
/// `usize::MAX`) carry `""` (never gold — LAS counts them wrong
/// via the head anyway, and the empty string falsifies silently
/// passing them through).
#[derive(Debug, Clone, Default)]
pub struct LabelModel {
    labels: Vec<String>,
    weights: LabelRows,
}

impl LabelModel {
    /// Closed label set, sorted (stable indices across runs).
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Classify every arc `(d, heads[d])`; index 0 is `""`.
    pub fn predict<S: AsRef<str>, T: AsRef<str>>(
        &self,
        words: &[S],
        tags: &[T],
        heads: &[usize],
    ) -> Vec<String> {
        self.decode(words, tags, heads).0
    }

    /// [`LabelModel::predict`] paired with per-token margins (best
    /// minus runner-up label score; `""` tokens carry 0.0).
    pub fn predict_margins<S: AsRef<str>, T: AsRef<str>>(
        &self,
        words: &[S],
        tags: &[T],
        heads: &[usize],
    ) -> (Vec<String>, Vec<f32>) {
        self.decode(words, tags, heads)
    }

    fn decode<S: AsRef<str>, T: AsRef<str>>(
        &self,
        words: &[S],
        tags: &[T],
        heads: &[usize],
    ) -> (Vec<String>, Vec<f32>) {
        let n = words.len();
        let w: Vec<String> = words.iter().map(|x| x.as_ref().to_lowercase()).collect();
        let t: Vec<String> = tags.iter().map(|x| x.as_ref().to_string()).collect();
        let mut out = vec![String::new(); n + 1];
        let mut margins = vec![0.0f32; n + 1];
        if self.labels.is_empty() {
            return (out, margins);
        }
        let mut feats = Vec::with_capacity(24);
        for d in 1..=n {
            let h = if d < heads.len() {
                heads[d]
            } else {
                usize::MAX
            };
            if h == usize::MAX || h > n {
                continue;
            }
            label_features(&w, &t, heads, d, &mut feats);
            let mut best = 0usize;
            let mut best_score = f32::NEG_INFINITY;
            let mut second = f32::NEG_INFINITY;
            for (li, _) in self.labels.iter().enumerate() {
                let mut s = 0.0f32;
                for f in feats.iter() {
                    if let Some(row) = self.weights.get(f) {
                        s += row[li];
                    }
                }
                if s > best_score {
                    second = best_score;
                    best_score = s;
                    best = li;
                } else if s > second {
                    second = s;
                }
            }
            out[d] = self.labels[best].clone();
            if second != f32::NEG_INFINITY {
                margins[d] = best_score - second;
            }
        }
        (out, margins)
    }

    /// Train on gold `(words, tags, heads, rels)` with the averaged
    /// perceptron (`iters` passes, `min_count` floor). Averaged-only
    /// by stage-1 evidence (averaging won +7.1 on arc decisions);
    /// plain is unbuilt — if labels land far short, plain is the
    /// named fallback, not an assumed alternative. Instances are
    /// gold arcs; non-projective sentences train fine (no oracle
    /// involved — classification, not search).
    pub fn train(
        data: &[(Vec<String>, Vec<String>, Vec<usize>, Vec<String>)],
        iters: usize,
        min_count: usize,
    ) -> Self {
        let mut set = std::collections::BTreeSet::new();
        for (_, _, _, rels) in data {
            for r in rels.iter().skip(1) {
                set.insert(r.clone());
            }
        }
        let labels: Vec<String> = set.into_iter().collect();
        let index = |r: &str| labels.iter().position(|l| l == r);
        let mut counts: U64Map<usize> = U64Map::default();
        let mut feats = Vec::with_capacity(24);
        for (raw, tags, heads, _) in data {
            let lower: Vec<String> = raw.iter().map(|x| x.to_lowercase()).collect();
            for d in 1..=raw.len() {
                label_features(&lower, tags, heads, d, &mut feats);
                for f in &feats {
                    *counts.entry(*f).or_insert(0) += 1;
                }
            }
        }
        let mut weights: LabelRows = LabelRows::default();
        let mut totals: U64Map<Vec<f64>> = U64Map::default();
        let mut stamp: U64Map<usize> = U64Map::default();
        let mut step = 0usize;
        for _ in 0..iters {
            for (raw, tags, heads, rels) in data {
                step += 1;
                let lower: Vec<String> = raw.iter().map(|x| x.to_lowercase()).collect();
                for (d, want_rel) in rels.iter().enumerate().skip(1) {
                    let want = match index(want_rel) {
                        Some(li) => li,
                        None => continue,
                    };
                    label_features(&lower, tags, heads, d, &mut feats);
                    let kept: Vec<u64> = feats
                        .iter()
                        .filter(|f| counts.get(f).is_some_and(|&c| c >= min_count))
                        .copied()
                        .collect();
                    let mut best = 0usize;
                    let mut best_score = f32::NEG_INFINITY;
                    for li in 0..labels.len() {
                        let mut s = 0.0f32;
                        for f in kept.iter() {
                            if let Some(row) = weights.get(f) {
                                s += row[li];
                            }
                        }
                        if s > best_score {
                            best_score = s;
                            best = li;
                        }
                    }
                    if best != want {
                        for f in &kept {
                            let last = stamp.insert(*f, step).unwrap_or(0);
                            let tot = totals.entry(*f).or_insert_with(|| vec![0.0; labels.len()]);
                            if let Some(cur) = weights.get(f) {
                                for (t, c) in tot.iter_mut().zip(cur.iter()) {
                                    *t += *c as f64 * (step - last) as f64;
                                }
                            }
                            let row = weights.entry(*f).or_insert_with(|| vec![0.0; labels.len()]);
                            row[want] += 1.0;
                            row[best] -= 1.0;
                        }
                    }
                }
            }
        }
        let total_steps = step.max(1) as f64;
        let mut averaged: LabelRows = LabelRows::default();
        for (f, row) in &weights {
            if !row.iter().any(|x| *x != 0.0) {
                continue;
            }
            if !counts.get(f).is_some_and(|&c| c >= min_count) {
                continue;
            }
            let last = stamp.get(f).copied().unwrap_or(0);
            let mut acc = totals
                .get(f)
                .cloned()
                .unwrap_or_else(|| vec![0.0; labels.len()]);
            for (t, c) in acc.iter_mut().zip(row.iter()) {
                *t += *c as f64 * (total_steps - last as f64);
            }
            let out: Vec<f32> = acc.iter().map(|t| (*t / total_steps) as f32).collect();
            if out.iter().any(|x| *x != 0.0) {
                averaged.insert(*f, out);
            }
        }
        LabelModel { labels, weights }
    }

    /// Deserialize a labeler artifact. Unknown structure is an
    /// error (fail loudly on a corrupt artifact, same as heads).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let v: serde_json::Value = serde_json::from_str(json)?;
        let labels: Vec<String> = serde_json::from_value(v["labels"].clone()).map_err(|_| {
            serde::de::Error::custom("labeler artifact missing string \"labels\" array")
        })?;
        let raw: std::collections::BTreeMap<u64, HashMap<String, f32>> =
            serde_json::from_value(v["weights"].clone())?;
        let mut weights: LabelRows = LabelRows::default();
        for (f, per_label) in raw {
            let mut row = vec![0.0f32; labels.len()];
            for (code, wgt) in &per_label {
                match labels.iter().position(|l| l == code) {
                    Some(li) => row[li] = *wgt,
                    None => {
                        return Err(serde::de::Error::custom(format!(
                            "unknown label code {code:?}"
                        )));
                    }
                }
            }
            weights.insert(f, row);
        }
        Ok(LabelModel { labels, weights })
    }

    /// Serialize the labeler: sorted label set, sorted feature
    /// keys, whole-number weights as integers, zero rows omitted —
    /// same artifact discipline as the arc model.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        type SortedRows =
            std::collections::BTreeMap<u64, std::collections::BTreeMap<String, serde_json::Value>>;
        let sorted: SortedRows = self
            .weights
            .iter()
            .map(|(f, row)| {
                (
                    *f,
                    self.labels
                        .iter()
                        .enumerate()
                        .filter(|(li, _)| row[*li] != 0.0)
                        .map(|(li, code)| {
                            let x = row[li];
                            let v =
                                if x.fract() == 0.0 && x >= i32::MIN as f32 && x <= i32::MAX as f32
                                {
                                    serde_json::Value::from(x as i32)
                                } else {
                                    serde_json::Number::from_f64(x as f64)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null)
                                };
                            (code.clone(), v)
                        })
                        .collect(),
                )
            })
            .collect();
        serde_json::to_string(&serde_json::json!({"labels": self.labels, "weights": sorted}))
    }
}
