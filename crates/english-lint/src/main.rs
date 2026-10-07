//! `english-lint file...`: Vale-comparable prose linting.
//! Prints `path:line:col [rule] message` per finding, one rule per
//! line, document order; exits 1 when findings exist (standard lint
//! behavior), 0 when clean. Weights errors exit 2 (never silent).

use std::process::ExitCode;

use english_lint::{
    ClauseComplexity, Models, Nominalization, Passive, Rule, SentenceLength, line_col, lint,
};

fn main() -> ExitCode {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("usage: english-lint <file>...");
        return ExitCode::from(2);
    }
    let models = match Models::load_workspace() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("english-lint: {e}");
            eprintln!("(parser + labeler weights are Tier-1 lazy assets, gitignored;");
            eprintln!(" train via english-dep-train --beam-train 4 and --labels)");
            return ExitCode::from(2);
        }
    };
    let length = SentenceLength::default();
    let complexity = ClauseComplexity::default();
    let rules: Vec<&dyn Rule> = vec![&Passive, &Nominalization, &length, &complexity];
    let mut total = 0usize;
    for path in &files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("english-lint: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        };
        for f in lint(&models, &source, &rules) {
            let (line, col) = line_col(&source, f.span.start);
            println!("{path}:{line}:{col} [{}] {}", f.rule, f.message);
            total += 1;
        }
    }
    if total > 0 {
        eprintln!("english-lint: {total} finding(s)");
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
