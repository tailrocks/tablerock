import Foundation
import XCTest

@testable import TableRockFeature

final class GridCellPresentationTests: XCTestCase {
  func testNullProjectsGlyphAndAccessibility() {
    let cell = WorkbenchCell(
      display: "", kind: 0, truncation: 0, originalByteCount: nil, bytes: Data())
    let p = GridCellPresentation.project(cell)
    XCTAssertEqual(p.title, "∅")
    XCTAssertEqual(p.accessibilityValue, "NULL")
    XCTAssertTrue(p.isNull)
    XCTAssertFalse(p.isNumeric)
  }

  func testEmptyTextProjectsDotGlyph() {
    let cell = WorkbenchCell(
      display: "", kind: 7, truncation: 0, originalByteCount: nil, bytes: Data())
    let p = GridCellPresentation.project(cell)
    XCTAssertEqual(p.title, "·")
    XCTAssertEqual(p.accessibilityValue, "Empty text")
  }

  func testBinaryProjectsByteCountWithGlyph() {
    let cell = WorkbenchCell(
      display: "", kind: 9, truncation: 0, originalByteCount: nil,
      bytes: Data([0x01, 0x02, 0x03]))
    let p = GridCellPresentation.project(cell)
    XCTAssertTrue(p.title.contains("⟨b 3⟩"), p.title)
    XCTAssertTrue(p.accessibilityValue.contains("Binary"))
  }

  func testTruncatedTextPrefixesEllipsis() {
    let cell = WorkbenchCell(
      display: "long", kind: 7, truncation: 1, originalByteCount: 99, bytes: Data("long".utf8))
    let p = GridCellPresentation.project(cell)
    XCTAssertTrue(p.title.hasPrefix("…"), p.title)
    XCTAssertTrue(p.isTruncated)
    XCTAssertTrue(p.accessibilityValue.contains("Truncated"))
  }

  func testIntegerIsNumericRightAlignCandidate() {
    let cell = WorkbenchCell(
      display: "42", kind: 2, truncation: 0, originalByteCount: nil, bytes: Data("42".utf8))
    let p = GridCellPresentation.project(cell)
    XCTAssertEqual(p.title, "42")
    XCTAssertTrue(p.isNumeric)
    XCTAssertTrue(p.statusFact.contains("numeric"))
  }

  func testStructuredEmptyUsesBracesGlyph() {
    let cell = WorkbenchCell(
      display: "", kind: 8, truncation: 0, originalByteCount: nil, bytes: Data())
    let p = GridCellPresentation.project(cell)
    XCTAssertEqual(p.title, "{}")
  }
}
