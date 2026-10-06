//! Throwaway differential-testing helper (NOT SHIPPED — delete after
//! the oracle sentence-diff item): dumps one TSV row per parsed
//! sentence: path, start byte, end byte, has_error flag, first 80
//! chars on one line. The `scripts/sent-diff.py` oracle compares
//! these boundaries against Punkt + spaCy.
//!
//! Usage: `cargo run -p english --example sent_bounds -- <files...>`

use std::path::PathBuf;

fn main() {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    for path in &files {
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let doc = english::Document::parse(&source);
        for para in doc.paragraphs() {
            for sent in para.sentences() {
                let span = sent.span();
                let text: String = sent
                    .text()
                    .chars()
                    .take(80)
                    .collect::<String>()
                    .replace(['\n', '\t'], " ");
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    path.display(),
                    span.start,
                    span.end,
                    sent.has_error(),
                    text
                );
            }
        }
    }
}
