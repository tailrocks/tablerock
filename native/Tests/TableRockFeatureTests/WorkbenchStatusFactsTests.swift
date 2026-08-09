import XCTest

@testable import TableRockFeature

final class WorkbenchStatusFactsTests: XCTestCase {
  func testReadyLineIncludesEngineAndRows() {
    let line = WorkbenchStatusFacts.line(
      operation: "READY",
      engine: "postgresql",
      querySummary: nil,
      queryError: nil,
      cancelOutcome: nil,
      catalogSummary: "12 relations",
      catalogError: nil,
      resultRowCount: 500,
      production: false)
    XCTAssertTrue(line.contains("postgresql"), line)
    XCTAssertTrue(line.contains("500 rows"), line)
    XCTAssertTrue(line.contains("12 relations"), line)
  }

  func testProductionAppendsReviewHint() {
    let line = WorkbenchStatusFacts.line(
      operation: "READY",
      engine: "sqlite",
      querySummary: "ok",
      queryError: nil,
      cancelOutcome: nil,
      catalogSummary: nil,
      catalogError: nil,
      resultRowCount: 3,
      production: true)
    XCTAssertTrue(line.contains("writes need review"), line)
  }

  func testQueryErrorPreferredOverSummary() {
    let line = WorkbenchStatusFacts.line(
      operation: "ERROR",
      engine: "postgresql",
      querySummary: "should not win",
      queryError: "syntax error",
      cancelOutcome: nil,
      catalogSummary: nil,
      catalogError: nil,
      resultRowCount: nil,
      production: false)
    XCTAssertTrue(line.contains("syntax error"), line)
    XCTAssertFalse(line.contains("should not win"), line)
  }
}
