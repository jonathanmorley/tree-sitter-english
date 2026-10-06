//! External scanner: lexes words and decides bare periods.
//!
//! Ported from `src/scanner.c` (removed; see git history). The logic is
//! intentionally a 1:1 translation so the two can be diffed: `word`,
//! `conjunction` and `subordinator` are external so the scanner can
//! remember the last word across tokens, with state surviving incremental
//! re-parse through serialize/deserialize.
//!
//! Word lexing:
//!   - Collect letters (plus one internal apostrophe run and internal
//!     hyphen runs, matching compounds like `well-known`), lowercased into
//!     state. Curly right single quote (U+2019) is accepted as an
//!     apostrophe and normalized. Non-ASCII letters (æ, œ, é…) are word
//!     text too, stored as raw UTF-8 bytes that never match the
//!     ASCII-only closed-class lists.
//!   - Letter followed by '.' followed by a letter: refuse, so the internal
//!     `dotted` token absorbs initialism runs (H.M.S., e.g.) wholesale.
//!   - Closed-class words are emitted as conjunction/subordinator only where
//!     the grammar allows those symbols (valid_symbols check); everywhere
//!     else they lex as plain words ("For home he left").
//!
//! Period decision, in order:
//!   1. previous word is a single letter other than a/i -> PERIOD (spaced
//!      initial: "J. Smith"). a/i stay splittable: "am I. You".
//!   2. previous word is a known abbreviation (mr, dr, ...) -> PERIOD.
//!   3. dot followed over whitespace by a lowercase letter or digit ->
//!      PERIOD: English sentences do not start lowercase, so the dot
//!      belongs to an unknown abbreviation or a spaced run ("p. 42").
//!   4. otherwise -> END_DOT.
//!
//! Known misses: a genuine initial "I." (I. M. Pei), and the a/i exclusion
//! trades that rarity against the far more common pronouns.

use std::ffi::c_void;
use std::os::raw::{c_char, c_uint};
use std::ptr;

type TSSymbol = u16;

/// Mirror of `struct TSLexer` in `src/tree_sitter/parser.h`. Layout must
/// match exactly, including the trailing `log` slot, which is never called.
#[repr(C)]
pub struct TSLexer {
    pub lookahead: i32,
    pub result_symbol: TSSymbol,
    pub advance: Option<unsafe extern "C" fn(*mut TSLexer, skip: bool)>,
    pub mark_end: Option<unsafe extern "C" fn(*mut TSLexer)>,
    pub get_column: Option<unsafe extern "C" fn(*mut TSLexer) -> u32>,
    pub is_at_included_range_start: Option<unsafe extern "C" fn(*const TSLexer) -> bool>,
    pub eof: Option<unsafe extern "C" fn(*const TSLexer) -> bool>,
    pub log: Option<unsafe extern "C" fn(*const TSLexer, *const c_char, ...)>,
}

/// Must match the order of `externals` in `grammar.js`.
#[repr(usize)]
#[derive(Clone, Copy)]
enum TokenType {
    EndDot = 0,
    Period = 1,
    Word = 2,
    Conjunction = 3,
    Subordinator = 4,
    // Appended, never reordered: must match `externals` in grammar.js.
    EllipsisEnd = 5,
    Interruption = 6,
    ColonHandoff = 7,
}

const MAX_WORD: usize = 31;

struct Scanner {
    /// Lowercase text of the last lexed word.
    word: [u8; MAX_WORD + 1],
    len: usize,
}

const CONJUNCTIONS: &[&str] = &["and", "but", "or", "nor", "so", "yet", "for"];

const SUBORDINATORS: &[&str] = &[
    "because",
    "although",
    "though",
    "if",
    "when",
    "while",
    "since",
    "unless",
    "before",
    "after",
    "until",
    "that",
    "which",
    "who",
    "whom",
    "whose",
    "as",
    "once",
    "than",
    "till",
    "whenever",
    "where",
    "whereas",
    "wherever",
    "whether",
    // PDTB audit 2026-09-27: `lest` (7x Moby, pure subordinator, no
    // main-verb use in either corpus) and `supposing` (5x Moby,
    // conditional only; EWT-absent) join. Main-verb `provided`/`given`
    // /`considering` stay out (EWT VERB-dominant); `albeit` has no
    // Moby evidence; `regardless`/`notwithstanding` are
    // prepositional/ADV. See AGENTS.md for the audit table.
    "lest",
    "supposing",
];

