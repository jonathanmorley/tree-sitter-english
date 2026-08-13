/**
 * Proof-of-concept tree-sitter grammar for English prose.
 *
 * Tier 1: source_file -> paragraph+ ; paragraph break = blank line.
 *         paragraph -> sentence+ ; sentence ends at [.?!] (+ closing quote/paren).
 *         Periods are robust to abbreviations via three layers:
 *           - `abbrev` keyword tokens absorb the trailing dot of common
 *             abbreviations (Mr., Dr., ...) so it is never exposed as a
 *             sentence end.
 *           - the `dotted` token matches initialism runs wholesale,
 *             including the trailing dot (H.M.S., U.S., e.g., p.m.).
 *           - an external scanner decides every remaining bare dot:
 *             a dot followed (over whitespace) by a lowercase letter or
 *             digit continues the sentence; anything else ends it.
 * Tier 2: sentence -> (clause | subordinate_clause) joined by conjunctions.
 *         A clause is a run of words/commas. A subordinate_clause is
 *         introduced by a subordinator and greedily absorbs the remainder
 *         of the sentence, which keeps the grammar deterministic.
 *
 * Tier 3 (subject/verb/object fields) was dropped: adding keyword extraction
 * for nouns/verbs/pronouns + an SVO alternative forced the parser to commit
 * to the SVO reading on non-SVO sentences (ERROR on e.g. "The book that I
 * read was good"), and fields on hidden tokens do not render. See POC_NOTES.md.
 */

// Each keyword is provided in lowercase and sentence-initial (capitalized)
// form so that "Although", "But", "And", etc. are recognised too. They are
// plain string literals so tree-sitter keyword-extracts them ahead of the
// catch-all `word` token.
const ci = words => words.flatMap(w => [w, w[0].toUpperCase() + w.slice(1)]);

const CONJUNCTIONS = ci([
  'and', 'but', 'or', 'nor', 'so', 'yet', 'for',
]);

const SUBORDINATORS = ci([
  'because', 'although', 'though', 'if', 'when', 'while', 'since',
  'unless', 'before', 'after', 'until', 'that', 'which', 'who',
  'whom', 'whose',
]);

// Abbreviations whose trailing dot is part of the token, so it never ends a
// sentence. Exact string literals win over `word` + bare dot by longest
// match. Sentence-often-final abbreviations like "etc." are deliberately NOT
// listed: the scanner's lowercase-continuation heuristic decides those
// instead ("etc. and more" continues, "etc. The" ends).
const ABBREVIATIONS = ci([
  'mr.', 'mrs.', 'ms.', 'dr.', 'st.', 'jr.', 'sr.', 'vs.',
  'inc.', 'ltd.', 'co.', 'no.', 'fig.', 'vol.', 'approx.',
]);

module.exports = grammar({
  name: 'english',

  // A single newline is whitespace (extra) so it is absorbed inside a sentence.
  // A blank line lexes as the longer `paragraph_break` token instead
  // (longest-match between extra `\n` and token `\n[ \t]*\n`).
  extras: $ => [/\s/],

  externals: $ => [
    $._end_dot, // a period that ends the sentence (scanner-decided, hidden)
    $.period,   // a period inside the sentence (scanner-decided)
  ],

  rules: {
    // NOTE: the FIRST rule in `rules` is the entry (start) rule.
    // A blank line (`paragraph_break`) is a SEPARATOR at the source_file
    // level. A paragraph is just sentence+, so the parser never ends a
    // paragraph except at a blank line / EOF: it cannot split sentences
    // that share a line into separate paragraphs.
    source_file: $ => seq(
      repeat($.paragraph_break),
      $.paragraph,
      repeat(seq(repeat1($.paragraph_break), $.paragraph)),
      repeat($.paragraph_break)
    ),

    paragraph: $ => repeat1($.sentence),

    sentence: $ => seq(
      choice($.clause, $.subordinate_clause),
      repeat(choice(
        seq($.conjunction, choice($.clause, $.subordinate_clause)),
        $.subordinate_clause
      )),
      $._sentence_end
    ),

    // The hidden external `_end_dot` token is a period the scanner judged
    // terminal. `?` and `!` are unambiguous and stay internal. Trailing
    // closing quotes/parens belong to the sentence end.
    _sentence_end: $ => seq(
      choice($._end_dot, /[?!]/),
      repeat(choice(/["'\u2019\u201D\u201C]/, /[)\]}]/))
    ),

    clause: $ => prec.left(repeat1(
      choice($._wordish, $.period, $._comma)
    )),

    subordinate_clause: $ => seq(
      $.subordinator,
      prec.left(repeat1(choice(
        $._wordish, $.period, $._comma, $.conjunction, $.subordinate_clause
      )))
    ),

    conjunction: $ => choice(...CONJUNCTIONS),

    subordinator: $ => choice(...SUBORDINATORS),

    paragraph_break: $ => /\n[ \t]*\n/,

    _comma: $ => ',',

    // Word-class tokens. `abbrev` and `dotted` absorb their dots so those
    // dots never reach the scanner.
    _wordish: $ => choice($.word, $.abbrev, $.dotted, $.number),

    abbrev: $ => choice(...ABBREVIATIONS),

    // Initialism/acronym runs with at least one internal period, plus an
    // optional trailing period: H.M.S. U.S. e.g. i.e. p.m. J.R.R.
    dotted: $ => /[A-Za-z]+(\.[A-Za-z]+)+\.?/,

    // Integers and decimals: the internal dot of 3.14 stays inside the token.
    number: $ => /\d+(\.\d+)?/,

    word: $ => /[A-Za-z]+('[A-Za-z]+)?/,
  },
});
