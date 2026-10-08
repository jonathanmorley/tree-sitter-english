//! Committed UD regression gates over the vendored test splits
//! (CC BY-SA, see `english-pos-train/data/README.md`). Production
//! decode (beam-2 + all 14 rules), exact UPOS. Release-only: 46k
//! tokens of beam decode is minutes in debug, seconds in release —
//! debug builds skip, release gates pin.

use english_pos::{RULES, apply_rules};

fn read_conllu(path: &str) -> Vec<(Vec<String>, Vec<String>)> {
    let mut sents = Vec::new();
    let mut words = Vec::new();
    let mut tags = Vec::new();
    for line in std::fs::read_to_string(path).expect("read").lines() {
        let line = line.trim();
        if line.is_empty() {
            if !words.is_empty() {
                sents.push((std::mem::take(&mut words), std::mem::take(&mut tags)));
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 || cols[0].contains(['-', '.']) {
            continue;
        }
        words.push(cols[1].to_string());
        tags.push(cols[3].to_string());
    }
    if !words.is_empty() {
        sents.push((words, tags));
    }
    sents
}

fn production(model: &english_pos::Model, words: &[String]) -> Vec<String> {
    let (mut tagged, lower) = model.tag_beam_margins_lowered(words);
    if !RULES.is_empty() {
        apply_rules(&mut tagged, RULES, &lower);
    }
    tagged
        .into_iter()
        .map(|(t, _)| t.upos().to_string())
        .collect()
}

fn score(file: &str, floor: usize, label: &str) {
    if cfg!(debug_assertions) {
        eprintln!("{label}: skipped (debug build; release-only gate)");
        return;
    }
    let model = english_pos::Model::from_json(include_str!("../weights/upos.json")).unwrap();
    let sents = read_conllu(file);
    let (mut ok, mut total) = (0usize, 0usize);
    for (words, gold) in &sents {
        for (p, g) in production(&model, words).iter().zip(gold.iter()) {
            total += 1;
            if p == g {
                ok += 1;
            }
        }
    }
    eprintln!("{label}: {ok}/{total} = {:.4}", ok as f64 / total as f64);
    assert!(
        ok >= floor,
        "{label} production {ok}/{total} below floor {floor}; move deliberately with model changes"
    );
}

#[test]
fn ewt_test_gate() {
    // Standing: 23290/25094 (92.81%). Floor leaves headroom for
    // sampling noise on future weights; regressions trip it loudly.
    score(
        "../english-pos-train/data/en_ewt-ud-test.conllu",
        23200,
        "ewt-test",
    );
}

#[test]
fn pud_test_gate() {
    // Standing: 19393/21180 (91.56%). First committed PUD number —
    // a standing second opinion, same floor discipline.
    score(
        "../english-pos-train/data/en_pud-ud-test.conllu",
        19300,
        "pud-test",
    );
}
