//! `english-lint [--batch] file...`: Vale-comparable prose linting.
//! Prints `path:line:col [rule] message` per finding, one rule per
//! line, document order; exits 1 when findings exist (standard lint
//! behavior), 0 when clean. Weights errors exit 2 (never silent).

use std::process::ExitCode;

use english_lint::{
    ClauseComplexity, CoordScope, Hedge, Models, NegScope, Nominalization, Passive, Rule,
    SentenceLength, VagueDemonstrative, Weasel, line_col, lint, lint_streaming,
};

fn main() -> ExitCode {
    let mut batch = false;
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| {
            if a == "--batch" {
                batch = true;
                false
            } else {
                true
            }
        })
        .collect();
    if files.is_empty() {
        eprintln!("usage: english-lint [--batch] <file...>");
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
    let weasel = Weasel;
    let hedge = Hedge;
    let coordscope = CoordScope;
    let negscope = NegScope;
    let vague = VagueDemonstrative;
    let rules: Vec<&dyn Rule> = vec![
        &Passive,
        &Nominalization,
        &length,
        &complexity,
        &weasel,
        &hedge,
        &vague,
        &coordscope,
        &negscope,
    ];
    let mut total = 0usize;
    for path in &files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("english-lint: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        };
        // Batch (legacy) vs streaming residency; findings identical
        // (pinned by tests/streaming.rs). `--batch` exists so the
        // shootout harness can measure both paths.
        let mut total_file = 0usize;
        if batch {
            for f in lint(&models, &source, &rules) {
                let (line, col) = line_col(&source, f.span.start);
                println!("{path}:{line}:{col} [{}] {}", f.rule, f.message);
                total_file += 1;
            }
        } else {
            lint_streaming(&models, &source, &rules, &mut |f| {
                let (line, col) = line_col(&source, f.span.start);
                println!("{path}:{line}:{col} [{}] {}", f.rule, f.message);
                total_file += 1;
            });
        }
        total += total_file;
    }
    if total > 0 {
        eprintln!("english-lint: {total} finding(s)");
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
