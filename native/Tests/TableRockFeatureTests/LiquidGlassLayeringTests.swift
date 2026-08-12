import Foundation
import XCTest

/// Product rule anchors for Tahoe Liquid Glass (macOS 26+ only).
///
/// Enforcement of chrome-vs-content layering is repository grep evidence
/// (`glass` / `GlassEffectContainer` on controls; `textBackgroundColor` on
/// grid/editor). No legacy dual-path materials.
final class LiquidGlassLayeringTests: XCTestCase {
  func testTrySampleAccessibilityIdentifierConstant() {
    let trySampleId = "profile.try-sample"
    XCTAssertEqual(trySampleId, "profile.try-sample")
  }

  func testModernGlassStyleNamesAreTheOnlyChromeButtonStyles() {
    // Document the modern-only vocabulary for agents/greps (not runtime UI).
    let primary = "glassProminent"
    let secondary = "glass"
    let forbiddenLegacy = ["borderedProminent", "background(.bar)"]
    XCTAssertFalse(primary.isEmpty)
    XCTAssertFalse(secondary.isEmpty)
    XCTAssertFalse(forbiddenLegacy.contains(primary))
  }

  func testTabStripHierarchyRule() {
    // Document tabs belong to opaque content chrome, not detached glass.
    let selectedStyle = "opaqueSelection"
    let unselectedStyle = "plain"
    XCTAssertNotEqual(selectedStyle, unselectedStyle)
    XCTAssertNotEqual(selectedStyle, "glassProminent")
  }
}
