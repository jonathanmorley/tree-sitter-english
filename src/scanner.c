/* External scanner: lexes words and decides bare periods.
 *
 * word, conjunction and subordinator are external so the scanner can
 * remember the last word across tokens. State survives incremental
 * re-parse through serialize/deserialize.
 *
 * Word lexing:
 *   - Collect letters (plus one internal apostrophe run, matching the old
 *     /[A-Za-z]+('[A-Za-z]+)?/ shape), lowercased into state.
 *   - Letter followed by '.' followed by a letter: refuse, so the internal
 *     `dotted` token absorbs initialism runs (H.M.S., e.g.) wholesale.
 *   - Closed-class words are emitted as conjunction/subordinator only where
 *     the grammar allows those symbols (valid_symbols check); everywhere
 *     else they lex as plain words ("For home he left."). This replaces
 *     tree-sitter keyword extraction, which external tokens bypass.
 *
 * Period decision, in order:
 *   1. previous word is a single letter other than a/i -> PERIOD (spaced
 *      initial: "J. Smith"). a/i stay splittable: "am I. You".
 *   2. previous word is a known abbreviation (mr, dr, ...) -> PERIOD.
 *   3. dot followed over whitespace by a lowercase letter or digit ->
 *      PERIOD: English sentences do not start lowercase, so the dot
 *      belongs to an unknown abbreviation or a spaced run ("p. 42").
 *   4. otherwise -> END_DOT.
 *
 * Known misses: a genuine initial "I." (I. M. Pei), and the a/i exclusion
 * trades that rarity against the far more common pronouns.
 */

#include "tree_sitter/parser.h"
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#define MAX_WORD 31

enum TokenType {
  END_DOT,
  PERIOD,
  WORD,
  CONJUNCTION,
  SUBORDINATOR,
};

typedef struct {
  char word[MAX_WORD + 1]; // lowercase text of the last lexed word
  uint32_t len;
} Scanner;

static const char *CONJUNCTIONS[] = {"and", "but", "or",  "nor",
                                     "so",  "yet", "for"};

static const char *SUBORDINATORS[] = {
    "because", "although", "though", "if",     "when", "while",  "since",
    "unless",  "before",   "after",  "until",  "that", "which",  "who",
    "whom",    "whose"};

static const char *ABBREVIATIONS[] = {
    "mr", "mrs", "ms", "dr", "st", "jr", "sr", "vs",
    "inc", "ltd", "co", "no", "fig", "vol", "approx"};

static bool is_alpha(int32_t c) {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

static bool is_space(int32_t c) {
  return c == ' ' || c == '\t' || c == '\f' || c == '\v';
}

// Consume one newline: \n, \r\n, or a lone \r.
static void consume_newline(TSLexer *lexer) {
  if (lexer->lookahead == '\n') {
    lexer->advance(lexer, true);
    return;
  }
  if (lexer->lookahead == '\r') {
    lexer->advance(lexer, true);
    if (lexer->lookahead == '\n') lexer->advance(lexer, true);
  }
}

// The scanner runs before extras are skipped, so it must skip whitespace
// itself. A blank line is the internal `paragraph_break` token: if one is
// forming, refuse, so the internal lexer can match it.
static void skip_whitespace(TSLexer *lexer, bool *paragraph_break_ahead) {
  for (;;) {
    if (is_space(lexer->lookahead)) {
      lexer->advance(lexer, true);
      continue;
    }
    if (lexer->lookahead == '\n' || lexer->lookahead == '\r') {
      consume_newline(lexer);
      // A blank line may contain spaces and tabs.
      while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
        lexer->advance(lexer, true);
      }
      if (lexer->lookahead == '\n' || lexer->lookahead == '\r') {
        *paragraph_break_ahead = true;
        return;
      }
      continue; // lone newline: hard-wrapped prose, keep going
    }
    break;
  }
}

