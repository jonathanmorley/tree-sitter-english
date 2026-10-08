//! Word-level finding spans on the real pipeline (POS-only rules;
//! dep rules cannot run here — see the mock-anchored span asserts
//! in src/lib.rs). Byte positions hand-computed, never generated.

use english_lint::{Hedge, Rule, Weasel, annotate_shallow};

fn tagger() -> english_pos::Model {
    english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap()
}

#[test]
fn weasel_span_covers_intensifier_pair() {
    // Real pipeline: "It was very good." → pieces It/was/very/good
    // at 0..2/3..6/7..11/12..16; the finding covers "very good".
    let doc = annotate_shallow(&tagger(), "It was very good.");
    let got = Weasel.check(&doc);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].rule, "syntax.weasel");
    assert_eq!(got[0].span, 7..16, "covers \"very good\", not the sentence");
}

#[test]
fn hedge_span_covers_hedge_pair() {
    // Real pipeline: "It was so good." → so/good at 7..9/11..14;
    // the finding covers the pair. (`quite` would be the natural
    // pick but confidently mistags DET here — the documented FN
    // class — so the test uses `so`.)
    let doc = annotate_shallow(&tagger(), "It was so good.");
    let got = Hedge.check(&doc);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].rule, "syntax.hedge");
    assert_eq!(got[0].span, 7..14, "covers \"so good\", not the sentence");
}