const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "st", "jr", "sr", "vs", "inc", "ltd", "co", "fig", "vol", "approx",
    // Abbreviation harvest 2026-09-27 (Moby evidence + EWT check):
    // `rev` (Rev. Henry, 2x, EWT-absent), `mt` (Mt. Hecla, 1x, title
    // pattern like mr/dr/st; EWT has mt as NOUN/PROPN words, which
    // only constrains tags, not dot handling). Rejected: `etc`
    // (genuinely ambiguous mid-list vs sentence-final — needs
    // statistical treatment, not list membership), `Ex` (single odd
    // instance in `U.S. Ex. Ex.`), `albeit` n/a.
    // REMOVED 2026-10-06: `no` — the number use (`No. 22`) is covered
    // by the digit-ahead rule, while the list entry wrongly swallowed
    // interjection `No.` before capitals (Moby 3x: `No. They`, `No.
    // The`, `No. Only`; EWT train has zero mid-sentence `No.`+digit).
    // Differential-oracle find (Punkt split where the grammar joined).
    "rev", "mt",
];

fn is_alpha(c: i32) -> bool {
    matches!(c, 0x41..=0x5A | 0x61..=0x7A)
}

/// ASCII letters plus any other Unicode alphabetic codepoint (æ, œ, é…).
/// Stored words are only ever compared against ASCII-only lists, so
/// non-ASCII bytes in the buffer simply never match a closed-class entry.
fn is_word_char(c: i32) -> bool {
    is_alpha(c)
        || u32::try_from(c)
            .ok()
            .and_then(char::from_u32)
            .is_some_and(|ch| ch.is_alphabetic())
}

fn is_space(c: i32) -> bool {
    matches!(c, 0x20 | 0x09 | 0x0C | 0x0B)
}

fn to_lower(c: i32) -> u8 {
    if (0x41..=0x5A).contains(&c) {
        (c - 0x41 + 0x61) as u8
    } else {
        c as u8
    }
}

fn in_list(word: &[u8], list: &[&str]) -> bool {
    list.iter().any(|entry| entry.as_bytes() == word)
}

/// Current lookahead codepoint. Must be re-read after every advance.
unsafe fn lookahead(lexer: *mut TSLexer) -> i32 {
    unsafe { (*lexer).lookahead }
}

unsafe fn advance(lexer: *mut TSLexer, skip: bool) {
    unsafe {
        (*lexer).advance.expect("TSLexer::advance is null")(lexer, skip);
    }
}

unsafe fn mark_end(lexer: *mut TSLexer) {
    unsafe {
        (*lexer).mark_end.expect("TSLexer::mark_end is null")(lexer);
    }
}

// Consume one newline: \n, \r\n, or a lone \r.
unsafe fn consume_newline(lexer: *mut TSLexer) {
    unsafe {
        if lookahead(lexer) == 0x0A {
            advance(lexer, true);
            return;
        }
        if lookahead(lexer) == 0x0D {
            advance(lexer, true);
            if lookahead(lexer) == 0x0A {
                advance(lexer, true);
            }
        }
    }
}

// The scanner runs before extras are skipped, so it must skip whitespace
// itself. A blank line is the internal `paragraph_break` token: if one is
// forming, refuse, so the internal lexer can match it.
unsafe fn skip_whitespace(lexer: *mut TSLexer, paragraph_break_ahead: &mut bool) {
    unsafe {
        loop {
            if is_space(lookahead(lexer)) {
                advance(lexer, true);
                continue;
            }
            if lookahead(lexer) == 0x0A || lookahead(lexer) == 0x0D {
                consume_newline(lexer);
                // A blank line may contain spaces and tabs.
                while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
                    advance(lexer, true);
                }
                if lookahead(lexer) == 0x0A || lookahead(lexer) == 0x0D {
                    *paragraph_break_ahead = true;
                    return;
                }
                continue; // lone newline: hard-wrapped prose, keep going
            }
            break;
        }
    }
}

