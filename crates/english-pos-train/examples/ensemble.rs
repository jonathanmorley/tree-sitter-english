//! TEMPORARY ensemble probe (delete after use): K perceptron members
//! on deterministically shuffled EWT train (LCG seeds), weight maps
//! averaged at the JSON level into ONE map (same shape/size), then
//! greedy exact on dev/test for baseline, members, and average,
//! plus pairwise member disagreement on dev (diversity gate).
//! Writes member/average weights to /tmp ONLY (never weights/).
//! Usage: `ensemble -- <corpus-dir>`.

use english_pos::Model;
use std::collections::BTreeMap;

fn read_conllu(path: &std::path::Path) -> Vec<(Vec<String>, Vec<String>)> {
    let text = std::fs::read_to_string(path).expect("read conllu");
    let mut out = vec![];
    let mut words = vec![];
    let mut tags = vec![];
    for line in text.lines().chain([""]) {
        let line = line.trim_end();
        if line.is_empty() {
            if !words.is_empty() {
                out.push((std::mem::take(&mut words), std::mem::take(&mut tags)));
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 || cols[0].contains('-') || cols[0].contains('.') {
            continue;
        }
        words.push(cols[1].to_string());
        tags.push(cols[3].to_string());
    }
    out
}

fn lcg_shuffle<T>(v: &mut [T], mut state: u64) {
    for i in (1..v.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = ((state >> 33) as usize) % (i + 1);
        v.swap(i, j);
    }
}

fn num_to_value(w: f32) -> serde_json::Value {
    if w.fract() == 0.0 && w >= i32::MIN as f32 && w <= i32::MAX as f32 {
        serde_json::Value::from(w as i32)
    } else {
        serde_json::Number::from_f64(w as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    }
}

fn as_f64(v: &serde_json::Value) -> f64 {
    v.as_i64()
        .map(|x| x as f64)
        .unwrap_or(v.as_f64().unwrap_or(0.0))
}

/// Entrywise mean of member weight maps; non-weight top-level keys
/// (tagdict — order-independent by construction, asserted equal)
/// taken from the first member.
fn average(jsons: &[String]) -> String {
    let vals: Vec<serde_json::Value> = jsons
        .iter()
        .map(|j| serde_json::from_str(j).expect("parse"))
        .collect();
    let dict0 = vals[0]["tagdict"].clone();
    for v in &vals[1..] {
        assert_eq!(v["tagdict"], dict0, "tagdict must be order-independent");
    }
    let k = vals.len() as f64;
    let mut acc: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for v in &vals {
        let w = v["weights"].as_object().expect("weights map");
        for (fid, row) in w {
            let e = acc.entry(fid.clone()).or_default();
            for (code, num) in row.as_object().expect("row map") {
                *e.entry(code.clone()).or_insert(0.0) += as_f64(num);
            }
        }
    }
    let mut weights = serde_json::Map::new();
    for (fid, row) in &acc {
        let mut o = serde_json::Map::new();
        for (code, sum) in row {
            let m = (*sum / k) as f32;
            if m != 0.0 {
                o.insert(code.clone(), num_to_value(m));
            }
        }
        if !o.is_empty() {
            weights.insert(fid.clone(), serde_json::Value::Object(o));
        }
    }
    let mut top = serde_json::Map::new();
    top.insert("tagdict".to_string(), dict0);
    top.insert("weights".to_string(), serde_json::Value::Object(weights));
    serde_json::Value::Object(top).to_string()
}

fn score(model: &Model, data: &[(Vec<String>, Vec<String>)]) -> (usize, usize) {
    let (mut ok, mut n) = (0usize, 0usize);
    for (words, gold) in data {
        for (t, g) in model.tag(words).iter().zip(gold.iter()) {
            n += 1;
            if t.upos() == g.as_str() {
                ok += 1;
            }
        }
    }
    (ok, n)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(&args[1]);
    let seeds: Vec<u64> = if args.len() > 2 {
        args[2]
            .split(',')
            .map(|s| s.parse().expect("seed"))
            .collect()
    } else {
        vec![1, 2, 3]
    };
    let train = read_conllu(&dir.join("en_ewt-ud-train.conllu"));
    let dev = read_conllu(&dir.join("en_ewt-ud-dev.conllu"));
    let test = read_conllu(&dir.join("en_ewt-ud-test.conllu"));
    println!(
        "train sents: {}, dev: {}, test: {}",
        train.len(),
        dev.len(),
        test.len()
    );
    let base = Model::train(&train, 20, 1);
    let (ok, n) = score(&base, &dev);
    let (okt, nt) = score(&base, &test);
    println!(
        "baseline greedy dev: {ok}/{n} = {:.4}",
        ok as f64 / n as f64
    );
    println!(
        "baseline greedy test: {okt}/{nt} = {:.4}",
        okt as f64 / nt as f64
    );
    std::fs::write("/tmp/opencode/ens-base.json", base.to_json().expect("ser")).unwrap();
    let mut member_jsons = vec![];
    for seed in seeds {
        let mut order: Vec<usize> = (0..train.len()).collect();
        lcg_shuffle(&mut order, seed);
        let shuffled: Vec<(Vec<String>, Vec<String>)> =
            order.iter().map(|&i| train[i].clone()).collect();
        let m = Model::train(&shuffled, 20, 1);
        let (ok, n) = score(&m, &dev);
        let (okt, nt) = score(&m, &test);
        println!(
            "member{seed} greedy dev: {ok}/{n} = {:.4}",
            ok as f64 / n as f64
        );
        println!(
            "member{seed} greedy test: {okt}/{nt} = {:.4}",
            okt as f64 / nt as f64
        );
        let json = m.to_json().expect("ser");
        std::fs::write(format!("/tmp/opencode/ens-m{seed}.json"), &json).unwrap();
        member_jsons.push(json);
    }
    // Diversity: pairwise member disagreement on dev (tag strings).
    let mut dev_tags: Vec<Vec<Vec<String>>> = vec![];
    for j in 0..3 {
        let m = Model::from_json(&member_jsons[j]).expect("parse");
        dev_tags.push(
            dev.iter()
                .map(|(w, _)| m.tag(w).iter().map(|t| t.upos().to_string()).collect())
                .collect(),
        );
    }
    let mut dis = 0usize;
    let mut tot = 0usize;
    for a in 0..3 {
        for b in (a + 1)..3 {
            for (sa, sb) in dev_tags[a].iter().zip(dev_tags[b].iter()) {
                for (x, y) in sa.iter().zip(sb.iter()) {
                    tot += 1;
                    if x != y {
                        dis += 1;
                    }
                }
            }
        }
    }
    println!(
        "member pairwise dev disagreement: {dis}/{tot} = {:.4}",
        dis as f64 / tot as f64
    );
    let avg_json = average(&member_jsons);
    std::fs::write("/tmp/opencode/ens-avg.json", &avg_json).unwrap();
    println!("avg weights bytes: {}", avg_json.len());
    let avg = Model::from_json(&avg_json).expect("parse avg");
    let (ok, n) = score(&avg, &dev);
    let (okt, nt) = score(&avg, &test);
    println!("average greedy dev: {ok}/{n} = {:.4}", ok as f64 / n as f64);
    println!(
        "average greedy test: {okt}/{nt} = {:.4}",
        okt as f64 / nt as f64
    );
}
