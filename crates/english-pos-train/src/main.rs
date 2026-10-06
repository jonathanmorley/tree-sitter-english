//! Offline trainer for the `english-pos` perceptron.
//!
//! Reads Universal Dependencies CoNLL-U (`--corpus <dir>` holding
//! `en_ewt-ud-{train,dev,test}.conllu`, fetched separately — the data
//! stays out of the repo), trains, evaluates, and writes the weight
//! JSON baked into `english-pos`.
//!
//! Usage:
//!   cargo run -p english-pos-train -- --corpus /tmp/ud --iters 10 --min-count 2
//!   cargo run -p english-pos-train -- --corpus /tmp/ud --eval-only test
//!   cargo run -p english-pos-train -- --corpus /tmp/ud/ewt \
//!     --finetune /tmp/ud-moby/moby-train.conllu --finetune-iters 3
//!
//! `--finetune` loads the committed weights and runs a few in-domain
//! passes over gold CoNLL-U (same columns as training) instead of
//! retraining from scratch: joint training on a tiny in-domain set
//! shakes shared weights with noisy updates, while fine-tuning teaches
//! in-domain lexicals with bounded drift. Gate on the dev/test report
//! below before keeping the result.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use english_pos::{Model, RULES, Tag, apply_rules};

/// Parse CoNLL-U, skipping multiword/pre-tokenized lines (IDs with `-`
/// or `.`) and empty nodes. Returns `(surface, lowercased, tags)`.
fn parse_conllu(path: &Path) -> Vec<(Vec<String>, Vec<String>, Vec<String>)> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let mut out = Vec::new();
    let mut words = Vec::new();
    let mut lower = Vec::new();
    let mut tags = Vec::new();
    let mut flush = |words: &mut Vec<String>, lower: &mut Vec<String>, tags: &mut Vec<String>| {
        if !words.is_empty() {
            out.push((
                std::mem::take(words),
                std::mem::take(lower),
                std::mem::take(tags),
            ));
        }
    };
    for line in content.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            flush(&mut words, &mut lower, &mut tags);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 {
            continue;
        }
        if cols[0].contains('-') || cols[0].contains('.') {
            continue;
        }
        if Tag::from_upos(cols[3]).is_none() {
            continue;
        }
        words.push(cols[1].to_string());
        lower.push(cols[1].to_lowercase());
        tags.push(cols[3].to_string());
    }
    flush(&mut words, &mut lower, &mut tags);
    out
}

fn accuracy(model: &Model, data: &[(Vec<String>, Vec<String>, Vec<String>)]) -> (usize, usize) {
    let mut correct = 0;
    let mut total = 0;
    let mut pred_hist: HashMap<String, usize> = HashMap::new();
    let mut gold_hist: HashMap<String, usize> = HashMap::new();
    let mut conf: HashMap<(String, String), usize> = HashMap::new();
    for (words, _, gold) in data {
        for (guess, g) in model.tag(words).iter().zip(gold) {
            total += 1;
            *pred_hist.entry(guess.upos().to_string()).or_insert(0) += 1;
            *gold_hist.entry(g.clone()).or_insert(0) += 1;
            if guess.upos() == g {
                correct += 1;
            } else {
                *conf
                    .entry((g.clone(), guess.upos().to_string()))
                    .or_insert(0) += 1;
            }
        }
    }
    eprintln!("pred: {pred_hist:?}");
    eprintln!("gold: {gold_hist:?}");
    let mut conf: Vec<((String, String), usize)> = conf.into_iter().collect();
    conf.sort_by_key(|item| std::cmp::Reverse(item.1));
    eprintln!("top confusions (gold->pred): {conf:?}");
    (correct, total)
}

/// Accuracy with correction `RULES` applied (via `tag_margins` +
/// `apply_rules`). Returns (correct, total, fires) where fires counts
/// tokens whose tag changed. Used by `--correct` reporting; the plain
/// `accuracy` above stays the gate.
fn accuracy_corrected(
    model: &Model,
    data: &[(Vec<String>, Vec<String>, Vec<String>)],
) -> (usize, usize, usize) {
    let mut correct = 0;
    let mut total = 0;
    let mut fires = 0;
    let mut shown = 0;
    for (words, _, gold) in data {
        let mut tagged: Vec<(Tag, f32)> = model.tag_margins(words);
        let before: Vec<Tag> = tagged.iter().map(|(t, _)| *t).collect();
        let margins: Vec<f32> = tagged.iter().map(|(_, m)| *m).collect();
        let pieces: Vec<String> = words.to_vec();
        apply_rules(&pieces, &mut tagged, RULES);
        for (i, ((t, m), b)) in tagged.iter().zip(&before).enumerate() {
            if t != b && shown < 15 {
                // First-N-errors printer (TnT style): word, neighbor
                // predicted tags, rewrite, gold, margin — plus the
                // firing rule, so multi-rule deltas attribute per rule.
                let prev = if i > 0 {
                    before[i - 1].upos()
                } else {
                    "<START>"
                };
                let next = before.get(i + 1).map(|x| x.upos()).unwrap_or("<END>");
                let who = RULES
                    .iter()
                    .find(|r| {
                        margins[i] > 0.0
                            && margins[i] < r.threshold
                            && (r.test)(&pieces, &before, i).is_some()
                    })
                    .map(|r| r.name)
                    .unwrap_or("?");
                eprintln!(
                    "  fire: {} [{}/{}] {}->{} gold={} margin={m:.1} rule={who}",
                    words[i],
                    prev,
                    next,
                    b.upos(),
                    t.upos(),
                    gold[i]
                );
                shown += 1;
            }
        }
        fires += tagged
            .iter()
            .zip(&before)
            .filter(|((t, _), b)| t != *b)
            .count();
        for ((guess, _), g) in tagged.iter().zip(gold) {
            total += 1;
            if guess.upos() == g {
                correct += 1;
            }
        }
    }
    (correct, total, fires)
}

