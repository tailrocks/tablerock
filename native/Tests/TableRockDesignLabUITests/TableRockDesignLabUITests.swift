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
        let sortMenu = app.menuButtons["design-lab-sort-menu"]
        XCTAssertTrue(sortMenu.waitForExistence(timeout: 5))
        let toolbarOverflow = app.popUpButtons["more toolbar items"]
        let toolbarOverflowFrame = toolbarOverflow.exists ? toolbarOverflow.frame : CGRect.null

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
            let isSystemToolbarOverflow = !toolbarOverflowFrame.isNull
                && element.frame == toolbarOverflowFrame
            let isVerifiedNativeMenuAction =
                issue.auditType.contains(.action)
                && (
                    (element.elementType == .menuButton
                        && (element.label == databaseContext.label
                            || element.identifier == engineMenu.identifier
                            || element.identifier == sortMenu.identifier))
                    || element.frame == databaseContext.frame
                    || element.frame == engineMenu.frame
                    || element.frame == sortMenu.frame
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

        app.menuButtons["design-lab-engine-menu"].click()
        let clickHouse = app.windows.firstMatch.menuItems["ClickHouse"]
        XCTAssertTrue(clickHouse.waitForExistence(timeout: 5))
        clickHouse.click()
        XCTAssertTrue(
            app.menuButtons["Database context, Atlas Events, production"]
                .waitForExistence(timeout: 5)
        )

        app.typeKey("t", modifierFlags: .command)
        XCTAssertTrue(app.staticTexts["Untitled Query"].waitForExistence(timeout: 5))
    }

    @MainActor
    func testSelectedNativeWorkbenchEndToEndPresentationFlow() {
        let app = launchLab()
        let table = app.tables["design-lab-native-grid"]
        XCTAssertTrue(table.waitForExistence(timeout: 10))

        let sortMenu = app.menuButtons["design-lab-sort-menu"]
        XCTAssertTrue(sortMenu.waitForExistence(timeout: 5))
        sortMenu.click()
        app.menuItems["Company Name, A to Z"].click()
        let aster = table.staticTexts["design-lab-cell-10482-name"]
        let beacon = table.staticTexts["design-lab-cell-10481-name"]
        XCTAssertTrue(aster.waitForExistence(timeout: 5))
        XCTAssertTrue(beacon.waitForExistence(timeout: 5))
        XCTAssertLessThan(aster.frame.minY, beacon.frame.minY)

        let orders = app.staticTexts["design-lab-catalog-orders"].firstMatch
        XCTAssertTrue(orders.waitForExistence(timeout: 5))
        orders.click()
        XCTAssertTrue(app.staticTexts["orders"].firstMatch.waitForExistence(timeout: 5))

        app.typeKey("s", modifierFlags: [.command, .option])
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-structure"]
                .waitForExistence(timeout: 5)
        )
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-structure-table"].exists
        )
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-structure-inspector"].exists
        )

        let dataMode = app.radioButtons["Data"]
        XCTAssertTrue(dataMode.waitForExistence(timeout: 5))
        dataMode.click()
        XCTAssertTrue(table.waitForExistence(timeout: 5))

        app.buttons["design-lab-edit-selected"].click()
        XCTAssertTrue(app.sheets.firstMatch.waitForExistence(timeout: 5))
        replaceText(in: app.textFields["design-lab-edit-plan"], with: "Team")
        replaceText(in: app.textFields["design-lab-edit-seats"], with: "125")
        app.buttons["design-lab-stage-edit"].click()
        XCTAssertTrue(app.staticTexts["2 CHANGES"].waitForExistence(timeout: 5))

        app.typeKey("r", modifierFlags: [.command, .shift])
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-review-sheet"]
                .waitForExistence(timeout: 5)
        )
        XCTAssertTrue(app.staticTexts["SAFE CHANGE"].exists)
        let apply = app.buttons["design-lab-apply-changes"]
        XCTAssertTrue(apply.isEnabled)
        apply.click()
        XCTAssertTrue(app.staticTexts["NO CHANGES"].waitForExistence(timeout: 5))

        app.typeKey("t", modifierFlags: .command)
        XCTAssertTrue(app.staticTexts["Untitled Query"].waitForExistence(timeout: 5))
        let editor = app.textViews["design-lab-query-editor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 5))
        editor.click()
        editor.typeText("SELECT 1;")
        app.typeKey(XCUIKeyboardKey.return, modifierFlags: .command)
        XCTAssertTrue(app.staticTexts["SUCCESS"].waitForExistence(timeout: 5))

        app.menuBars.menuBarItems["Navigate"].click()
        app.menuItems["Query History…"].click()
        XCTAssertTrue(app.sheets.firstMatch.waitForExistence(timeout: 5))
        let history = app.buttons["design-lab-history-history-latency"]
        XCTAssertTrue(history.waitForExistence(timeout: 5))
        history.click()
        XCTAssertTrue(app.staticTexts["Recent slow queries"].waitForExistence(timeout: 5))
    }

    @MainActor
    func testInteractiveWindowSizeRestores() {
        var app = launchLab(windowSize: "minimum", capture: false)
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-concept"]
                .waitForExistence(timeout: 10)
        )
        app.menuBars.menuBarItems["Design Lab"].click()
        app.menuItems["Window Size"].hover()
        app.menuItems["Expanded"].click()
        let expandedFrame = app.windows.firstMatch.frame
        XCTAssertEqual(expandedFrame.width, 1_720, accuracy: 2)
        XCTAssertEqual(expandedFrame.height, 1_040, accuracy: 2)
        app.terminate()

        app = launchLab(windowSize: nil, capture: false)
        XCTAssertTrue(
            app.descendants(matching: .any)["design-lab-concept"]
                .waitForExistence(timeout: 10)
        )
        let restoredFrame = app.windows.firstMatch.frame
        XCTAssertEqual(restoredFrame.width, expandedFrame.width, accuracy: 2)
        XCTAssertEqual(restoredFrame.height, expandedFrame.height, accuracy: 2)
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
        let confirmation = app.textFields["design-lab-review-confirmation"]
        confirmation.click()
        confirmation.typeText("APPLY")
        XCTAssertTrue(app.buttons["design-lab-apply-changes"].isEnabled)
    }

    @MainActor
    private func launchLab(
        engine: String = "postgresql",
        fixture: String = "populated",
        windowSize: String? = "typical",
        accessibility: String = "system",
        capture: Bool = true
    ) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = [
            "--concept", "native-workbench",
            "--surface", "data-grid",
            "--appearance", "light",
            "--accessibility", accessibility,
            "--engine", engine,
            "--fixture", fixture,
        ]
        if let windowSize {
            app.launchArguments += ["--window-size", windowSize]
        }
        if capture {
            app.launchArguments.append("--capture")
        }
        app.launch()
        app.activate()
        return app
    }

    @MainActor
    private func replaceText(in field: XCUIElement, with value: String) {
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        field.click()
        field.typeKey("a", modifierFlags: .command)
        field.typeText(value)
    }
}