static char to_lower(int32_t c) {
  return (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : (char)c;
}

static bool in_list(const char *word, uint32_t len, const char **list,
                    size_t count) {
  for (size_t i = 0; i < count; i++) {
    if (strlen(list[i]) == len && strncmp(word, list[i], len) == 0) {
      return true;
    }
  }
  return false;
}

void *tree_sitter_english_external_scanner_create(void) {
  Scanner *scanner = calloc(1, sizeof(Scanner));
  return scanner;
}

void tree_sitter_english_external_scanner_destroy(void *payload) {
  free(payload);
}

unsigned tree_sitter_english_external_scanner_serialize(void *payload,
                                                        char *buffer) {
  Scanner *scanner = payload;
  if (scanner->len > MAX_WORD) return 0;
  buffer[0] = (char)scanner->len;
  memcpy(buffer + 1, scanner->word, scanner->len);
  return scanner->len + 1;
}

void tree_sitter_english_external_scanner_deserialize(void *payload,
                                                      const char *buffer,
                                                      unsigned length) {
  Scanner *scanner = payload;
  scanner->len = 0;
  scanner->word[0] = '\0';
  if (length < 1) return;
  uint32_t len = (uint8_t)buffer[0];
  if (len > MAX_WORD || len + 1 > length) return;
  scanner->len = len;
  memcpy(scanner->word, buffer + 1, len);
  scanner->word[len] = '\0';
}

static bool scan_dot(Scanner *scanner, TSLexer *lexer) {
  lexer->advance(lexer, false);
  lexer->mark_end(lexer);

  // Spaced single-letter initial (but never a/i: real words).
  if (scanner->len == 1 && scanner->word[0] != 'a' && scanner->word[0] != 'i') {
    lexer->result_symbol = PERIOD;
    return true;
  }

  // Known abbreviation.
  if (in_list(scanner->word, scanner->len, ABBREVIATIONS,
              sizeof(ABBREVIATIONS) / sizeof(ABBREVIATIONS[0]))) {
    lexer->result_symbol = PERIOD;
    return true;
  }

  // Skip whitespace, including newlines, to the next content character.
  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
         lexer->lookahead == '\n' || lexer->lookahead == '\r') {
    lexer->advance(lexer, true);
  }

  bool continues = (lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
                   (lexer->lookahead >= '0' && lexer->lookahead <= '9');

  lexer->result_symbol = continues ? PERIOD : END_DOT;
  return true;
}

bool tree_sitter_english_external_scanner_scan(void *payload, TSLexer *lexer,
                                               const bool *valid_symbols) {
  Scanner *scanner = payload;

  if ((valid_symbols[END_DOT] || valid_symbols[PERIOD]) &&
      lexer->lookahead == '.') {
    return scan_dot(scanner, lexer);
  }

  if (!valid_symbols[WORD] && !valid_symbols[CONJUNCTION] &&
      !valid_symbols[SUBORDINATOR]) {
    return false;
  }

  bool paragraph_break_ahead = false;
  skip_whitespace(lexer, &paragraph_break_ahead);
  if (paragraph_break_ahead) return false;
  if (!is_alpha(lexer->lookahead)) return false;

  char buf[MAX_WORD + 1];
  uint32_t len = 0;

  for (;;) {
    if (is_alpha(lexer->lookahead)) {
      if (len < MAX_WORD) buf[len++] = to_lower(lexer->lookahead);
      lexer->advance(lexer, false);
      lexer->mark_end(lexer);
      continue;
    }
    if (lexer->lookahead == '\'') {
      // Apostrophe belongs to the word only between letters.
      lexer->advance(lexer, false);
      if (is_alpha(lexer->lookahead)) {
        if (len < MAX_WORD) buf[len++] = '\'';
        continue;
      }
      break; // trailing apostrophe stays outside the token
    }
    if (lexer->lookahead == '.') {
      // Refuse letter-dot-letter runs so the internal `dotted` token can
      // absorb them (H.M.S., e.g.). Any other dot is left for scan_dot.
      lexer->advance(lexer, false);
      if (is_alpha(lexer->lookahead)) return false;
      break;
    }
    break;
  }

  buf[len] = '\0';

  if (in_list(buf, len, CONJUNCTIONS,
              sizeof(CONJUNCTIONS) / sizeof(CONJUNCTIONS[0])) &&
      valid_symbols[CONJUNCTION]) {
    lexer->result_symbol = CONJUNCTION;
    return true;
  }
  if (in_list(buf, len, SUBORDINATORS,
              sizeof(SUBORDINATORS) / sizeof(SUBORDINATORS[0])) &&
      valid_symbols[SUBORDINATOR]) {
    lexer->result_symbol = SUBORDINATOR;
    return true;
  }

  // Remember the word for the next period decision.
  scanner->len = len;
  memcpy(scanner->word, buf, len + 1);

  lexer->result_symbol = WORD;
  return true;
}
