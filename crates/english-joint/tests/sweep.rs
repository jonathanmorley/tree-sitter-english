//! Sweep book-eval on the joint tag head (Stage-1 accuracy gate).
//! Gold table shared with `english-pos-neural` (single source, never
//! duplicated); bar 0.87 like every tag path. Genre/hard/moby stand
//! on the torch screen with identical weights (parity-proven decode).
//! The i8 variant re-gates the same bar on the quantized artifact.

use english_joint::{JointModel, QJointModel};

const VENDORED_I8: &str = include_str!("../weights/joint-i8.json");

#[path = "../../english-pos-neural/tests/tables/sweep.rs"]
mod tables;

#[test]
fn sweep_joint_tags_meet_bar() {
    let weights = match std::fs::read_to_string("/tmp/opencode/round3/joint.json") {
        Ok(w) => w,
        Err(_) => {
            eprintln!("skip: /tmp joint weights absent");
            return;
        }
    };
    let model = JointModel::from_json(&weights).expect("weights load");
    let mut ok = 0usize;
    let mut tot = 0usize;
    for (_, _, words, gold) in tables::SENTENCES {
        let p = model.parse(words);
        assert_eq!(p.tags.len(), gold.len());
        for (g, want) in p.tags.iter().zip(gold.iter()) {
            tot += 1;
            ok += (g.upos() == *want) as usize;
        }
    }
    let acc = ok as f64 / tot as f64;
    eprintln!("joint sweep tags: {ok}/{tot} = {acc:.4}");
    assert!(acc >= 0.87, "sweep bar");
}

#[test]
fn sweep_joint_quant_tags_meet_bar() {
    let model = QJointModel::from_json(VENDORED_I8).expect("i8 weights load");
    let mut ok = 0usize;
    let mut tot = 0usize;
    for (_, _, words, gold) in tables::SENTENCES {
        let p = model.parse(words);
        assert_eq!(p.tags.len(), gold.len());
        for (g, want) in p.tags.iter().zip(gold.iter()) {
            tot += 1;
            ok += (g.upos() == *want) as usize;
        }
    }
    let acc = ok as f64 / tot as f64;
    eprintln!("joint i8 sweep tags: {ok}/{tot} = {acc:.4}");
    assert!(acc >= 0.87, "sweep bar");
}
