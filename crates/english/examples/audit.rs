//! Real-prose error audit: parse files, count ERROR/MISSING nodes, and
//! (for `examples/`) verify byte-identical output against the checked-in
//! `.parse.txt` snapshots (proving the Rust scanner matches the old C one).
//!
//! Errors split into prose vs transcription (Gutenberg `_`/`*` markup,
//! which is out of grammar scope): only the prose histogram measures
//! grammar quality.
//!
//! Usage: `cargo run -p english --example audit -- [files...]`
//! With no files, audits `examples/*.txt`. Snapshots are picked up as
//! `<input>.parse.txt` siblings.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser, Point};

#[derive(Default)]
struct Stats {
    paragraphs: usize,
    sentences: usize,
    clauses: usize,
    words: usize,
    error_nodes: usize,
    missing_nodes: usize,
    error_kinds: std::collections::HashMap<String, usize>,
    transcription_kinds: std::collections::HashMap<String, usize>,
    error_examples: std::collections::HashMap<String, String>,
    contexts: Vec<String>,
}

fn context(source: &str, byte: usize) -> String {
    let from = byte.saturating_sub(60);
    let to = (byte + 60).min(source.len());
    // Snap to char boundaries.
    let mut from = from;
    while from < byte && !source.is_char_boundary(from) {
        from += 1;
    }
    let mut to = to;
    while to > byte && !source.is_char_boundary(to) {
        to -= 1;
    }
    source[from..to].replace('\n', "\\n")
}

fn describe(source: &str, node: Node) -> String {
    let Point { row, column } = node.start_position();
    format!(
        "[{row},{column}] {} {:?} ctx: {:?}",
        node.kind(),
        &source[node.start_byte()..node.end_byte()],
        context(source, node.start_byte())
    )
}

/// Gutenberg transcription markup, not English prose: italics markers,
/// footnote markers, and section breaks. Bucketed separately so the
/// error histogram measures the grammar, not the transcription.
fn is_transcription(text: &str) -> bool {
    text.contains('_') || text.contains('*')
}

fn walk(source: &str, node: Node, stats: &mut Stats, in_error: bool) {
    let nested = in_error || node.is_error();
    if node.is_error() {
        stats.error_nodes += 1;
        if !in_error {
            let text = source[node.start_byte()..node.end_byte()].to_string();
            let map = if is_transcription(&text) {
                &mut stats.transcription_kinds
            } else {
                &mut stats.error_kinds
            };
            *map.entry(text.clone()).or_insert(0) += 1;
            stats
                .error_examples
                .entry(text)
                .or_insert_with(|| describe(source, node));
            if stats.contexts.len() < 20 {
                stats
                    .contexts
                    .push(format!("ERROR {}", describe(source, node)));
            }
        }
    }
    if node.is_missing() {
        stats.missing_nodes += 1;
        if stats.contexts.len() < 20 {
            stats
                .contexts
                .push(format!("MISSING {}", describe(source, node)));
        }
    }
    match node.kind() {
        "paragraph" => stats.paragraphs += 1,
        "sentence" => stats.sentences += 1,
        "clause" | "subordinate_clause" => stats.clauses += 1,
        "word" | "dotted" | "number" => stats.words += 1,
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(source, child, stats, nested);
    }
}

/// Positioned S-expression matching `tree-sitter parse` output: a node's
/// closing paren follows its last child with no newline.
fn write_sexp(node: Node, out: &mut String, depth: usize) {
    let indent = "  ".repeat(depth);
    let s = node.start_position();
    let e = node.end_position();
    let _ = write!(
        out,
        "{indent}({} [{}, {}] - [{}, {}]",
        node.kind(),
        s.row,
        s.column,
        e.row,
        e.column
    );
    let mut cursor = node.walk();
    let children: Vec<Node> = node.named_children(&mut cursor).collect();
    if children.is_empty() {
        out.push(')');
    } else {
        for child in children {
            out.push('\n');
            write_sexp(child, out, depth + 1);
        }
        out.push(')');
    }
}

fn audit(path: &Path) -> bool {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let mut parser = Parser::new();
    parser
        .set_language(&english::language())
        .expect("failed to load English grammar");
    let start = std::time::Instant::now();
    let tree = parser.parse(&source, None).expect("parse returned None");
    let elapsed = start.elapsed();

    let mut stats = Stats::default();
    walk(&source, tree.root_node(), &mut stats, false);
    let mut sexp = String::new();
    write_sexp(tree.root_node(), &mut sexp, 0);
    sexp.push('\n');

    let mut ok = true;
    // Inputs are `<name>.txt` and snapshots are `<name>.txt.parse.txt`:
    let candidate = PathBuf::from(format!("{}.parse.txt", path.display()));
    if let Ok(expected) = fs::read_to_string(&candidate) {
        // Snapshots are pure S-expressions, but one checked-in file also
        // captured the CLI's trailing `<path>\tParse: …` summary line
        // (including nondeterministic timing). Drop footer lines.
        let expected: String = expected
            .lines()
            .filter(|line| !line.contains("\tParse:"))
            .collect::<Vec<_>>()
            .join("\n");
        if sexp.trim_end() == expected.trim_end() {
            println!("  snapshot: identical");
        } else {
            ok = false;
            println!("  snapshot: DIFFERS from checked-in output");
            let dump = PathBuf::from(format!(
                "target/audit-{}.actual",
                path.file_stem()
                    .expect("input has no stem")
                    .to_string_lossy()
            ));
            fs::create_dir_all("target").ok();
            fs::write(&dump, &sexp).expect("failed to write actual output");
            println!("  actual written to {}", dump.display());
        }
    }

    let prose_errors: usize = stats.error_kinds.values().sum();
    let transcription_errors: usize = stats.transcription_kinds.values().sum();
    println!(
        "{}: {} bytes, {} para / {} sent / {} clauses / {} words, {} error ({} prose / {} transcription) / {} missing, has_error={}, {:?}",
        path.display(),
        source.len(),
        stats.paragraphs,
        stats.sentences,
        stats.clauses,
        stats.words,
        stats.error_nodes,
        prose_errors,
        transcription_errors,
        stats.missing_nodes,
        tree.root_node().has_error(),
        elapsed
    );
    for e in &stats.contexts {
        println!("  {e}");
    }
    let mut kinds: Vec<_> = stats.error_kinds.iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(a.1));
    for (text, count) in kinds.iter().take(10) {
        println!("  error {count}x: {text:?}");
        if let Some(example) = stats.error_examples.get(*text) {
            println!("    e.g. {example}");
        }
    }
    let mut transcription: Vec<_> = stats.transcription_kinds.iter().collect();
    transcription.sort_by(|a, b| b.1.cmp(a.1));
    for (text, count) in transcription.iter().take(5) {
        println!("  transcription {count}x: {text:?}");
    }
    ok
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let files: Vec<PathBuf> = if args.is_empty() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("examples");
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .expect("failed to read examples dir")
            .map(|e| e.expect("bad entry").path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
            .filter(|p| !p.to_string_lossy().ends_with(".parse.txt"))
            .collect();
        entries.sort();
        entries
    } else {
        args.iter().map(PathBuf::from).collect()
    };

    let mut failed = 0;
    for path in &files {
        if !audit(path) {
            failed += 1;
        }
        println!();
    }
    if failed > 0 {
        panic!("{failed} file(s) differ from snapshots");
    }
}