// True when only a clause boundary (or input end) follows the current
// position: terminal marks, closing delimiters/quotes, clause punctuation
// (; : em/en-dash, ASCII `--` runs), or EOF. Newlines deliberately do
// not count: a subordinate clause may continue on the next line, and
// only same-line evidence overrules that greedy reading.
unsafe fn end_ahead(lexer: *mut TSLexer) -> bool {
    unsafe {
        // Lookahead only: advance(false) so mark_end stays at the word end.
        // advance(true) here would corrupt the token range (zero-length
        // subordinator) per the external-scanner docs. Input position
        // rewinds to mark_end on success, so this peeking is free.
        while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
            advance(lexer, false);
        }
        // ASCII dash run (`--`, the Gutenberg substitute) counts as
        // boundary, like em/en-dash (`sailed for--treasure` degrades
        // `for` to a plain word). A lone hyphen does not (compounds):
        // return false directly, since only the run was consumed and
        // `-` can never start a boundary match below.
        if lookahead(lexer) == 0x2D {
            advance(lexer, false);
            return lookahead(lexer) == 0x2D;
        }
        matches!(
            lookahead(lexer),
            0x2E | 0x3F | 0x21 | // . ? !
            0x3B | 0x3A | // ; :
            0x2014 | 0x2013 | // em/en-dash
            0x29 | 0x5D | 0x7D | // ) ] }
            0x22 | 0x27 | 0x60 | 0x2018 | 0x2019 | 0x201C | 0x201D // quotes
        ) || (*lexer).eof.expect("TSLexer::eof is null")(lexer)
    }
}

// `…` (U+2026): consume the single char, then apply the terminal
// test. Refusals rewind fully (like dot runs), so the internal
// `ellipsis` rule retries mid-sentence uses.
unsafe fn scan_ellipsis_char(lexer: *mut TSLexer, _valid_symbols: *const bool) -> bool {
    unsafe {
        advance(lexer, false);
        mark_end(lexer);
        if !ellipsis_end_ahead(lexer) {
            return false;
        }
        (*lexer).result_symbol = TokenType::EllipsisEnd as TSSymbol;
        true
    }
}

// `...`: count the run (first dot already consumed). Runs of 4+ always
// terminate, covering the whole run; a run of exactly 3 terminates on
// boundary-ahead and otherwise yields to the internal mid-sentence
// token. Shorter runs are not ellipses (the user: exactly three dots).
// Refusals rewind fully (like the letter-dot-letter refusal), so the
// internal rule retries from the run start.
unsafe fn scan_dot_run(lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    unsafe {
        let mut count = 1;
        while lookahead(lexer) == 0x2E {
            advance(lexer, false);
            count += 1;
        }
        if count < 3 || !valid(valid_symbols, TokenType::EllipsisEnd) {
            return false;
        }
        // Mark the run end before peeking: trailing spaces must not join
        // the token (rewind-to-mark restores them on success, full rewind
        // on refusal).
        mark_end(lexer);
        if count == 3 && !ellipsis_end_ahead(lexer) {
            return false;
        }
        (*lexer).result_symbol = TokenType::EllipsisEnd as TSSymbol;
        true
    }
}

// Boundary evidence for a terminal `...`: uppercase, EOF, terminal
// marks, closers and quotes — but not `; :`, dashes, lowercase or
// openers, which continue the sentence (compare end_ahead, which also
// excludes newlines; a trailing ellipsis may hand off across a break).
unsafe fn ellipsis_end_ahead(lexer: *mut TSLexer) -> bool {
    unsafe {
        while matches!(lookahead(lexer), 0x20 | 0x09 | 0x0A | 0x0D) {
            advance(lexer, false);
        }
        let c = lookahead(lexer);
        char::from_u32(c as u32).is_some_and(|ch| ch.is_uppercase())
            || matches!(
                c,
                0x3F | 0x21 | // ? !
                0x29 | 0x5D | 0x7D | // ) ] }
                0x22 | 0x27 | 0x60 | 0x2018 | 0x2019 | 0x201C | 0x201D // quotes
            )
            || (*lexer).eof.expect("TSLexer::eof is null")(lexer)
    }
}

