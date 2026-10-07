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
fn real_segmentation_not_naive() {
    // Abbreviations + times split naive `.`-splitters into 3+ pieces;
    // the wasm-linked scanner holds them inside one sentence.
    let v: serde_json::Value =
        serde_json::from_str(&analyze("Mr. Smith arrived at 10:30.")).unwrap();
    let sents = v["sentences"].as_array().unwrap();
    assert_eq!(sents.len(), 1);
    // Tree carries real token kinds (abbreviation dots surface).
    let kinds: Vec<&str> = sents[0]["tree"][0]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["k"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"Period"));
    assert!(kinds.contains(&"Number"));
}

#[test]
fn weasel_fires() {
    let v: serde_json::Value = serde_json::from_str(&analyze("It was very good.")).unwrap();
    let findings = v["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["rule"], "syntax.weasel");
}

#[test]
fn complexity_fires_with_real_clauses() {
    let text = "She laughed and he cried and they left because it ended.";
    let v: serde_json::Value = serde_json::from_str(&analyze(text)).unwrap();
    let sents = v["sentences"].as_array().unwrap();
    assert_eq!(sents.len(), 1);
    assert!(sents[0]["clauses"].as_u64().unwrap() >= 4);
    let findings = v["findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["rule"] == "syntax.clause-complexity"));
}
