/* External scanner: decides whether a bare period ends the sentence.
 *
 * Only reached for dots that the internal lexer could not absorb:
 * abbrev keywords (Mr., Dr., ...) and dotted initialism runs (H.M.S.,
 * e.g.) never expose their dots.
 *
 * Decision, purely forward-looking (no serialized state):
 *   - dot followed, over whitespace, by a lowercase letter or digit
 *     -> PERIOD: English sentences do not start lowercase, so this dot
 *     belongs to an unknown abbreviation, a spaced initialism run, or a
 *     continuation like "p. 42".
 *   - anything else (uppercase letter, quote, newline-then-uppercase,
 *     EOF) -> END_DOT.
 *
 * Known misses: spaced single-letter initials before a capital
 * ("J. Smith"), and unknown abbreviations at end of line before a
 * capital-starting continuation.
 */

#include "tree_sitter/parser.h"

enum TokenType {
  END_DOT,
  PERIOD,
};

void *tree_sitter_english_external_scanner_create(void) {
  return NULL;
}

void tree_sitter_english_external_scanner_destroy(void *payload) {}

unsigned tree_sitter_english_external_scanner_serialize(void *payload,
                                                        char *buffer) {
  return 0;
}

void tree_sitter_english_external_scanner_deserialize(void *payload,
                                                      const char *buffer,
                                                      unsigned length) {}

bool tree_sitter_english_external_scanner_scan(void *payload, TSLexer *lexer,
                                               const bool *valid_symbols) {
  if (!valid_symbols[END_DOT] && !valid_symbols[PERIOD]) return false;
  if (lexer->lookahead != '.') return false;

  lexer->advance(lexer, false);
  lexer->mark_end(lexer);

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
