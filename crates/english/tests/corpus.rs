//! Mirror of `tree-sitter test` in Rust.
//!
//! Reads `test/corpus/*.txt`, parses each input with the English grammar
//! and compares the resulting S-expression against the expectation. This
//! exists so the corpus suite runs under `cargo test`: the tree-sitter CLI
//! only links C/C++ scanners, so once the external scanner is rewritten in
//! Rust this harness becomes the only corpus runner.

use std::fs;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

struct CorpusTest {
    file: String,
    name: String,
    input: String,
    expected: String,
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test")
        .join("corpus")
}

fn parse_corpus_file(path: &Path) -> Vec<CorpusTest> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let lines: Vec<&str> = content.lines().collect();
    let file = path
        .file_name()
        .expect("corpus path has no file name")
        .to_string_lossy()
        .into_owned();

    let mut tests = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }
        assert!(
            is_divider(lines[i]),
            "expected '==================' divider in {}, line {}",
            file,
            i + 1
        );
        let name = lines[i + 1].to_string();
        assert!(
            is_divider(lines[i + 2]),
            "expected divider after test name in {file}:{name}"
        );
        i += 3;

        let mut input = Vec::new();
        while i < lines.len() && lines[i] != "---" {
            input.push(lines[i]);
            i += 1;
        }
        assert!(i < lines.len(), "missing '---' separator in {file}:{name}");
        i += 1;

        let mut expected = Vec::new();
        while i < lines.len() && !is_divider(lines[i]) {
            expected.push(lines[i]);
            i += 1;
        }

        tests.push(CorpusTest {
            file: file.clone(),
            name,
            // Lines strictly between the header and `---`, joined verbatim:
            // leading/trailing blank lines are part of the input (they only
            // matter for all-blank inputs, where extras absorb singles and
            // `paragraph_break` matches doubles).
            input: input.join("\n"),
            expected: expected.join("\n"),
        });
    }
    tests
}

fn is_divider(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|c| c == '=')
}

fn sexp(node: Node, out: &mut String) {
    out.push('(');
    out.push_str(node.kind());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        out.push(' ');
        sexp(child, out);
    }
    out.push(')');
}

fn normalize(sexp: &str) -> String {
    sexp.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn corpus() {
    let dir = corpus_dir();
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .map(|entry| entry.expect("failed to read corpus entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no corpus files in {}", dir.display());

    let mut parser = Parser::new();
    parser
        .set_language(&english::language())
        .expect("failed to load English grammar");

    let mut failures = Vec::new();
    let mut total = 0;
    for path in &files {
        for test in parse_corpus_file(path) {
            total += 1;
            let tree = parser
                .parse(&test.input, None)
                .unwrap_or_else(|| panic!("parse returned None for {}", test.name));
            let mut actual = String::new();
            sexp(tree.root_node(), &mut actual);
            if normalize(&actual) == normalize(&test.expected) {
                println!("ok - {}:{}", test.file, test.name);
            } else {
                failures.push(format!(
                    "{}:{}\n  input:    {:?}\n  expected: {}\n  actual:   {}",
                    test.file,
                    test.name,
                    test.input,
                    normalize(&test.expected),
                    normalize(&actual)
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{}/{} corpus tests failed:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
    println!("{total} corpus tests passed");
}
