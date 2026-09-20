import XCTest
import SwiftTreeSitter
import TreeSitterEnglish

final class TreeSitterEnglishTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_english())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading English grammar")
    }
}
