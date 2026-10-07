//! Training smoke tests: the perceptron memorizes tiny data (machinery
//! check, not a quality claim) and stays deterministic.

use english_dep::Model;

fn tiny() -> Vec<(Vec<String>, Vec<String>, Vec<usize>)> {
    // "Time flies": 1->2 (nsubj), 2->0 (root).
    vec![(
        vec!["time".to_string(), "flies".to_string()],
        vec!["NOUN".to_string(), "VERB".to_string()],
        vec![0, 2, 0],
    )]
}

fn uas(model: &Model, data: &[(Vec<String>, Vec<String>, Vec<usize>)]) -> (usize, usize) {
    let mut ok = 0;
    let mut total = 0;
    for (w, t, g) in data {
        for (i, h) in model.parse(w, t).iter().enumerate().skip(1) {
            total += 1;
            if *h == g[i] {
                ok += 1;
            }
        }
    }
    (ok, total)
}

#[test]
fn train_memorizes_tiny_tree() {
    let data = tiny();
    let model = Model::train(&data, 20, 1);
    let (ok, total) = uas(&model, &data);
    assert_eq!((ok, total), (2, 2));
}

#[test]
fn train_is_deterministic() {
    let data = tiny();
    let a = Model::train(&data, 20, 1).to_json().unwrap();
    let b = Model::train(&data, 20, 1).to_json().unwrap();
    assert_eq!(a, b);
}

#[test]
fn json_roundtrip() {
    let data = tiny();
    let json = Model::train(&data, 20, 1).to_json().unwrap();
    let back = Model::from_json(&json).unwrap();
    let (ok, total) = uas(&back, &data);
    assert_eq!((ok, total), (2, 2));
    assert!(Model::from_json(r#"{"weights": {"1": {"BOGUS": 2}}}"#).is_err());
}

#[test]
fn train_beam_memorizes_tiny_tree() {
    // LaSO training memorizes trivial data (under beam decode —
    // the path it optimizes; greedy decode of beam-trained weights
    // carries no such guarantee, measured 0/2 on this toy while
    // beam reads 2/2) and stays deterministic.
    let data = tiny();
    let model = Model::train_beam(&data, 20, 1, 2);
    let (w, t, g) = &data[0];
    let (bheads, _) = model.parse_beam(w, t, 2);
    assert_eq!(&bheads[1..], &g[1..]);
    let again = Model::train_beam(&data, 20, 1, 2).to_json().unwrap();
    assert_eq!(model.to_json().unwrap(), again);
}
