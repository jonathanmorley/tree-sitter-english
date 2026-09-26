//! Forensics: per-template weight votes for one token in context.
//!
//! Re-derives each template id with [`english_pos::hash_feature`] (the
//! discriminants are documented there; this list must track
//! [`english_pos::features`]) and prints the NOUN vs VERB weights, so a
//! surprising tag can be traced to its voting features.
//!
//! Usage: `cargo run -p english-pos --example forensics -- Time flies like an arrow .`

use english_pos::{Model, TAGS, hash_feature};

fn noun() -> usize {
    TAGS.iter().position(|t| *t == "NOUN").unwrap()
}

fn verb() -> usize {
    TAGS.iter().position(|t| *t == "VERB").unwrap()
}

fn show(model: &Model, name: &str, id: u64) {
    match model.feature_weights(id) {
        Some(arr) => println!(
            "  {name:<14} id={id:016x} NOUN={:+6.1} VERB={:+6.1}",
            arr[noun()],
            arr[verb()]
        ),
        None => println!("  {name:<14} id={id:016x} (pruned)"),
    }
}

fn main() {
    // Usage: forensics -- <words...> [--at N]. Index defaults to 1.
    let mut at = 1usize;
    let mut words: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--at" {
            at = args.next().expect("--at needs a number").parse().unwrap();
        } else {
            words.push(a);
        }
    }
    assert!(!words.is_empty(), "usage: forensics -- <words...> [--at N]");
    assert!(at < words.len(), "--at out of range");
    let model =
        Model::from_json(include_str!("../weights/upos.json")).expect("invalid weights JSON");
    let lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
    // Tag history as the decoder would see it: retag prefix to get prev tags.
    let prefix_tags = model.tag(&words[..at]);
    let prev1 = prefix_tags
        .last()
        .map(|t| t.upos())
        .unwrap_or(english_pos::START1)
        .to_string();
    let prev2 = if prefix_tags.len() >= 2 {
        prefix_tags[prefix_tags.len() - 2].upos().to_string()
    } else {
        english_pos::START2.to_string()
    };
    let w = lower[at].as_str();
    println!("token {:?} (prev {prev2} {prev1})", words[at]);
    show(&model, "bias", 0x9e3779b97f4a7c15 ^ 0x01);
    show(&model, "w", hash_feature(0x10, &[w]));
    show(
        &model,
        "w-1",
        hash_feature(
            0x11,
            &[if at > 0 {
                &lower[at - 1]
            } else {
                english_pos::START1
            }],
        ),
    );
    show(
        &model,
        "w+1",
        hash_feature(
            0x12,
            &[lower
                .get(at + 1)
                .map(String::as_str)
                .unwrap_or(english_pos::START1)],
        ),
    );
    show(&model, "t-1", hash_feature(0x13, &[&prev1]));
    show(&model, "t-2", hash_feature(0x14, &[&prev2]));
    show(&model, "t-1+2", hash_feature(0x15, &[&prev1, &prev2]));
    let chars: Vec<char> = w.chars().collect();
    for n in 1..=3 {
        if chars.len() >= n {
            let p: String = chars[..n].iter().collect();
            let s: String = chars[chars.len() - n..].iter().collect();
            show(
                &model,
                &format!("pref{n}"),
                hash_feature(0x20 + n as u8, &[&p]),
            );
            show(
                &model,
                &format!("suf{n}"),
                hash_feature(0x24 + n as u8, &[&s]),
            );
        }
    }
    let raw_word = words[at].as_str();
    if raw_word.chars().next().is_some_and(|c| c.is_uppercase())
        && raw_word.chars().skip(1).all(|c| !c.is_uppercase())
    {
        show(&model, "title", 0x9e3779b97f4a7c15 ^ 0x02);
    }
}
