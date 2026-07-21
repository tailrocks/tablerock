import Foundation
import Testing
@testable import TableRockFeature

@Test func structuredTreeIsDeterministicAndTyped() throws {
  let rows = try StructuredValueTree.decode(Data(#"{"z":[true,null],"a":1}"#.utf8))
  #expect(rows.map(\.label) == ["root", "a", "z", "[0]", "[1]"])
  #expect(rows.map(\.value) == ["Object (2)", "1", "Array (2)", "true", "NULL"])
  #expect(rows.map(\.depth) == [0, 1, 1, 2, 2])
}

@Test func structuredTreeFailsClosedAtInputNodeAndDepthBounds() throws {
  #expect(throws: StructuredValueTreeError.inputLimitExceeded(actual: 3, limit: 2)) {
    try StructuredValueTree.decode(Data("123".utf8), maxInputBytes: 2)
  }
  #expect(throws: StructuredValueTreeError.nodeLimitExceeded(2)) {
    try StructuredValueTree.decode(Data("[1,2]".utf8), maxNodes: 2)
  }
  #expect(throws: StructuredValueTreeError.depthLimitExceeded(0)) {
    try StructuredValueTree.decode(Data("[1]".utf8), maxDepth: 0)
  }
  #expect(throws: StructuredValueTreeError.invalidJSON) {
    try StructuredValueTree.decode(Data("not-json".utf8))
  }
}
