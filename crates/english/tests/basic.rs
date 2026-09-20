use english::{Document, WordKind};

#[test]
fn hello_world_structure() {
    let doc = Document::parse("Hello world.\n");
    assert!(!doc.has_error());

    let paragraphs = doc.paragraphs();
    assert_eq!(paragraphs.len(), 1);

    let sentences = paragraphs[0].sentences();
    assert_eq!(sentences.len(), 1);

    let clauses = sentences[0].clauses();
    assert_eq!(clauses.len(), 1);
    assert!(!clauses[0].is_subordinate());

    let words: Vec<_> = clauses[0].words();
    assert_eq!(words.len(), 2);
    assert!(words.iter().all(|w| w.kind() == WordKind::Word));
    assert_eq!(words[0].text(), "Hello");
    assert_eq!(words[1].text(), "world");
    assert_eq!(clauses[0].text(), "Hello world");
}

#[test]
fn abbreviation_does_not_split_sentence() {
    // Mirrors test/corpus/abbreviations.txt.
    let doc = Document::parse("Mr. Smith arrived.\n");
    assert!(!doc.has_error());

    let sentences = doc.paragraphs()[0].sentences();
    assert_eq!(sentences.len(), 1);

    let texts: Vec<_> = sentences[0].clauses()[0]
        .words()
        .iter()
        .map(|w| w.text())
        .collect();
    assert_eq!(texts, ["Mr", "Smith", "arrived"]);
}

#[test]
fn initialism_is_one_dotted_token() {
    let doc = Document::parse("The H.M.S. Beagle sailed.\n");
    assert!(!doc.has_error());

    let words = doc.paragraphs()[0].sentences()[0].clauses()[0].words();
    assert_eq!(words.len(), 4);
    assert_eq!(words[1].kind(), WordKind::Dotted);
    assert_eq!(words[1].text(), "H.M.S.");
}

#[test]
fn semicolon_joins_coordinate_clauses() {
    // Mirrors test/corpus/punctuation.txt.
    let doc = Document::parse("I came; I saw.\n");
    assert!(!doc.has_error());

    let clauses = doc.paragraphs()[0].sentences()[0].clauses();
    assert_eq!(clauses.len(), 2);
    assert_eq!(clauses[0].text(), "I came");
    assert_eq!(clauses[1].text(), "I saw");
}

#[test]
fn subordinate_clause_reports_subordinator() {
    let doc = Document::parse("He left because he was tired.\n");
    assert!(!doc.has_error());

    let clauses = doc.paragraphs()[0].sentences()[0].clauses();
    assert_eq!(clauses.len(), 2);
    assert!(!clauses[0].is_subordinate());
    assert!(clauses[1].is_subordinate());
    assert_eq!(clauses[1].subordinator(), Some("because"));
}

#[test]
fn blank_line_separates_paragraphs() {
    let doc = Document::parse("First.\n\nSecond.\n");
    assert!(!doc.has_error());
    assert_eq!(doc.paragraphs().len(), 2);
}

#[test]
fn empty_input_parses_cleanly() {
    let doc = Document::parse("");
    assert!(!doc.has_error());
    assert!(doc.paragraphs().is_empty());
}

#[test]
fn hyphenated_compound_is_one_word() {
    // Mirrors test/corpus/hyphens.txt.
    let doc = Document::parse("The well-known fact stood.\n");
    assert!(!doc.has_error());

    let words = doc.paragraphs()[0].sentences()[0].clauses()[0].words();
    assert_eq!(words.len(), 4);
    assert_eq!(words[1].kind(), WordKind::Word);
    assert_eq!(words[1].text(), "well-known");
}

#[test]
fn blank_lines_only_parse_cleanly() {
    let doc = Document::parse("\n\n");
    assert!(!doc.has_error());
    assert!(doc.paragraphs().is_empty());
}
