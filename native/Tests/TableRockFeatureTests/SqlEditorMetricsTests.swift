import Foundation
import XCTest

@testable import TableRockFeature

final class SqlEditorMetricsTests: XCTestCase {
  func testCaretOnFirstLine() {
    let text = "SELECT 1;"
    let c = SqlEditorMetrics.caret(text: text, selection: NSRange(location: 0, length: 0))
    XCTAssertEqual(c.line, 1)
    XCTAssertEqual(c.column, 1)
    XCTAssertEqual(c.lineCount, 1)
  }

  func testCaretSecondLine() {
    let text = "SELECT 1;\nSELECT 2;"
    let c = SqlEditorMetrics.caret(text: text, selection: NSRange(location: 10, length: 0))
    XCTAssertEqual(c.line, 2)
    XCTAssertEqual(c.column, 1)
    XCTAssertEqual(c.lineCount, 2)
  }

  func testSelectionSummary() {
    XCTAssertNil(SqlEditorMetrics.selectionSummary(selection: NSRange(location: 0, length: 0)))
    XCTAssertEqual(
      SqlEditorMetrics.selectionSummary(selection: NSRange(location: 2, length: 5)),
      "sel 5")
  }

  func testStatusChipIncludesRunning() {
    let chip = SqlEditorMetrics.statusChip(
      text: "SELECT 1;",
      selection: NSRange(location: 0, length: 0),
      isRunning: true,
      hasError: false)
    XCTAssertTrue(chip.contains("L1"), chip)
    XCTAssertTrue(chip.contains("RUNNING"), chip)
  }

  func testEmptyBufferIsLineOne() {
    let c = SqlEditorMetrics.caret(text: "", selection: NSRange(location: 0, length: 0))
    XCTAssertEqual(c.line, 1)
    XCTAssertEqual(c.column, 1)
    XCTAssertEqual(c.lineCount, 1)
  }
}
