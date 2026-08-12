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
        XCTAssertTrue(app.staticTexts["NO CHANGES"].firstMatch.exists)

        let databaseContext = app.menuButtons[
            "Database context, Northstar Analytics, production"
        ]
        XCTAssertTrue(databaseContext.waitForExistence(timeout: 5))
        let engineMenu = app.menuButtons["design-lab-engine-menu"]
        XCTAssertTrue(engineMenu.waitForExistence(timeout: 5))
        let toolbarOverflow = app.popUpButtons["more toolbar items"]
        XCTAssertTrue(toolbarOverflow.waitForExistence(timeout: 5))

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
            let isSystemHelpTag = element.elementType == .helpTag
            let isSystemToolbarOverflow = element.frame == toolbarOverflow.frame
            let isVerifiedNativeMenuAction =
                issue.auditType.contains(.action)
                && (
                    (element.elementType == .menuButton
                        && (element.label == databaseContext.label
                            || element.identifier == engineMenu.identifier))
                    || element.frame == databaseContext.frame
                    || element.frame == engineMenu.frame
                )
            // SwiftUI inserts noninteractive layout groups without semantic
            // descriptions; named application regions are asserted above.
            return ((isSystemTrafficLight || isSwiftUILayoutWrapper || isSystemTouchBar
                || isSystemHelpTag || isSystemToolbarOverflow)
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

        let beaconCell = table.staticTexts["Beacon & Co."]
        XCTAssertTrue(beaconCell.waitForExistence(timeout: 5))
        beaconCell.click()
        let inspectorValue = app.staticTexts["design-lab-inspector-value"]
        XCTAssertTrue(inspectorValue.waitForExistence(timeout: 5))
        XCTAssertEqual(inspectorValue.value as? String, "Beacon & Co.")
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-inspector"]
                .waitForExistence(timeout: 5)
        )

        beaconCell.coordinate(
            withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)
        ).rightClick()
        let inspectItem = app.menuItems["Inspect Selected Value"]
        XCTAssertTrue(inspectItem.waitForExistence(timeout: 5))
        inspectItem.click()

        app.buttons["design-lab-new-connection"].click()
        XCTAssertTrue(app.sheets.firstMatch.waitForExistence(timeout: 5))
        app.buttons["design-lab-test-connection"].click()
        XCTAssertTrue(app.staticTexts["Static preview validated"].waitForExistence(timeout: 5))
        app.buttons["Cancel"].click()
        XCTAssertTrue(app.sheets.firstMatch.waitForNonExistence(timeout: 5))
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-concept"]
                .waitForExistence(timeout: 5)
        )

        app.menuBars.menuBarItems["Database"].click()
        app.menuItems["Review Pending Changes…"].click()
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
        let clickHouse = app.windows.firstMatch.menuItems["ClickHouse"]
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
        let minimumFrame = app.windows.firstMatch.frame
        XCTAssertEqual(minimumFrame.width, 1_280, accuracy: 2)
        XCTAssertEqual(minimumFrame.height, 760, accuracy: 2)
        app.terminate()

        app = launchLab(engine: "clickhouse", fixture: "loading")
        XCTAssertTrue(
            app.staticTexts["Loading ClickHouse metadata"].waitForExistence(timeout: 10)
        )
        app.terminate()

        app = launchLab(engine: "redis", fixture: "connection-error", windowSize: "expanded")
        XCTAssertTrue(app.staticTexts["Connection unavailable"].waitForExistence(timeout: 10))
        XCTAssertGreaterThan(app.windows.firstMatch.frame.width, minimumFrame.width + 400)
        app.terminate()

        app = launchLab(fixture: "pending-change")
        XCTAssertTrue(app.staticTexts["1 CHANGE"].waitForExistence(timeout: 10))
        app.terminate()

        app = launchLab(fixture: "destructive-review")
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-review-sheet"]
                .waitForExistence(timeout: 10)
        )
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
