; Highlight queries for tree-sitter-english (used by the Pages demo
; via web-tree-sitter; tested in `wasm/README.md`, not the corpus
; runner — captures are presentation, never semantics).
;
; Capture names follow the standard theme convention so any
; tree-sitter theme applies; the demo maps them to its own palette.

(conjunction) @keyword

(subordinator) @keyword

(semicolon) @punctuation.delimiter

(colon) @punctuation.delimiter

(em_dash) @punctuation.delimiter

(number) @number

(currency) @number

(quote) @string

(ellipsis) @punctuation.special

(period) @punctuation

; Words and dotted initialisms take the default face (uncaptured).
