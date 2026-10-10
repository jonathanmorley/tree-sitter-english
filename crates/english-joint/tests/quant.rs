//! Stage-3 quant gates for the joint path: int8 artifact vs the
//! torch-f32 parity sample (pre-set smoke band ≤ 20 combined diffs —
//! decisive gates are dev/test/PUD + sweep) and the sweep tag bar.
//!
//! The i8 artifact is vendored (`weights/joint-i8.json`, single-digit
//! rule) so the sweep gate runs with no skips; the parity band still
//! needs /tmp vectors (EWT text) and skips without them. The f32
//! reference stays regenerable (`export_joint.py` + screen recipe,
//! dep-weights precedent) — the consumer chooses the path.

use english_joint::QJointModel;

#[test]
fn joint_quant_parity_band() {
    // /tmp-vs-/tmp by design (the sweep gate below is the vendored
    // one): the parity vectors embed EWT text, so this gate can never
    // run vendored-only anyway.
    let weights = match std::fs::read_to_string("/tmp/opencode/round3/joint-i8.json") {
        Ok(w) => w,
        Err(_) => {
            eprintln!("skip: /tmp joint i8 weights absent");
            return;
        }
    };
    let vectors = match std::fs::read_to_string("/tmp/opencode/round3/joint-parity.json") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skip: /tmp joint parity vectors absent");
            return;
        }
    };
    let model = QJointModel::from_json(&weights).expect("i8 weights load");
    let rows: Vec<serde_json::Value> = serde_json::from_str(&vectors).expect("vectors parse");
    let (mut dt, mut dh, mut dr) = (0, 0, 0);
    for r in &rows {
        let words: Vec<&str> = r["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_str().unwrap())
            .collect();
        let want_t: Vec<&str> = r["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        let want_h: Vec<i64> = r["heads"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h.as_i64().unwrap())
            .collect();
        let want_r: Vec<&str> = r["rels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        let p = model.parse(&words);
        for (g, w) in p.tags.iter().zip(want_t.iter()) {
            dt += (g.upos() != *w) as usize;
        }
        for (g, w) in p.heads.iter().zip(want_h.iter()) {
            dh += (*g as i64 != *w) as usize;
        }
        for (g, w) in p.rels.iter().zip(want_r.iter()) {
            dr += (*g != *w) as usize;
        }
    }
    eprintln!("joint i8 parity band: tags {dt} heads {dh} rels {dr}");
    assert!(dt + dh + dr <= 20, "quant smoke band");
}
