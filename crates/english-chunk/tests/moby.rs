//! Moby spot-eval for the chunker: hand-chunked gold on Moby-Dick
//! sentences (tokens/tags from the prose eval), asserting exact chunk
//! sequences. Chunking is deterministic rules over gold tags, so these
//! are regression pins (bar 1.0 by construction), not accuracy bars:
//! they verify implementation == spec (`README.md`), while the spec's
//! linguistic merit rests on CoNLL-2000 fidelity argued there.

use english_chunk::{ChunkKind, ChunkKind as K, chunk_tagged};
use english_pos::Tag;

fn pieces(pairs: &[(&str, Tag)]) -> Vec<(String, Tag)> {
    pairs.iter().map(|(w, t)| (w.to_string(), *t)).collect()
}

fn check(pairs: &[(&str, Tag)], want: &[ChunkKind]) {
    let input = pieces(pairs);
    let got: Vec<ChunkKind> = chunk_tagged(&input).iter().map(|c| c.kind()).collect();
    let words: Vec<&str> = pairs.iter().map(|(w, _)| *w).collect();
    assert_eq!(got, want, "sentence: {words:?}");
}

#[test]
fn moby_spot_chunks() {
    // Call me Ishmael.
    check(
        &[
            ("Call", Tag::Verb),
            ("me", Tag::Pron),
            ("Ishmael", Tag::Propn),
            (".", Tag::Punct),
        ],
        &[K::Verb, K::Noun, K::Noun, K::Punct],
    );
    // I thought I would sail about a little and see the watery part of the world.
    check(
        &[
            ("I", Tag::Pron),
            ("thought", Tag::Verb),
            ("I", Tag::Pron),
            ("would", Tag::Aux),
            ("sail", Tag::Verb),
            ("about", Tag::Adp),
            ("a", Tag::Det),
            ("little", Tag::Adj),
            ("and", Tag::Cconj),
            ("see", Tag::Verb),
            ("the", Tag::Det),
            ("watery", Tag::Adj),
            ("part", Tag::Noun),
            ("of", Tag::Adp),
            ("the", Tag::Det),
            ("world", Tag::Noun),
            (".", Tag::Punct),
        ],
        &[
            K::Noun,
            K::Verb,
            K::Noun,
            K::Verb,
            K::Prep,
            K::Conj,
            K::Verb,
            K::Noun,
            K::Prep,
            K::Punct,
        ],
    );
    // It is a way I have of driving off the spleen and regulating the circulation.
    // (`off the spleen` is one Prep: ADP + nominal run.)
    check(
        &[
            ("It", Tag::Pron),
            ("is", Tag::Aux),
            ("a", Tag::Det),
            ("way", Tag::Noun),
            ("I", Tag::Pron),
            ("have", Tag::Verb),
            ("of", Tag::Adp),
            ("driving", Tag::Verb),
            ("off", Tag::Adp),
            ("the", Tag::Det),
            ("spleen", Tag::Noun),
            ("and", Tag::Cconj),
            ("regulating", Tag::Verb),
            ("the", Tag::Det),
            ("circulation", Tag::Noun),
            (".", Tag::Punct),
        ],
        &[
            K::Noun,
            K::Verb,
            K::Noun,
            K::Noun,
            K::Verb,
            K::Prep,
            K::Verb,
            K::Prep,
            K::Conj,
            K::Verb,
            K::Noun,
            K::Punct,
        ],
    );
    // Right and left, the streets take you waterward.
    check(
        &[
            ("Right", Tag::Adv),
            ("and", Tag::Cconj),
            ("left", Tag::Adv),
            (",", Tag::Punct),
            ("the", Tag::Det),
            ("streets", Tag::Noun),
            ("take", Tag::Verb),
            ("you", Tag::Pron),
            ("waterward", Tag::Adv),
            (".", Tag::Punct),
        ],
        &[
            K::Adverb,
            K::Conj,
            K::Adverb,
            K::Punct,
            K::Noun,
            K::Verb,
            K::Noun,
            K::Adverb,
            K::Punct,
        ],
    );
    // Circumambulate the city of a dreamy Sabbath afternoon.
    check(
        &[
            ("Circumambulate", Tag::Verb),
            ("the", Tag::Det),
            ("city", Tag::Noun),
            ("of", Tag::Adp),
            ("a", Tag::Det),
            ("dreamy", Tag::Adj),
            ("Sabbath", Tag::Propn),
            ("afternoon", Tag::Noun),
            (".", Tag::Punct),
        ],
        &[K::Verb, K::Noun, K::Prep, K::Punct],
    );
    // Go from Corlears Hook to Coenties Slip, and from thence, by Whitehall, northward.
    check(
        &[
            ("Go", Tag::Verb),
            ("from", Tag::Adp),
            ("Corlears", Tag::Propn),
            ("Hook", Tag::Propn),
            ("to", Tag::Adp),
            ("Coenties", Tag::Propn),
            ("Slip", Tag::Propn),
            (",", Tag::Punct),
            ("and", Tag::Cconj),
            ("from", Tag::Adp),
            ("thence", Tag::Adv),
            (",", Tag::Punct),
            ("by", Tag::Adp),
            ("Whitehall", Tag::Propn),
            (",", Tag::Punct),
            ("northward", Tag::Adv),
            (".", Tag::Punct),
        ],
        &[
            K::Verb,
            K::Prep,
            K::Prep,
            K::Punct,
            K::Conj,
            K::Prep,
            K::Adverb,
            K::Punct,
            K::Prep,
            K::Punct,
            K::Adverb,
            K::Punct,
        ],
    );
    // What do you see?
    check(
        &[
            ("What", Tag::Pron),
            ("do", Tag::Aux),
            ("you", Tag::Pron),
            ("see", Tag::Verb),
            ("?", Tag::Punct),
        ],
        &[K::Noun, K::Verb, K::Noun, K::Verb, K::Punct],
    );
    // There is magic in it. (`in it` is one Prep, like `off the spleen`.)
    check(
        &[
            ("There", Tag::Pron),
            ("is", Tag::Verb),
            ("magic", Tag::Noun),
            ("in", Tag::Adp),
            ("it", Tag::Pron),
            (".", Tag::Punct),
        ],
        &[K::Noun, K::Verb, K::Noun, K::Prep, K::Punct],
    );
    // Yet here they all unite. (`they` + lone-DET `all`: two Nouns;
    // DET cannot follow a nominal in one run.)
    check(
        &[
            ("Yet", Tag::Cconj),
            ("here", Tag::Adv),
            ("they", Tag::Pron),
            ("all", Tag::Det),
            ("unite", Tag::Verb),
            (".", Tag::Punct),
        ],
        &[K::Conj, K::Adverb, K::Noun, K::Noun, K::Verb, K::Punct],
    );
    // But look! here come more crowds, pacing straight for the water.
    check(
        &[
            ("But", Tag::Cconj),
            ("look", Tag::Verb),
            ("!", Tag::Punct),
            ("here", Tag::Adv),
            ("come", Tag::Verb),
            ("more", Tag::Adj),
            ("crowds", Tag::Noun),
            (",", Tag::Punct),
            ("pacing", Tag::Verb),
            ("straight", Tag::Adv),
            ("for", Tag::Adp),
            ("the", Tag::Det),
            ("water", Tag::Noun),
            (".", Tag::Punct),
        ],
        &[
            K::Conj,
            K::Verb,
            K::Punct,
            K::Adverb,
            K::Verb,
            K::Noun,
            K::Punct,
            K::Verb,
            K::Adverb,
            K::Prep,
            K::Punct,
        ],
    );
}
