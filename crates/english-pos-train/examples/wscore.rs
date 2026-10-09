//! Greedy exact of a weights file against gold tags (measurement
//! harness for weights comparisons; kept — re-created too often to
//! keep deleting). Usage: `wscore -- <weights.json> <words> <gold>`.

use english_pos::Model;

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
    let model =
        Model::from_json(&std::fs::read_to_string(&args[1]).expect("weights")).expect("parse");
    let sents = blocks(&args[2]);
    let golds = blocks(&args[3]);
    assert_eq!(sents.len(), golds.len());
    let (mut ok, mut n) = (0usize, 0usize);
    for (words, gold) in sents.iter().zip(golds.iter()) {
        for (t, g) in model.tag(words).iter().zip(gold.iter()) {
            n += 1;
            if t.upos() == g.as_str() {
                ok += 1;
            }
        }
    }
    println!("{ok}/{n} = {:.4}", ok as f64 / n as f64);
}