fn is_dash(c: i32) -> bool {
    // Em/en dash plus figure dash (U+2012) and horizontal bar (U+2015):
    // the destructive-tokenizer range U+2012-U+2015. Words never contain
    // them (the word loop takes ASCII hyphen only), so they always reach
    // dash handling whole.
    matches!(c, 0x2012..=0x2015)
}

fn is_closer(c: i32) -> bool {
    matches!(
        c,
        0x29 | 0x5D | 0x7D | // ) ] }
        0x22 | 0x27 | 0x60 | 0x2018 | 0x2019 | 0x201C | 0x201D // quotes
    )
}

// Outcome of probing for a boundary-handoff token: the scanner
// arbitrates where the grammar cannot — a mark with only a boundary
// after it ends the sentence, while text after it continues the
// join/elaboration.
//
// Lexer-contract notes (see also AGENTS.md):
// - A refusal rewinds fully (like the letter-dot-letter refusal), so
//   probing is free: `Refused` returns false and the internal token
//   matches from the mark start.
// - `advance(true)` excludes chars from the token range (leading
//   whitespace skipping); `advance(false)` includes them. Leading
//   skips below must stay `true`, or an `Absent` fall-through would
//   yield space-prefixed word tokens.
// - The caller must distinguish the refusals: `Refused` (mark seen,
//   text ahead) returns false; `Absent` (no mark seen) falls through
//   to word/conjunction/subordinator lexing (words are external-only —
//   returning false there would make them unlexable).
#[derive(PartialEq, Eq)]
enum Handoff {
    Emitted,
    Refused,
    Absent,
}

// Abandoned clause: an em/en-dash run — or an ASCII double-hyphen
// run, the Gutenberg edition substitute — emitted (one or more,
// wholesale) only on boundary-ahead: a blank line or EOF after
// optional closing quotes/parens, which the token absorbs (`Faith,
// sir, I've——` + blank, `said—"` + blank, trailing run at EOF).
// Anywhere else (clause text ahead, single newline) refuse, so the
// internal em_dash join reading applies. A lone hyphen is neither:
// it returns `Refused` (rewind → today's ERROR path), never falling
// through, so `-` keeps erroring exactly as before.
unsafe fn scan_interruption(lexer: *mut TSLexer) -> Handoff {
    unsafe {
        while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
            advance(lexer, true);
        }
        if is_dash(lookahead(lexer)) {
            while is_dash(lookahead(lexer)) {
                advance(lexer, false);
            }
        } else if lookahead(lexer) == 0x2D {
            let mut count = 0;
            while lookahead(lexer) == 0x2D {
                advance(lexer, false);
                count += 1;
            }
            if count < 2 {
                return Handoff::Refused;
            }
        } else {
            return Handoff::Absent;
        }
        mark_end(lexer);
        while is_closer(lookahead(lexer)) {
            advance(lexer, false);
        }
        mark_end(lexer);
        // Boundary-ahead: blank line, or EOF (after at most one
        // single line break plus spaces — a trailing newline at EOF
        // still hands off to nothing). A single break with text after
        // it refuses: same-line semantics make that a join.
        while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
            advance(lexer, false);
        }
        let c = lookahead(lexer);
        if c == 0x0A || c == 0x0D {
            if c == 0x0D {
                advance(lexer, false);
            }
            if lookahead(lexer) == 0x0A {
                advance(lexer, false);
            }
            while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
                advance(lexer, false);
            }
            let c2 = lookahead(lexer);
            if c2 != 0x0A && c2 != 0x0D && !(*lexer).eof.expect("TSLexer::eof is null")(lexer) {
                return Handoff::Refused;
            }
        } else if !(*lexer).eof.expect("TSLexer::eof is null")(lexer) {
            return Handoff::Refused;
        }
        (*lexer).result_symbol = TokenType::Interruption as TSSymbol;
        Handoff::Emitted
    }
}

