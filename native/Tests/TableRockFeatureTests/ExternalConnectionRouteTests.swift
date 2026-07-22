import Foundation
import XCTest
@testable import TableRockFeature

final class ExternalConnectionRouteTests: XCTestCase {
  func testExtractsSinglePercentDecodedDatabaseUrl() throws {
    let url = try XCTUnwrap(
      URL(
        string:
          "tablerock://open?url=postgresql%3A%2F%2Fu%3Ap%40db.example%3A5432%2Fapp"))
    XCTAssertEqual(
      try externalConnectionUrlPayload(url),
      "postgresql://u:p@db.example:5432/app")
  }

  func testRejectsNonCanonicalRoutesAndDuplicatePayloads() throws {
    let inputs = [
      "file://open?url=postgresql%3A%2F%2Fdb%2Fapp",
      "tablerock://other?url=postgresql%3A%2F%2Fdb%2Fapp",
      "tablerock://open/path?url=postgresql%3A%2F%2Fdb%2Fapp",
      "tablerock://open?url=postgresql%3A%2F%2Fdb%2Fapp&url=redis%3A%2F%2Fdb",
      "tablerock://open?url=postgresql%3A%2F%2Fdb%2Fapp&extra=1",
      "tablerock://open?url=",
      "tablerock://open?url=postgresql%3A%2F%2Fdb%2Fapp#fragment",
    ]
    for input in inputs {
      let url = try XCTUnwrap(URL(string: input))
      XCTAssertThrowsError(try externalConnectionUrlPayload(url), input)
    }
  }

  func testRejectsOversizedEnvelopeBeforeParsing() throws {
    let url = try XCTUnwrap(URL(string: "tablerock://open?url=\(String(repeating: "a", count: 40))"))
    XCTAssertThrowsError(try externalConnectionUrlPayload(url, maximumBytes: 32)) { error in
      XCTAssertEqual(
        error as? ExternalConnectionRouteError,
        .tooLarge(actual: url.absoluteString.utf8.count, maximum: 32))
    }
  }
}