/// Report accuracy for one split, plus the corrected numbers when
/// `--correct` is set (fires counts rule rewrites).
fn report(
    model: &Model,
    data: &[(Vec<String>, Vec<String>, Vec<String>)],
    which: &str,
    correct: bool,
) {
    let (ok, total) = accuracy(model, data);
    println!(
        "{which}: {ok}/{total} = {:.2}%",
        100.0 * ok as f64 / total.max(1) as f64
    );
    if correct {
        let (cok, _, fires) = accuracy_corrected(model, data);
        println!(
            "{which}+rules: {cok}/{total} = {:.2}% ({fires} fires)",
            100.0 * cok as f64 / total.max(1) as f64
        );
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: english-pos-train --corpus <dir> [--iters N] [--min-count N] [--eval-only test|dev] [--finetune <conllu> [--finetune-iters N]] [--correct]"
    );
    std::process::exit(2);
}

fn main() {
    let mut corpus: Option<PathBuf> = None;
    let mut iters = 10usize;
    let mut min_count = 2usize;
    let mut eval_only: Option<String> = None;
    let mut finetune: Option<PathBuf> = None;
    let mut finetune_iters = 3usize;
    let mut correct = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus" => corpus = Some(PathBuf::from(args.next().unwrap_or_else(|| usage()))),
            "--iters" => {
                iters = args
                    .next()
                    .unwrap_or_else(|| usage())
                    .parse()
                    .unwrap_or_else(|_| usage())
            }
            "--min-count" => {
                min_count = args
                    .next()
                    .unwrap_or_else(|| usage())
                    .parse()
                    .unwrap_or_else(|_| usage())
            }
            "--eval-only" => {
                eval_only = Some(args.next().unwrap_or_else(|| usage()));
            }
            "--finetune" => {
                finetune = Some(PathBuf::from(args.next().unwrap_or_else(|| usage())));
            }
            "--finetune-iters" => {
                finetune_iters = args
                    .next()
                    .unwrap_or_else(|| usage())
                    .parse()
                    .unwrap_or_else(|_| usage())
            }
            "--correct" => correct = true,
            _ => usage(),
        }
    }
    let corpus = corpus.unwrap_or_else(|| usage());
    let split = |name: &str| corpus.join(format!("en_ewt-ud-{name}.conllu"));

    let weights_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("english-pos")
        .join("weights")
        .join("upos.json");

    if let Some(which) = eval_only {
        let data = parse_conllu(&split(&which));
        let json = fs::read_to_string(&weights_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", weights_path.display()));
        let model = Model::from_json(&json).expect("invalid weights JSON");
        report(&model, &data, &which, correct);
        return;
    }

    if let Some(ft) = finetune {
        let json = fs::read_to_string(&weights_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", weights_path.display()));
        let mut model = Model::from_json(&json).expect("invalid weights JSON");
        let ft_raw = parse_conllu(&ft);
        let ft_data: Vec<(Vec<String>, Vec<String>)> = ft_raw
            .iter()
            .map(|(words, _, tags)| (words.clone(), tags.clone()))
            .collect();
        println!(
            "finetune sentences: {}, tokens: {}, passes: {finetune_iters}",
            ft_data.len(),
            ft_data.iter().map(|(w, _)| w.len()).sum::<usize>()
        );
        model.finetune(&ft_data, finetune_iters);
        let json = model.to_json().expect("serialize weights");
        println!("weights: {} bytes", json.len());
        fs::write(&weights_path, &json).expect("write weights");

        for which in ["dev", "test"] {
            let data = parse_conllu(&split(which));
            report(&model, &data, which, correct);
        }
        return;
    }

    let train_raw = parse_conllu(&split("train"));
    let train: Vec<(Vec<String>, Vec<String>)> = train_raw
        .iter()
        .map(|(words, _, tags)| (words.clone(), tags.clone()))
        .collect();
    println!(
        "train sentences: {}, tokens: {}",
        train.len(),
        train.iter().map(|(w, _)| w.len()).sum::<usize>()
    );
    let model = Model::train(&train, iters, min_count);
    let json = model.to_json().expect("serialize weights");
    println!("weights: {} bytes", json.len());
    fs::create_dir_all(weights_path.parent().unwrap()).expect("mkdir weights");
    fs::write(&weights_path, &json).expect("write weights");

    for which in ["dev", "test"] {
        let data = parse_conllu(&split(which));
        report(&model, &data, which, correct);
    }
}