// Abandoned elaboration: a colon with only a boundary after it
// (`He said:` + blank, trailing colon at EOF). Same arbitration as
// the interruption: text after the colon refuses (internal colon
// elaborates), so this fires exactly where elaboration is
// impossible. The colon itself is the token (single char, no run,
// no closers — closers after a colon belong to the elaboration).
unsafe fn scan_colon_handoff(lexer: *mut TSLexer) -> Handoff {
    unsafe {
        while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
            advance(lexer, true);
        }
        if lookahead(lexer) != 0x3A {
            return Handoff::Absent;
        }
        advance(lexer, false);
        mark_end(lexer);
        // Digit-guarded colon (treebank `([:,])([^\\d])` shape): a digit
        // after the colon is a time (`10:30`), never a handoff — refuse
        // immediately so times behave exactly as before (currently an
        // error; full time support needs number-token work, residual).
        if lookahead(lexer) >= 0x30 && lookahead(lexer) <= 0x39 {
            return Handoff::Refused;
        }
        while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
            advance(lexer, false);
        }
        let c = lookahead(lexer);
        if c == 0x0A || c == 0x0D {
            if c == 0x0D {
                advance(lexer, false);
            }
            if lookahead(lexer) == 0x0A {
                advance(lexer, false);
            }
            while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
                advance(lexer, false);
            }
            let c2 = lookahead(lexer);
            if c2 != 0x0A && c2 != 0x0D && !(*lexer).eof.expect("TSLexer::eof is null")(lexer) {
                return Handoff::Refused;
            }
        } else if !(*lexer).eof.expect("TSLexer::eof is null")(lexer) {
            return Handoff::Refused;
        }
        (*lexer).result_symbol = TokenType::ColonHandoff as TSSymbol;
        Handoff::Emitted
    }
}

unsafe fn scan_dot(scanner: &Scanner, lexer: *mut TSLexer, valid_symbols: *const bool) -> bool {
    unsafe {
        advance(lexer, false);
        if lookahead(lexer) == 0x2E {
            return scan_dot_run(lexer, valid_symbols);
        }
        mark_end(lexer);

        // Spaced single-letter initial (but never a/i: real words).
        if scanner.len == 1 && scanner.word[0] != b'a' && scanner.word[0] != b'i' {
            (*lexer).result_symbol = TokenType::Period as TSSymbol;
            return true;
        }

        // Known abbreviation.
        if in_list(&scanner.word[..scanner.len], ABBREVIATIONS) {
            (*lexer).result_symbol = TokenType::Period as TSSymbol;
            return true;
        }

        // Skip whitespace, including newlines, to the next content character.
        while matches!(lookahead(lexer), 0x20 | 0x09 | 0x0A | 0x0D) {
            advance(lexer, true);
        }

        let continues = matches!(lookahead(lexer), 0x61..=0x7A | 0x30..=0x39);

        (*lexer).result_symbol = if continues {
            TokenType::Period
        } else {
            TokenType::EndDot
        } as TSSymbol;
        true
    }
}

