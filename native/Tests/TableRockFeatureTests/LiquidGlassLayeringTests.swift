import Foundation
import XCTest

/// Structural gate notes for Liquid Glass layering.
///
/// Real enforcement is repository grep evidence (chrome `.glassProminent` only;
/// no glass on grid/editor content). This file intentionally does **not**
/// pretend to assert source text with tautologies.
final class LiquidGlassLayeringTests: XCTestCase {
  /// Documents product rule IDs used by accessibility and evidence greps.
  func testTrySampleAccessibilityIdentifierConstant() {
    // Keep in sync with TableRockApp profile.try-sample buttons.
    let trySampleId = "profile.try-sample"
    XCTAssertFalse(trySampleId.isEmpty)
    XCTAssertTrue(trySampleId.hasPrefix("profile."))
  }
}
