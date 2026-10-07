//! Offline trainer for the `english-dep` arc-eager parser. Not published.
//!
//! Reads Universal Dependencies CoNLL-U (`--corpus <dir>` holding
//! `en_ewt-ud-{train,dev,test}.conllu`; multiword and empty nodes
//! skipped), trains the unlabeled perceptron, reports UAS on
//! dev/test with gold tags plus pipeline tags from the committed
//! `english-pos` model (the cascade number), and writes
//! `../english-dep/weights/dep.json` (committed).
//!
//! Usage:
//!   cargo run -p english-dep-train -- --corpus /tmp/ud --iters 20 --min-count 1
//!   cargo run -p english-dep-train -- --corpus /tmp/ud/ewt --eval-only test

use std::fs;
use std::path::{Path, PathBuf};

use english_dep::Model;

type Sent = (Vec<String>, Vec<String>, Vec<usize>);

/// Parse CoNLL-U sentences into `(lowered words, UPOS tags, heads)`
/// with a dummy index 0 (`heads[0]` unused; oracle never reads it
/// since buffer positions start at 1). UD head `0` (root) survives
/// as-is.
fn parse_conllu(path: &Path) -> Vec<Sent> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let mut out = Vec::new();
    let mut words = Vec::new();
    let mut tags = Vec::new();
    let mut heads = vec![0usize];
    // Sentences with any headless token (e.g. POS-only oracle batches
    // concatenated into the corpus) are skipped whole: dropping
    // single tokens would renumber every head after them.
    let mut headed = true;
    let mut flush = |words: &mut Vec<String>,
                     tags: &mut Vec<String>,
                     heads: &mut Vec<usize>,
                     headed: &mut bool| {
        if !words.is_empty() && *headed {
            out.push((
                std::mem::take(words),
                std::mem::take(tags),
                std::mem::replace(heads, vec![0usize]),
            ));
        } else {
            words.clear();
            tags.clear();
            *heads = vec![0usize];
        }
        *headed = true;
    };
    for line in content.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            flush(&mut words, &mut tags, &mut heads, &mut headed);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 8 {
            continue;
        }
        if cols[0].contains('-') || cols[0].contains('.') {
            continue;
        }
        if english_pos::Tag::from_upos(cols[3]).is_none() {
            continue;
        }
        let head: usize = match cols[6].parse() {
            Ok(h) => h,
            Err(_) => {
                headed = false;
                continue;
            }
        };
        words.push(cols[1].to_lowercase());
        tags.push(cols[3].to_string());
        heads.push(head);
    }
    flush(&mut words, &mut tags, &mut heads, &mut headed);
    out
}

