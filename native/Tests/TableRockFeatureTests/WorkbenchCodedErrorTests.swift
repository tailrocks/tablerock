import XCTest

@testable import TableRockFeature

final class WorkbenchCodedErrorTests: XCTestCase {
  private struct CodedFailure: WorkbenchCodedError {
    let workbenchCode: String?
  }

  private struct UncodedFailure: Error {}

  func testProjectsOnlyStableBackendCodes() {
    XCTAssertEqual(
      workbenchErrorCode(CodedFailure(workbenchCode: "sql-file-external-change")),
      "sql-file-external-change"
    )
    XCTAssertNil(workbenchErrorCode(UncodedFailure()))
  }
}
