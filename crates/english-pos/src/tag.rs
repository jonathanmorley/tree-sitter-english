//! The 17 Universal POS tags.

use std::fmt;
use std::str::FromStr;

/// A Universal POS tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Adj,
    Adp,
    Adv,
    Aux,
    Cconj,
    Det,
    Intj,
    Noun,
    Num,
    Part,
    Pron,
    Propn,
    Punct,
    Sconj,
    Sym,
    Verb,
    X,
}

impl Tag {
    /// UPOS code (`NOUN`, `VERB`, …).
    pub fn upos(self) -> &'static str {
        match self {
            Tag::Adj => "ADJ",
            Tag::Adp => "ADP",
            Tag::Adv => "ADV",
            Tag::Aux => "AUX",
            Tag::Cconj => "CCONJ",
            Tag::Det => "DET",
            Tag::Intj => "INTJ",
            Tag::Noun => "NOUN",
            Tag::Num => "NUM",
            Tag::Part => "PART",
            Tag::Pron => "PRON",
            Tag::Propn => "PROPN",
            Tag::Punct => "PUNCT",
            Tag::Sconj => "SCONJ",
            Tag::Sym => "SYM",
            Tag::Verb => "VERB",
            Tag::X => "X",
        }
    }

    /// Parse a UPOS code; `None` for anything else.
    pub fn from_upos(code: &str) -> Option<Self> {
        Some(match code {
            "ADJ" => Tag::Adj,
            "ADP" => Tag::Adp,
            "ADV" => Tag::Adv,
            "AUX" => Tag::Aux,
            "CCONJ" => Tag::Cconj,
            "DET" => Tag::Det,
            "INTJ" => Tag::Intj,
            "NOUN" => Tag::Noun,
            "NUM" => Tag::Num,
            "PART" => Tag::Part,
            "PRON" => Tag::Pron,
            "PROPN" => Tag::Propn,
            "PUNCT" => Tag::Punct,
            "SCONJ" => Tag::Sconj,
            "SYM" => Tag::Sym,
            "VERB" => Tag::Verb,
            "X" => Tag::X,
            _ => return None,
        })
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.upos())
    }
}

impl FromStr for Tag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tag::from_upos(s).ok_or(())
    }
}
