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
//!     apostrophe and normalized.
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
}

const MAX_WORD: usize = 31;

struct Scanner {
    /// Lowercase text of the last lexed word.
    word: [u8; MAX_WORD + 1],
    len: usize,
}

const CONJUNCTIONS: &[&str] = &["and", "but", "or", "nor", "so", "yet", "for"];

const SUBORDINATORS: &[&str] = &[
    "because", "although", "though", "if", "when", "while", "since", "unless", "before", "after",
    "until", "that", "which", "who", "whom", "whose", "as", "once", "than", "till", "whenever",
    "where", "whereas", "wherever", "whether",
];

const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "st", "jr", "sr", "vs", "inc", "ltd", "co", "no", "fig", "vol",
    "approx",
];

fn is_alpha(c: i32) -> bool {
    matches!(c, 0x41..=0x5A | 0x61..=0x7A)
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

// True when only a sentence boundary (or input end) follows the current
// position: terminal marks, closing delimiters/quotes, or EOF. Newlines
// deliberately do not count: a subordinate clause may continue on the
// next line, and only same-line evidence overrules that greedy reading.
unsafe fn end_ahead(lexer: *mut TSLexer) -> bool {
    unsafe {
        // Lookahead only: advance(false) so mark_end stays at the word end.
        // advance(true) here would corrupt the token range (zero-length
        // subordinator) per the external-scanner docs. Input position
        // rewinds to mark_end on success, so this peeking is free.
        while lookahead(lexer) == 0x20 || lookahead(lexer) == 0x09 {
            advance(lexer, false);
        }
        matches!(
            lookahead(lexer),
            0x2E | 0x3F | 0x21 | // . ? !
            0x29 | 0x5D | 0x7D | // ) ] }
            0x22 | 0x27 | 0x2018 | 0x2019 | 0x201C | 0x201D // quotes
        ) || (*lexer).eof.expect("TSLexer::eof is null")(lexer)
    }
}

unsafe fn scan_dot(scanner: &Scanner, lexer: *mut TSLexer) -> bool {
    unsafe {
        advance(lexer, false);
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
            return scan_dot(scanner, lexer);
        }

        if !valid(valid_symbols, TokenType::Word)
            && !valid(valid_symbols, TokenType::Conjunction)
            && !valid(valid_symbols, TokenType::Subordinator)
        {
            return false;
        }

        let mut paragraph_break_ahead = false;
        skip_whitespace(lexer, &mut paragraph_break_ahead);
        if paragraph_break_ahead {
            return false;
        }
        if !is_alpha(lookahead(lexer)) {
            return false;
        }

        let mut buf = [0u8; MAX_WORD + 1];
        let mut len = 0usize;
        // Set when the dot branch below consumes a `.` that turns out not
        // to start a dotted run: the lexer rewinds to the marked end, so
        // the dot is re-lexed on the next scan. It still counts as boundary
        // evidence here (scan_dot will judge it on its own merits).
        let mut dot_passed = false;

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
            if lookahead(lexer) == 0x27 || lookahead(lexer) == 0x2019 {
                // Apostrophe (ASCII or curly right single quote U+2019)
                // belongs to the word only between letters. Normalized to
                // ASCII for comparison.
                advance(lexer, false);
                if is_alpha(lookahead(lexer)) {
                    if len < MAX_WORD {
                        buf[len] = b'\'';
                        len += 1;
                    }
                    continue;
                }
                break; // trailing apostrophe stays outside the token
            }
            if lookahead(lexer) == 0x2D {
                // Hyphen belongs to the word only between letters, like the
                // apostrophe above (`well-known`, `Mast-Head`). A leading
                // hyphen never reaches this loop (the alpha guard above
                // rejects it); a trailing one stays outside the token.
                advance(lexer, false);
                if is_alpha(lookahead(lexer)) {
                    if len < MAX_WORD {
                        buf[len] = b'-';
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
            && (dot_passed || end_ahead(lexer))
        {
            // Trailing closed-class word with nothing after it ("Who did
            // that?", "Because."): read it as a plain word instead of
            // opening a clause that has no content. Falls through to the
            // WORD emission below, including the memory update.
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
