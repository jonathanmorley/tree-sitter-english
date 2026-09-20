package tree_sitter_english_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_english "example.invalid/english_ts/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_english.Language())
	if language == nil {
		t.Errorf("Error loading English grammar")
	}
}
