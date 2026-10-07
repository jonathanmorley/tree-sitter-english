use english_web::analyze;

#[test]
fn analyzes_plain_sentence() {
    let v: serde_json::Value = serde_json::from_str(&analyze("Time flies like an arrow.")).unwrap();
    let sents = v["sentences"].as_array().unwrap();
    assert_eq!(sents.len(), 1);
    let tags: Vec<&str> = sents[0]["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["tag"].as_str().unwrap())
        .collect();
    // Canonical flies-VERB reading holds on the demo path too.
    assert_eq!(tags, vec!["NOUN", "VERB", "ADP", "DET", "NOUN"]);
    assert!(v["findings"].as_array().unwrap().is_empty());
}

#[test]
fn weasel_fires() {
    let v: serde_json::Value = serde_json::from_str(&analyze("It was very good.")).unwrap();
    let findings = v["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["rule"], "syntax.weasel");
}

#[test]
fn multisentence_spans() {
    let v: serde_json::Value =
        serde_json::from_str(&analyze("Hello world. It was really bad.")).unwrap();
    let sents = v["sentences"].as_array().unwrap();
    assert_eq!(sents.len(), 2);
    assert_eq!(sents[1]["text"], "It was really bad.");
    let findings = v["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule"] == "syntax.weasel"));
}
