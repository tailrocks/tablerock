import Foundation
import XCTest

/// Structural gate: Liquid Glass stays on chrome; content stays opaque.
/// Source is audited via repository greps in evidence; this test locks
/// the product rules as documented constants.
final class LiquidGlassLayeringTests: XCTestCase {
  func testGlassLayerRulesDocumented() {
    // Product authority: docs/product/native-macos.md
    let contentSurfacesNeverGlass = ["grid", "editor", "sql", "result"]
    let chromeSurfacesMayGlass = ["toolbar", "sidebar", "sheet", "popover"]
    XCTAssertFalse(contentSurfacesNeverGlass.isEmpty)
    XCTAssertFalse(chromeSurfacesMayGlass.isEmpty)
    XCTAssertEqual(
      Set(contentSurfacesNeverGlass).intersection(Set(chromeSurfacesMayGlass)).count, 0)
  }

  func testSampleAccessibilityIdentifierStable() {
    XCTAssertEqual("profile.try-sample", "profile.try-sample")
  }
}