/// Split parsed sentences into projective training material.
/// Non-projective sentences are skipped for *training* (the static
/// oracle is complete only on projective trees) but kept for eval:
/// greedy decode attaches anything, so UAS stays honest.
pub fn projective_only(data: Vec<Sent>) -> Vec<Sent> {
    let mut kept = Vec::new();
    let mut skipped = 0usize;
    for (w, t, h) in data {
        if is_projective(&h) {
            kept.push((w, t, h));
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 {
        eprintln!(
            "skipped {skipped} non-projective training sentences (static oracle is projective-only)"
        );
    }
    kept
}

/// True when no two arcs cross (intervals nest or disjoint).
/// The static arc-eager oracle is complete only on projective
/// trees; non-projective training sentences are skipped, never
/// approximated. (Index pairs are the natural formulation for arc
/// crossing — the lint's iterator rewrite obscures it.)
#[allow(clippy::needless_range_loop)]
fn is_projective(heads: &[usize]) -> bool {
    let n = heads.len() - 1;
    for d1 in 1..=n {
        let (a1, b1) = (heads[d1].min(d1), heads[d1].max(d1));
        for d2 in (d1 + 1)..=n {
            let (a2, b2) = (heads[d2].min(d2), heads[d2].max(d2));
            if (a1 < a2 && a2 < b1 && b1 < b2) || (a2 < a1 && a1 < b2 && b2 < b1) {
                return false;
            }
        }
    }
    true
}

fn uas(model: &Model, data: &[Sent], tagger: Option<&english_pos::Model>) -> (usize, usize) {
    let mut ok = 0;
    let mut total = 0;
    for (words, tags, gold) in data {
        let tags: Vec<String> = match tagger {
            // Pipeline reality: our tagger's tags, not gold.
            Some(m) => m.tag(words).iter().map(|t| t.upos().to_string()).collect(),
            None => tags.clone(),
        };
        for (i, h) in model.parse(words, &tags).iter().enumerate().skip(1) {
            total += 1;
            if i < gold.len() && *h == gold[i] {
                ok += 1;
            }
        }
    }
    (ok, total)
}

fn uas_beam(
    model: &Model,
    data: &[Sent],
    tagger: Option<&english_pos::Model>,
    width: usize,
) -> (usize, usize) {
    let mut ok = 0;
    let mut total = 0;
    for (words, tags, gold) in data {
        let tags: Vec<String> = match tagger {
            Some(m) => m.tag(words).iter().map(|t| t.upos().to_string()).collect(),
            None => tags.clone(),
        };
        let (heads, _) = model.parse_beam(words, &tags, width);
        for (i, h) in heads.iter().enumerate().skip(1) {
            total += 1;
            if i < gold.len() && *h == gold[i] {
                ok += 1;
            }
        }
    }
    (ok, total)
}

fn report(
    model: &Model,
    data: &[Sent],
    which: &str,
    tagger: Option<&english_pos::Model>,
    beam: Option<usize>,
) {
    let (ok, total) = uas(model, data, tagger);
    let via = if tagger.is_some() { "+tagger" } else { "+gold" };
    println!(
        "{which}{via}: {ok}/{total} = {:.2}%",
        100.0 * ok as f64 / total.max(1) as f64
    );
    if let Some(k) = beam {
        let (bok, _) = uas_beam(model, data, tagger, k);
        println!(
            "{which}{via}+beam{k}: {bok}/{total} = {:.2}%",
            100.0 * bok as f64 / total.max(1) as f64
        );
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: english-dep-train --corpus <dir> [--iters N] [--min-count N] [--averaged] [--beam K] [--beam-train K] [--eval-only test|dev]"
    );
    std::process::exit(2);
}

fn main() {
    let mut corpus: Option<PathBuf> = None;
    let mut iters = 20usize;
    let mut min_count = 1usize;
    let mut averaged = false;
    let mut eval_only: Option<String> = None;
    let mut beam: Option<usize> = None;
    let mut beam_train: Option<usize> = None;
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
            "--averaged" => averaged = true,
            "--beam" => {
                beam = Some(
                    args.next()
                        .unwrap_or_else(|| usage())
                        .parse()
                        .unwrap_or_else(|_| usage()),
                )
            }
            "--beam-train" => {
                beam_train = Some(
                    args.next()
                        .unwrap_or_else(|| usage())
                        .parse()
                        .unwrap_or_else(|_| usage()),
                )
            }
            _ => usage(),
        }
    }
    let corpus = corpus.unwrap_or_else(|| usage());
    let split = |name: &str| corpus.join(format!("en_ewt-ud-{name}.conllu"));

    let weights_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("english-dep")
        .join("weights")
        .join("dep.json");

    // The committed tagger for pipeline (+tagger) reporting.
    let tagger_json = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("english-pos")
        .join("weights")
        .join("upos.json");
    let tagger_json = fs::read_to_string(&tagger_json).expect("read tagger weights");
    let tagger = english_pos::Model::from_json(&tagger_json).expect("invalid tagger weights JSON");

    if let Some(which) = eval_only {
        let data = parse_conllu(&split(&which));
        let json = fs::read_to_string(&weights_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", weights_path.display()));
        let model = Model::from_json(&json).expect("invalid weights JSON");
        report(&model, &data, &which, None, beam);
        report(&model, &data, &which, Some(&tagger), beam);
        return;
    }

    let train = projective_only(parse_conllu(&split("train")));
    println!(
        "train sentences: {}, tokens: {}",
        train.len(),
        train.iter().map(|(w, _, _)| w.len()).sum::<usize>()
    );
    let model = if let Some(k) = beam_train {
        println!("beam training (LaSO early update, width {k}) + averaging");
        Model::train_beam(&train, iters, min_count, k)
    } else if averaged {
        println!("averaged perceptron (Collins)");
        Model::train_averaged(&train, iters, min_count)
    } else {
        Model::train(&train, iters, min_count)
    };
    let json = model.to_json().expect("serialize weights");
    println!("weights: {} bytes", json.len());
    fs::create_dir_all(weights_path.parent().unwrap()).expect("mkdir weights");
    fs::write(&weights_path, &json).expect("write weights");

    for which in ["dev", "test"] {
        let data = parse_conllu(&split(which));
        report(&model, &data, which, None, beam);
        report(&model, &data, which, Some(&tagger), beam);
    }
}