unsafe fn valid(valid_symbols: *const bool, token: TokenType) -> bool {
    unsafe { *valid_symbols.add(token as usize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_english_external_scanner_create() -> *mut c_void {
    Box::into_raw(Box::new(Scanner {
        word: [0; MAX_WORD + 1],
        len: 0,
    })) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_english_external_scanner_destroy(payload: *mut c_void) {
    if !payload.is_null() {
        unsafe {
            drop(Box::from_raw(payload as *mut Scanner));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_english_external_scanner_serialize(
    payload: *mut c_void,
    buffer: *mut c_char,
) -> c_uint {
    unsafe {
        let scanner = &*(payload as *mut Scanner);
        if scanner.len > MAX_WORD {
            return 0;
        }
        *buffer = scanner.len as c_char;
        ptr::copy_nonoverlapping(
            scanner.word.as_ptr() as *const c_char,
            buffer.add(1),
            scanner.len,
        );
        (scanner.len + 1) as c_uint
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_english_external_scanner_deserialize(
    payload: *mut c_void,
    buffer: *const c_char,
    length: c_uint,
) {
    unsafe {
        let scanner = &mut *(payload as *mut Scanner);
        scanner.len = 0;
        scanner.word[0] = 0;
        if length < 1 {
            return;
        }
        let len = *buffer as u8 as usize;
        if len > MAX_WORD || len + 1 > length as usize {
            return;
        }
        scanner.len = len;
        ptr::copy_nonoverlapping(buffer.add(1) as *const u8, scanner.word.as_mut_ptr(), len);
        scanner.word[len] = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tree_sitter_english_external_scanner_scan(
    payload: *mut c_void,
    lexer: *mut TSLexer,
    valid_symbols: *const bool,
) -> bool {
    unsafe {
        let scanner = &mut *(payload as *mut Scanner);

        if (valid(valid_symbols, TokenType::EndDot) || valid(valid_symbols, TokenType::Period))
            && lookahead(lexer) == 0x2E
        {
            return scan_dot(scanner, lexer, valid_symbols);
        }

        // Single-char ellipsis U+2026: terminal `…` emits EllipsisEnd
        // on boundary-ahead, else refuses (full rewind) so the
        // internal `ellipsis` rule takes it mid-sentence. Mirrors
        // the count == 3 arm of scan_dot_run; the 4+ wholesale rule
        // has no single-char analogue.
        if valid(valid_symbols, TokenType::EllipsisEnd) && lookahead(lexer) == 0x2026 {
            return scan_ellipsis_char(lexer, valid_symbols);
        }

        // Abandoned clause hands off at a dash run: like the dot check
        // above, this sits before the word/conjunction/subordinator
        // early-return, because at a clause boundary none of those are
        // valid while the interruption may be. The gate includes `-`
        // (ASCII runs reach the probe; lone hyphens refuse inside).
        // The outcome decides: Emitted returns true; Refused returns
        // false (the run rewinds, the internal em_dash joins or errors
        // as before); Absent falls through to word lexing below
        // (words are external-only).
        if valid(valid_symbols, TokenType::Interruption)
            && (is_dash(lookahead(lexer))
                || lookahead(lexer) == 0x2D
                || lookahead(lexer) == 0x20
                || lookahead(lexer) == 0x09)
        {
            match scan_interruption(lexer) {
                Handoff::Emitted => return true,
                Handoff::Refused => return false,
                Handoff::Absent => {}
            }
        }

        // Abandoned elaboration hands off at a colon: same placement
        // and outcome discipline as the interruption above. A colon
        // with text after it refuses (internal colon elaborates).
        if valid(valid_symbols, TokenType::ColonHandoff)
            && (lookahead(lexer) == 0x3A || lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09)
        {
            match scan_colon_handoff(lexer) {
                Handoff::Emitted => return true,
                Handoff::Refused => return false,
                Handoff::Absent => {}
            }
        }

        if !valid(valid_symbols, TokenType::Word)
            && !valid(valid_symbols, TokenType::Conjunction)
            && !valid(valid_symbols, TokenType::Subordinator)
        {
            return false;
        }

        // `&` joins like `and` (Enderby & Sons) where conjunctions are
        // valid — the closed-class discipline (emit only where the
        // grammar allows). Spaces and single newlines are skipped
        // with advance(true) so a miss falls through to word lexing
        // below with a clean token start; never `return false` here
        // (words are external-only) — EXCEPT across a blank line,
        // which must survive for the paragraph_break token (refusal
        // rewinds fully). Plain clause interiors disallow
        // conjunctions, so mid-clause `&` (R&D) keeps erroring there
        // as a conjunction; subordinate interiors accept it as a
        // join, which reads fine. Unspaced `&word` (R&D, `&c.`,
        // AT&T — EWT keeps such runs whole as NOUN/PROPN) is NOT a
        // conjunction: decline to word lexing below, which absorbs
        // it (Word-gated, so conjunction-only states are untouched).
        // Word buffer, seeded with `&` on the `&`-lead path below.
        let mut buf = [0u8; MAX_WORD + 1];
        let mut len = 0usize;
        // True when the `&` probe below already consumed an unspaced
        // `&` + letter: entry checks are skipped, the loop starts
        // with `buf = "&"`.
        let mut amp_lead = false;
        if valid(valid_symbols, TokenType::Conjunction) {
            loop {
                while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
                    advance(lexer, true);
                }
                if lookahead(lexer) == 0x0A {
                    advance(lexer, true);
                } else if lookahead(lexer) == 0x0D {
                    advance(lexer, true);
                    if lookahead(lexer) == 0x0A {
                        advance(lexer, true);
                    }
                } else {
                    break;
                }
                while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
                    advance(lexer, true);
                }
                // A second break is a blank line: abort (full rewind
                // restores it for paragraph_break).
                if lookahead(lexer) == 0x0A || lookahead(lexer) == 0x0D {
                    return false;
                }
            }
            if lookahead(lexer) == 0x26 {
                advance(lexer, false);
                // Unspaced `&` + letter is a word (decline, Word-gated).
                if valid(valid_symbols, TokenType::Word) && is_word_char(lookahead(lexer)) {
                    amp_lead = true;
                    buf[0] = b'&';
                    len = 1;
                } else {
                    mark_end(lexer);
                    (*lexer).result_symbol = TokenType::Conjunction as TSSymbol;
                    return true;
                }
            }
        }

        let mut paragraph_break_ahead = false;
        if !amp_lead {
            skip_whitespace(lexer, &mut paragraph_break_ahead);
            if paragraph_break_ahead {
                return false;
            }
            if lookahead(lexer) == 0x26 {
                // `&`-leading word where the probe above declined or
                // was invalid (Word-gated): absorb like the lead.
                if !valid(valid_symbols, TokenType::Word) {
                    return false;
                }
                advance(lexer, false);
                if !is_word_char(lookahead(lexer)) {
                    return false;
                }
                buf[0] = b'&';
                len = 1;
            } else if !is_word_char(lookahead(lexer)) {
                return false;
            }
        }

        // (buf/len/amp_lead/dot_passed/dash_run_passed declared above.)
        // Set when the dot branch below consumes a `.` that turns out not
        // to start a dotted run: the lexer rewinds to the marked end, so
        // the dot is re-lexed on the next scan. It still counts as boundary
        // evidence here (scan_dot will judge it on its own merits).
        let mut dot_passed = false;
        // Set when the hyphen branch below consumes the first `-` of a
        // `--` run: same rewind story (the run re-lexes whole, either as
        // the internal `--+` join dash or via the interruption probe),
        // and the run counts as boundary evidence for trailing
        // closed-class words (`remember that--and` degrades `that`).
        let mut dash_run_passed = false;

        loop {
            if is_alpha(lookahead(lexer)) {
                if len < MAX_WORD {
                    buf[len] = to_lower(lookahead(lexer));
                    len += 1;
                }
                advance(lexer, false);
                mark_end(lexer);
                continue;
            }
            if let Some(ch) = u32::try_from(lookahead(lexer))
                .ok()
                .and_then(char::from_u32)
                .filter(|ch| !ch.is_ascii() && ch.is_alphabetic())
            {
                // Non-ASCII letter (æ, œ, é…): consume as word text,
                // storing raw UTF-8 bytes. ASCII-only lists can never
                // match, so no lowercasing is needed.
                let mut encoded = [0u8; 4];
                let bytes = ch.encode_utf8(&mut encoded);
                if len + bytes.len() <= MAX_WORD {
                    buf[len..len + bytes.len()].copy_from_slice(bytes.as_bytes());
                    len += bytes.len();
                }
                advance(lexer, false);
                mark_end(lexer);
                continue;
            }
            if lookahead(lexer) == 0x27 || lookahead(lexer) == 0x2019 {
                // Apostrophe (ASCII or curly right single quote U+2019)
                // belongs to the word only between letters. Normalized to
                // ASCII for comparison.
                advance(lexer, false);
                if is_word_char(lookahead(lexer)) {
                    if len < MAX_WORD {
                        buf[len] = b'\'';
                        len += 1;
                    }
                    continue;
                }
                // Elided compound (sou'-wester): apostrophe-hyphen-letter
                // all belong to the word. A hyphen-then-junk breaks
                // instead; the rewind-to-mark on success re-lexes from
                // the apostrophe, so nothing is lost.
                if lookahead(lexer) == 0x2D {
                    advance(lexer, false);
                    if is_word_char(lookahead(lexer)) {
                        if len < MAX_WORD {
                            buf[len] = b'\'';
                            len += 1;
                        }
                        if len < MAX_WORD {
                            buf[len] = b'-';
                            len += 1;
                        }
                        continue;
                    }
                }
                break; // trailing apostrophe stays outside the token
            }
            if lookahead(lexer) == 0x2D {
                // Hyphen belongs to the word only between letters, like the
                // apostrophe above (`well-known`, `Mast-Head`). A leading
                // hyphen never reaches this loop (the alpha guard above
                // rejects it); a trailing one stays outside the token.
                // A `--` run is different: it is the ASCII dash handoff /
                // join mark, so flag it (like dot_passed) for the trailing
                // closed-class decision below, then break — emission
                // rewinds to the mark and the run re-lexes whole.
                advance(lexer, false);
                if is_word_char(lookahead(lexer)) {
                    if len < MAX_WORD {
                        buf[len] = b'-';
                        len += 1;
                    }
                    continue;
                }
                // Letter-digit codes (`M-3`, `A-1`): absorb the hyphen
                // and the digit run (EWT keeps CCA-15 whole as PROPN).
                // Digits need no lowercasing; mark per char like above.
                if matches!(lookahead(lexer), 0x30..=0x39) {
                    if len < MAX_WORD {
                        buf[len] = b'-';
                        len += 1;
                    }
                    while matches!(lookahead(lexer), 0x30..=0x39) {
                        if len < MAX_WORD {
                            buf[len] = lookahead(lexer) as u8;
                            len += 1;
                        }
                        advance(lexer, false);
                        mark_end(lexer);
                    }
                    continue;
                }
                if lookahead(lexer) == 0x2D {
                    dash_run_passed = true;
                }
                break;
            }
            if lookahead(lexer) == 0x26 {
                // Ampersand belongs to the word only between letters
                // (`R&D`, `AT&T` — EWT keeps such runs whole as
                // NOUN/PROPN). A leading `&` never reaches this loop
                // (handled at entry/the probe); a trailing or spaced
                // one stays outside the token for the conjunction
                // probe or an error, as before.
                advance(lexer, false);
                if is_word_char(lookahead(lexer)) {
                    if len < MAX_WORD {
                        buf[len] = b'&';
                        len += 1;
                    }
                    continue;
                }
                break;
            }
            if lookahead(lexer) == 0x2E {
                // Refuse letter-dot-letter runs so the internal `dotted`
                // token can absorb them (H.M.S., e.g.). Any other dot is
                // left for scan_dot (rewound and re-lexed after the mark).
                advance(lexer, false);
                if is_alpha(lookahead(lexer)) {
                    return false;
                }
                dot_passed = true;
                break;
            }
            break;
        }

        let closed_conjunction =
            in_list(&buf[..len], CONJUNCTIONS) && valid(valid_symbols, TokenType::Conjunction);
        let closed_subordinator =
            in_list(&buf[..len], SUBORDINATORS) && valid(valid_symbols, TokenType::Subordinator);
        if (closed_conjunction || closed_subordinator)
            && valid(valid_symbols, TokenType::Word)
            && (dot_passed || dash_run_passed || end_ahead(lexer))
        {
            // Trailing closed-class word with only a boundary after it
            // ("Who did that?", "Because.", "remember that—and",
            // "marvellous and—in", "remember that--and"): read it as a
            // plain word instead of opening a clause that has no content.
            // Falls through to the WORD emission below, including the
            // memory update.
        } else {
            if closed_conjunction {
                (*lexer).result_symbol = TokenType::Conjunction as TSSymbol;
                return true;
            }
            if closed_subordinator {
                (*lexer).result_symbol = TokenType::Subordinator as TSSymbol;
                return true;
            }
        }

        // Remember the word for the next period decision.
        scanner.word[..len].copy_from_slice(&buf[..len]);
        scanner.word[len] = 0;
        scanner.len = len;

        (*lexer).result_symbol = TokenType::Word as TSSymbol;
        true
    }
}
