import XCTest

@testable import TableRockFeature

final class ColumnHeaderPresentationTests: XCTestCase {
  func testUnsortedColumnKeepsDatabaseName() {
    XCTAssertEqual(
      workbenchColumnHeaderTitle(
        column: "created_at",
        sorts: [WorkbenchBrowseSort(column: "id")]),
      "created_at")
  }

  func testSortedColumnShowsDirectionAndStablePriority() {
    let sorts = [
      WorkbenchBrowseSort(column: "tenant_id", descending: true),
      WorkbenchBrowseSort(column: "created_at"),
    ]

    XCTAssertEqual(
      workbenchColumnHeaderTitle(column: "tenant_id", sorts: sorts),
      "tenant_id ↓ 1")
    XCTAssertEqual(
      workbenchColumnHeaderTitle(column: "created_at", sorts: sorts),
      "created_at ↑ 2")
  }
}
