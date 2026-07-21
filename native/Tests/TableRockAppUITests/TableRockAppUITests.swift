import XCTest

final class TableRockAppUITests: XCTestCase {
  override func setUpWithError() throws {
    continueAfterFailure = false
  }

  func testWorkbenchLaunchesWithIsolatedScriptedBackend() throws {
    let app = launch(scenario: "success")

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.outlines["sidebar.profiles"].exists)
  }

  func testSlowQueryCancelsThroughRustBoundary() throws {
    let app = launch(scenario: "slow-until-cancelled")
    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))

    app.buttons["Connect"].click()
    let editor = app.textViews["query.editor"]
    XCTAssertTrue(editor.waitForExistence(timeout: 10))
    editor.click()
    editor.typeText("SELECT pg_sleep(30);")

    let run = app.buttons["query.run"]
    run.click()
    let cancel = app.buttons["query.cancel"]
    XCTAssertTrue(cancel.isEnabled)
    cancel.click()

    let status = app.staticTexts["query.status"]
    let cancelled = NSPredicate(format: "label CONTAINS[c] 'Requested'")
    expectation(for: cancelled, evaluatedWith: status)
    waitForExpectations(timeout: 10)
  }

  func testProfileEditorFixtureExposesStableControls() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_PROFILE_EDITOR": "1"])

    XCTAssertTrue(app.textFields["profile.editor.name"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.buttons["profile.editor.save"].exists)
  }

  func testAccessibilityFixtureExposesCatalogEditorAndGrid() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_ACCESSIBILITY_AUDIT": "1"])

    XCTAssertTrue(app.outlines["catalog.outline"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.textViews["query.editor"].exists)
    XCTAssertTrue(app.tables["results.grid"].exists)
  }

  func testLargeGridFixtureExposesBoundedNativeTable() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_GRID_ROWS": "10000"])

    XCTAssertTrue(app.tables["results.grid"].waitForExistence(timeout: 10))
  }

  private func launch(
    scenario: String,
    environment: [String: String] = [:]
  ) -> XCUIApplication {
    let app = XCUIApplication()
    let root = FileManager.default.temporaryDirectory
      .appendingPathComponent("TableRock-XCUITest-\(UUID().uuidString)", isDirectory: true)
    addTeardownBlock { try? FileManager.default.removeItem(at: root) }
    app.launchEnvironment = [
      "TABLEROCK_TEST_MODE": "1",
      "TABLEROCK_TEST_ROOT": root.path,
      "TABLEROCK_TEST_BACKEND": "scripted",
      "TABLEROCK_TEST_SCENARIO": scenario,
    ].merging(environment) { _, fixture in fixture }
    app.launch()
    return app
  }
}
