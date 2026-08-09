import XCTest

@testable import TableRockFeature

final class ProfilePresentationTests: XCTestCase {
  func testEngineBadgeCodes() {
    XCTAssertEqual(ProfileEngineBadge.code("postgresql"), "PG")
    XCTAssertEqual(ProfileEngineBadge.code("ClickHouse"), "CH")
    XCTAssertEqual(ProfileEngineBadge.code("redis"), "RD")
    XCTAssertEqual(ProfileEngineBadge.code("sqlite"), "SQ")
    XCTAssertEqual(ProfileEngineBadge.accessibilityName("postgresql"), "PostgreSQL")
  }

  func testLiveStateSplitsWordAndDetail() {
    let parts = ProfileLiveStatePresentation.parts(from: "Healthy · 12 ms")
    XCTAssertEqual(parts.word, "HEALTHY")
    XCTAssertEqual(parts.detail, "12 ms")
    XCTAssertEqual(
      ProfileLiveStatePresentation.line(from: "Disconnected"),
      "DISCONNECTED")
  }

  func testConnectingWordIsUppercased() {
    let parts = ProfileLiveStatePresentation.parts(from: "Connecting")
    XCTAssertEqual(parts.word, "CONNECTING")
    XCTAssertNil(parts.detail)
  }
}
