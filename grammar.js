/**
 * Proof-of-concept tree-sitter grammar for English prose.
 *
 * Tier 1: source_file -> paragraph+ ; paragraph break = blank line.
 *         paragraph -> sentence+ ; sentence ends at [.?!] (+ closing quote/paren).
 *         Periods are robust to abbreviations. Word, conjunction and
 *         subordinator are external tokens lexed by src/scanner.c, which
 *         remembers the last word and decides every bare dot:
 *           - previous word is a single letter other than a/i (a spaced
 *             initial like "J.") -> the dot continues the sentence;
 *           - previous word is a known abbreviation (mr, dr, ...) -> the
 *             dot continues the sentence;
 *           - otherwise a dot followed over whitespace by a lowercase
 *             letter or digit continues the sentence; anything else ends it.
 *         The `dotted` token matches initialism runs wholesale, including
 *         the trailing dot (H.M.S., U.S., e.g.). The scanner refuses such
 *         runs so `dotted` can absorb them.
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

export default grammar({
  name: 'english',

  // A single newline is whitespace (extra) so it is absorbed inside a sentence.
  // A blank line lexes as the longer `paragraph_break` token instead
  // (longest-match between extra `\n` and token `\n[ \t]*\n`).
  extras: $ => [/\s/],

  // Order defines the scanner's enum. `word`, `conjunction` and
  // `subordinator` are external so the scanner can track the last word;
  // the scanner emits each one only where the grammar allows it, which
  // replaces internal keyword extraction.
  externals: $ => [
    $._end_dot,      // a period that ends the sentence (hidden)
    $.period,        // a period inside the sentence
    $.word,          // any open-class word run
    $.conjunction,   // and but or nor so yet for
    $.subordinator,  // because although that which who ...
  ],

  rules: {
    // NOTE: the FIRST rule in `rules` is the entry (start) rule.
    // A blank line (`paragraph_break`) is a SEPARATOR at the source_file
    // level. A paragraph is just sentence+, so the parser never ends a
    // paragraph except at a blank line / EOF: it cannot split sentences
    // that share a line into separate paragraphs.
    // The paragraph core is optional so empty input parses as a bare
    // source_file instead of an error.
    source_file: $ => seq(
      repeat($.paragraph_break),
      optional(seq(
        $.paragraph,
        repeat(seq(repeat1($.paragraph_break), $.paragraph)),
        repeat($.paragraph_break),
      )),
    ),

    paragraph: $ => repeat1($.sentence),

    sentence: $ => seq(
      choice($.clause, $.subordinate_clause),
      repeat(choice(
        seq($.conjunction, choice($.clause, $.subordinate_clause)),
        seq($.semicolon, choice($.clause, $.subordinate_clause)),
        seq($.colon, choice($.clause, $.subordinate_clause)),
        seq($.em_dash, choice($.clause, $.subordinate_clause)),
        $.subordinate_clause
      )),
      $._sentence_end
    ),

    // The hidden external `_end_dot` token is a period the scanner judged
    // terminal. `?` and `!` are unambiguous and stay internal. Trailing
    // closing quotes/parens belong to the sentence end. These terminal
    // quotes use inline regex (not $.quote) so they stay hidden and do not
    // conflict with visible quote tokens that start a new clause.
    _sentence_end: $ => seq(
      choice($._end_dot, /[?!]/),
      repeat(choice(/["'\u2018\u2019\u201D\u201C]/, /[)\]}]/))
    ),

    clause: $ => prec.left(repeat1(
      choice($._wordish, $.period, $._comma, $.quote)
    )),

    subordinate_clause: $ => seq(
      $.subordinator,
      prec.left(repeat1(choice(
        $._wordish, $.period, $._comma, $.conjunction,
        $.subordinate_clause, $.quote
      )))
    ),

    paragraph_break: $ => /\r?\n[ \t]*\r?\n/,

    _comma: $ => ',',

    // Clause-joining punctuation. Semicolons conjoin coordinate clauses.
    // Colons and em-dashes introduce elaborating material. En dash (U+2013)
    // is included as an em-dash substitute.
    semicolon: $ => ';',
    colon: $ => ':',
    em_dash: $ => /—|–/,

    // Quote marks, visible inside clauses so they are queryable.
    quote: $ => /["'\u2018\u2019\u201C\u201D]/,

    // Word-class tokens. `dotted` absorbs initialism runs so their dots
    // never reach the scanner; `number` does the same for decimals.
    _wordish: $ => choice($.word, $.dotted, $.number),

    // Initialism/acronym runs with at least one internal period, plus an
    // optional trailing period: H.M.S. U.S. e.g. i.e. p.m. J.R.R.
    // The scanner refuses letter-dot-letter sequences so this token wins.
    dotted: $ => /[A-Za-z]+(\.[A-Za-z]+)+\.?/,

    // Integers and decimals: the internal dot of 3.14 stays inside the token.
    number: $ => /\d+(\.\d+)?/,
  },
});
