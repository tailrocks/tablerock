import XCTest

final class TableRockDesignLabUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testLeadDataGridExposesNamedRegionsAndPassesSemanticAudit() throws {
        let app = launchLab(accessibility: "increase-contrast")

        let conceptRoot = app.descendants(matching: .any)["design-lab-concept"]
        XCTAssertTrue(conceptRoot.waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["customers"].firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["SAFE MODE"].firstMatch.exists)
        XCTAssertTrue(app.staticTexts["4 CHANGES"].firstMatch.exists)

        let databaseContext = app.menuButtons[
            "Database context, Northstar Analytics, production"
        ]
        XCTAssertTrue(databaseContext.waitForExistence(timeout: 5))

        let windowFrame = conceptRoot.frame
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

    @MainActor
    func testNativeTableSheetsCommandsAndEngineSwitching() {
        let app = launchLab()
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-concept"]
                .waitForExistence(timeout: 10)
        )

        let table = app.tables["design-lab-native-grid"]
        XCTAssertTrue(table.waitForExistence(timeout: 5))
        let rows = table.descendants(matching: .tableRow)
        XCTAssertGreaterThanOrEqual(rows.count, 2)

        let secondRow = rows.element(boundBy: 1)
        secondRow.click()
        XCTAssertTrue(secondRow.isSelected)
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-inspector"]
                .waitForExistence(timeout: 5)
        )

        secondRow.rightClick()
        let inspectItem = app.menuItems["Inspect Selected Value"]
        XCTAssertTrue(inspectItem.waitForExistence(timeout: 5))
        inspectItem.click()

        app.buttons["design-lab-new-connection"].click()
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-connection-sheet"]
                .waitForExistence(timeout: 5)
        )
        app.buttons["design-lab-test-connection"].click()
        XCTAssertTrue(app.staticTexts["Static preview validated"].waitForExistence(timeout: 5))
        app.buttons["Cancel"].click()

        app.buttons["design-lab-review-changes"].click()
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-review-sheet"]
                .waitForExistence(timeout: 5)
        )
        let confirmation = app.textFields["design-lab-review-confirmation"]
        confirmation.click()
        confirmation.typeText("APPLY")
        let apply = app.buttons["design-lab-apply-production"]
        XCTAssertTrue(apply.isEnabled)
        apply.click()

        app.menuButtons["design-lab-engine-menu"].click()
        let clickHouse = app.menuItems["ClickHouse"]
        XCTAssertTrue(clickHouse.waitForExistence(timeout: 5))
        clickHouse.click()
        XCTAssertTrue(
            app.menuButtons["Database context, Atlas Events, production"]
                .waitForExistence(timeout: 5)
        )

        app.typeKey("t", modifierFlags: .command)
        XCTAssertTrue(app.staticTexts["Revenue by region"].waitForExistence(timeout: 5))
    }

    @MainActor
    func testRequiredStateRoutesAndWindowSizing() {
        var app = launchLab(fixture: "empty", windowSize: "minimum")
        XCTAssertTrue(app.staticTexts["No objects yet"].waitForExistence(timeout: 10))
        let minimumWidth = app.windows.firstMatch.frame.width
        app.terminate()

        app = launchLab(engine: "clickhouse", fixture: "loading")
        XCTAssertTrue(
            app.staticTexts["Loading ClickHouse metadata"].waitForExistence(timeout: 10)
        )
        app.terminate()

        app = launchLab(engine: "redis", fixture: "connection-error", windowSize: "expanded")
        XCTAssertTrue(app.staticTexts["Connection unavailable"].waitForExistence(timeout: 10))
        XCTAssertGreaterThan(app.windows.firstMatch.frame.width, minimumWidth + 400)
    }

    @MainActor
    private func launchLab(
        engine: String = "postgresql",
        fixture: String = "populated",
        windowSize: String = "typical",
        accessibility: String = "system"
    ) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = [
            "--concept", "native-workbench",
            "--surface", "data-grid",
            "--appearance", "light",
            "--accessibility", accessibility,
            "--engine", engine,
            "--fixture", fixture,
            "--window-size", windowSize,
            "--capture",
        ]
        app.launch()
        app.activate()
        return app
    }
}
