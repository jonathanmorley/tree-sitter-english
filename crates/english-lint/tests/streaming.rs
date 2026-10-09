//! Streaming identity: `lint_streaming_shallow` emits byte-identical
//! findings to the batch shallow path on adversarial inputs
//! (dash-handoff blank absorption, abbreviations, parentheticals,
//! multi-paragraph, empty/whitespace-only, CRLF).

use english_lint::{
    ClauseComplexity, Hedge, Rule, SentenceLength, VagueDemonstrative, Weasel, annotate_shallow,
    lint_streaming_shallow,
};

fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(SentenceLength::default()),
        Box::new(ClauseComplexity::default()),
        Box::new(Weasel),
        Box::new(Hedge),
        Box::new(VagueDemonstrative),
    ]
}

fn batch(source: &str) -> Vec<english_lint::Finding> {
    let model =
        english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap();
    let doc = annotate_shallow(&model, source);
    let rs = rules();
    let refs: Vec<&dyn Rule> = rs.iter().map(|r| r.as_ref()).collect();
    let mut out = Vec::new();
    for rule in &refs {
        out.extend(rule.check(&doc));
    }
    out.sort_by(|a, b| a.span.start.cmp(&b.span.start).then(a.rule.cmp(b.rule)));
    out
}

fn streamed(source: &str) -> Vec<english_lint::Finding> {
    let model =
        english_pos::Model::from_json(include_str!("../../english-pos/weights/upos.json")).unwrap();
    let rs = rules();
    let refs: Vec<&dyn Rule> = rs.iter().map(|r| r.as_ref()).collect();
    let mut out = Vec::new();
    lint_streaming_shallow(&model, source, &refs, &mut |f| out.push(f));
    out
}

#[test]
fn streaming_matches_batch() {
    let cases = [
        "Time flies like an arrow. It was really very good.",
        "Mr. Smith arrived at 10:30, and Mrs. Jones left because the meeting ended.",
        "He said—\n\nNext paragraph here.",
        "The whale (a huge beast, quite large) swam on, and on, and on, and on, and on, and on.",
        "First paragraph here.\n\nSecond paragraph, extremely long and winding through many clauses that keep going.",
        "",
        "   \n\n  \n",
        "Line one.\r\n\r\nLine two after CRLF blank.",
        "She laughed — and then she cried; it cost $5, so very good.",
        // Document-context rule: the vague demonstrative in sentence
        // two needs sentence one's nominals (streaming must carry
        // the previous sentence, not isolate each span).
        "She kept the letter and the photograph on the desk. This upset him.",
    ];
    for text in cases {
        assert_eq!(batch(text), streamed(text), "input: {text:?}");
    }
    // The parity case above must actually exercise the rule (a
    // vacuous pass with zero findings on both sides proves nothing).
    let vague = "She kept the letter and the photograph on the desk. This upset him.";
    assert!(
        !batch(vague).is_empty(),
        "parity case fires nothing even in batch"
    );
}
