//! Uncertainty harvester: rank book sentences by tagger difficulty
//! for hand-tagging into eval sets (never auto-labels — gold tags
//! stay hand-made per EWT discipline; cf. the no-misparse rule).
//!
//! Per sentence: low-margin count, beam-vs-greedy diffs, correction
//! fires, and pattern flags (3sg-`-s` noun shapes, `that`, mid
//! titlecase, coordination depth, prep runs, subordination,
//! quote/paren presence, MWE-ish adverbial shapes). Score ranks
//! review priority; a human picks diverse, English-prose tops into
//! `tests/hard.rs`-style evals with bars set from measurement.
//!
//! Usage: `cargo run --release -p english-pos --example candidates --
//!   <file>... [topN]`
//! Always run with `--release` (dev builds are ~10x slower).

use english_pos::{RULES, Tag, apply_rules, sentence_pieces};

fn is_s_shaped(lower: &str) -> bool {
    lower.len() > 3
        && lower.as_bytes().ends_with(b"s")
        && !lower.ends_with("ss")
        && lower.bytes().all(|c| c.is_ascii_lowercase())
}

fn is_titlecase(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_uppercase() => chars.all(|c| !c.is_uppercase()),
        _ => false,
    }
}

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let (files, topn) = match raw_args.as_slice() {
        [] => panic!("usage: candidates <file>... [topN]"),
        [rest @ ..] => {
            let maybe_n = rest.last().unwrap().parse::<usize>().ok();
            match maybe_n {
                Some(n) if rest.len() > 1 => (rest[..rest.len() - 1].to_vec(), n),
                _ => (rest.to_vec(), 150),
            }
        }
    };
    let model = english_pos::Model::from_json(include_str!("../weights/upos.json"))
        .expect("invalid weights JSON");
    let mut rows: Vec<(i32, String)> = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).expect("failed to read input");
        let doc = english::Document::parse(&text);
        for (pi, para) in doc.paragraphs().iter().enumerate() {
            for (si, sent) in para.sentences().iter().enumerate() {
                let pieces = sentence_pieces(sent);
                if pieces.is_empty() {
                    continue;
                }
                let greedy = model.tag_margins(&pieces);
                let beam = model.tag_beam_margins(&pieces);
                let mut corrected: Vec<(Tag, f32)> = greedy.iter().map(|(t, m)| (*t, *m)).collect();
                let owned: Vec<String> = pieces.clone();
                apply_rules(&owned, &mut corrected, RULES);
                let tags: Vec<Tag> = greedy.iter().map(|(t, _)| *t).collect();

                let mut score = 0i32;
                let mut flags: Vec<&str> = Vec::new();
                let mut low = 0;
                let mut min_m = f32::INFINITY;
                for (_, m) in &greedy {
                    if *m < 2.0 {
                        low += 1;
                    }
                    min_m = min_m.min(*m);
                }
                score += low;
                let mut beam_diff = 0;
                for ((g, _), (b, _)) in greedy.iter().zip(beam.iter()) {
                    if g != b {
                        beam_diff += 1;
                    }
                }
                score += 2 * beam_diff;
                let mut fires: Vec<&str> = Vec::new();
                for (i, ((t, _), (c, _))) in greedy.iter().zip(corrected.iter()).enumerate() {
                    if t != c {
                        score += 2;
                        // Attribute by re-testing rules (offline only).
                        for r in RULES {
                            let snap: Vec<Tag> = greedy.iter().map(|(x, _)| *x).collect();
                            let low: Vec<String> = owned.iter().map(|p| p.to_lowercase()).collect();
                            if (r.test)(&owned, &snap, &low, i).is_some() {
                                fires.push(r.name);
                                break;
                            }
                        }
                    }
                }
                // Pattern flags (shape-level, model-independent).
                let mut s_noun = 0;
                let mut thats = 0;
                let mut title_mid = 0;
                let mut cconj = 0;
                let mut adp_run = 0;
                let mut best_adp = 0;
                let mut subord = false;
                for (idx, (w, t)) in pieces.iter().zip(tags.iter()).enumerate() {
                    let lw = w.to_lowercase();
                    if *t == Tag::Noun && is_s_shaped(&lw) {
                        s_noun += 1;
                    }
                    if lw == "that" {
                        thats += 1;
                    }
                    if idx > 0 && is_titlecase(w) {
                        title_mid += 1;
                    }
                    if *t == Tag::Cconj {
                        cconj += 1;
                    }
                    if *t == Tag::Adp {
                        adp_run += 1;
                        best_adp = best_adp.max(adp_run);
                    } else {
                        adp_run = 0;
                    }
                    if *t == Tag::Sconj {
                        subord = true;
                    }
                }
                let raw_text: String = sent.text().replace(['\n', '\t'], " ");
                if s_noun > 0 {
                    score += s_noun;
                    flags.push("s-noun");
                }
                if thats > 1 {
                    score += thats;
                    flags.push("that-cascade");
                } else if thats == 1 {
                    score += 1;
                    flags.push("that");
                }
                if title_mid > 0 {
                    score += title_mid;
                    flags.push("title-mid");
                }
                if cconj > 1 {
                    score += cconj;
                    flags.push("coord");
                }
                if best_adp >= 3 {
                    score += best_adp;
                    flags.push("prep-chain");
                }
                if subord {
                    score += 1;
                    flags.push("subord");
                }
                if raw_text.contains('"')
                    || raw_text.contains('\u{201c}')
                    || raw_text.contains('\'')
                {
                    score += 1;
                    flags.push("quote");
                }
                if raw_text.contains('(') || raw_text.contains('—') {
                    score += 1;
                    flags.push("paren/dash");
                }
                if sent.has_error() {
                    score += 3;
                    flags.push("ERROR");
                }
                if score == 0 {
                    continue;
                }
                rows.push((
                    score,
                    format!(
                        "{score}\t{path}:p{pi}s{si}\tmin_m={min_m:.1}\tbeamΔ={beam_diff}\tfires={fires:?}\tflags={flags:?}\t{raw_text}",
                    ),
                ));
            }
        }
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, row) in rows.iter().take(topn) {
        println!("{row}");
    }
    eprintln!("# ranked {} scored sentences", rows.len());
}
