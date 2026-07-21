import XCTest

final class TableRockAppUITests: XCTestCase {
  override func setUpWithError() throws {
    continueAfterFailure = false
  }

  @MainActor
  func testWorkbenchLaunchesWithIsolatedScriptedBackend() throws {
    let app = launch(scenario: "success")

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.outlines["sidebar.profiles"].exists)
  }

  @MainActor
  func testSlowQueryCancelsThroughRustBoundary() throws {
    let app = launch(scenario: "slow-until-cancelled")
    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))

    let connect = app.buttons["connection.direct.connect"]
    XCTAssertTrue(connect.waitForExistence(timeout: 10))
    connect.click()

    let run = app.buttons["query.run"]
    XCTAssertTrue(run.waitForExistence(timeout: 10))
    let runnable = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: run)
    XCTAssertEqual(XCTWaiter.wait(for: [runnable], timeout: 10), .completed)
    run.click()
    let cancel = app.buttons["query.cancel"]
    let cancellable = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: cancel)
    XCTAssertEqual(XCTWaiter.wait(for: [cancellable], timeout: 10), .completed)
    cancel.click()

    let status = app.staticTexts["query.status"]
    let cancelled = NSPredicate(format: "label CONTAINS[c] 'Requested'")
    let terminalState = XCTNSPredicateExpectation(predicate: cancelled, object: status)
    XCTAssertEqual(XCTWaiter.wait(for: [terminalState], timeout: 10), .completed)
  }

  @MainActor
  func testProfileEditorFixtureExposesStableControls() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_PROFILE_EDITOR": "1"])

    XCTAssertTrue(app.textFields["profile.editor.name"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.buttons["profile.editor.save"].exists)
  }

  @MainActor
  func testAccessibilityFixtureExposesCatalogEditorAndGrid() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_ACCESSIBILITY_AUDIT": "1"])

    XCTAssertTrue(app.outlines["catalog.outline"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.textViews["query.editor"].exists)
    XCTAssertTrue(app.tables["results.grid"].exists)
  }

  @MainActor
  func testLargeGridFixtureExposesBoundedNativeTable() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_GRID_ROWS": "10000"])

    XCTAssertTrue(app.tables["results.grid"].waitForExistence(timeout: 10))
  }

  @MainActor
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
