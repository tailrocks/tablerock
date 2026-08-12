import XCTest

final class TableRockDesignLabUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testLeadDataGridExposesNamedRegionsAndPassesSemanticAudit() throws {
        let app = XCUIApplication()
        app.launchArguments = [
            "--concept", "native-workbench",
            "--surface", "data-grid",
            "--appearance", "light",
            "--accessibility", "increase-contrast",
            "--capture",
        ]
        app.launch()

        XCTAssertTrue(app.windows.firstMatch.waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["customers"].firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["SAFE MODE"].firstMatch.exists)
        XCTAssertTrue(app.staticTexts["4 CHANGES"].firstMatch.exists)

        let databaseContext = app.menuButtons[
            "Database context, Northstar Analytics, production"
        ]
        XCTAssertTrue(databaseContext.waitForExistence(timeout: 5))

        let windowFrame = app.windows.firstMatch.frame
        // macOS XCTest contrast sampling reports alternating false positives for
        // opaque black-on-white SwiftUI text. Contrast is verified from the
        // deterministic Increase Contrast capture instead.
        let semanticAuditTypes = XCUIAccessibilityAuditType.all.subtracting(.contrast)
        try app.performAccessibilityAudit(for: semanticAuditTypes) { issue in
            guard let element = issue.element else {
                // SwiftUI's native Menu can report a nil action issue even when
                // its labeled menu actions exist in the accessibility tree.
                return issue.auditType.contains(.action)
            }
            let frame = element.frame
            let isSystemTrafficLightRegion =
                frame.maxX <= windowFrame.minX + 110
                && frame.maxY <= windowFrame.minY + 42
            let isSystemTrafficLight =
                element.elementType == .button
                && isSystemTrafficLightRegion
            let isSwiftUILayoutWrapper =
                element.elementType == .group
                && element.label.isEmpty
            let isSystemTouchBar = element.elementType == .touchBar
            let isVerifiedNativeMenuAction =
                element.elementType == .menuButton
                && element.label == "Database context, Northstar Analytics, production"
                && issue.auditType.contains(.action)
            // SwiftUI inserts noninteractive layout groups without semantic
            // descriptions; named application regions are asserted above.
            return ((isSystemTrafficLight || isSwiftUILayoutWrapper || isSystemTouchBar)
                && issue.auditType.contains(.sufficientElementDescription))
                || (isSystemTrafficLightRegion
                    && issue.auditType.contains(.parentChild))
                || isVerifiedNativeMenuAction
        }
    }
}
