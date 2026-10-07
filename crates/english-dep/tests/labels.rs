//! Labeler smoke tests: the classifier memorizes tiny arcs
//! (machinery check, not a quality claim), stays deterministic,
//! and marks stranded tokens with "" (never gold).

use english_dep::LabelModel;

/// (words, tags, heads, rels); index 0 is the dummy (""/0/"").
type LabToy = Vec<(Vec<String>, Vec<String>, Vec<usize>, Vec<String>)>;

fn tiny() -> LabToy {
    // "Time flies": 1->2 nsubj, 2->0 root.
    vec![(
        vec!["time".to_string(), "flies".to_string()],
        vec!["NOUN".to_string(), "VERB".to_string()],
        vec![0, 2, 0],
        vec!["".to_string(), "nsubj".to_string(), "root".to_string()],
    )]
}

#[test]
fn labels_memorize_tiny_arcs() {
    let data = tiny();
    let model = LabelModel::train(&data, 20, 1);
    let (w, t, h, g) = &data[0];
    let got = model.predict(w, t, h);
    assert_eq!(&got[1..], &g[1..]);
}

#[test]
fn labels_are_deterministic() {
    let data = tiny();
    let a = LabelModel::train(&data, 20, 1).to_json().unwrap();
    let b = LabelModel::train(&data, 20, 1).to_json().unwrap();
    assert_eq!(a, b);
}

#[test]
fn labels_json_roundtrip() {
    let data = tiny();
    let json = LabelModel::train(&data, 20, 1).to_json().unwrap();
    let back = LabelModel::from_json(&json).unwrap();
    assert_eq!(back.labels(), &["nsubj".to_string(), "root".to_string()]);
    let (w, t, h, g) = &data[0];
    assert_eq!(&back.predict(w, t, h)[1..], &g[1..]);
    assert!(LabelModel::from_json(r#"{"labels": ["a"], "weights": {"1": {"BOGUS": 2}}}"#).is_err());
    assert!(LabelModel::from_json(r#"{"weights": {}}"#).is_err());
}

#[test]
fn stranded_tokens_carry_empty_label() {
    // Head usize::MAX (deadlock strand): no arc, no label.
    let data = tiny();
    let model = LabelModel::train(&data, 20, 1);
    let (w, t, _, _) = &data[0];
    let got = model.predict(w, t, &[0, usize::MAX, usize::MAX]);
    assert_eq!(got[1], "");
    assert_eq!(got[2], "");
}
