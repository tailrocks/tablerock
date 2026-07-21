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
    let app = launch(
      scenario: "slow-until-cancelled",
      environment: ["TABLEROCK_FIXTURE_ACTIVE_QUERY": "1"])
    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))

    let cancel = app.buttons["query.cancel"]
    let cancellable = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: cancel)
    XCTAssertEqual(XCTWaiter.wait(for: [cancellable], timeout: 10), .completed)
    cancel.click()

    let status = app.staticTexts["query.status"]
    let cancelled = NSPredicate(
      format: "value CONTAINS[c] 'Requested' OR value CONTAINS[c] 'cancelled'")
    let terminalState = XCTNSPredicateExpectation(predicate: cancelled, object: status)
    XCTAssertEqual(XCTWaiter.wait(for: [terminalState], timeout: 10), .completed)
  }

  @MainActor
  func testProfileCreationOpensEditorThroughUserControl() throws {
    let app = launch(scenario: "success")

    let add = app.buttons["profile.add"]
    XCTAssertTrue(add.waitForExistence(timeout: 10))
    add.click()

    XCTAssertTrue(app.textFields["profile.editor.name"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.buttons["profile.editor.save"].exists)
  }

  @MainActor
  func testMultiWindowFixtureCreatesIndependentWorkbenchWindow() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_MULTI_WINDOW": "1"])

    let windows = app.windows.matching(identifier: "window.workbench")
    let twoWindows = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "count == 2"), object: windows)
    XCTAssertEqual(XCTWaiter.wait(for: [twoWindows], timeout: 10), .completed)
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
  func testSettingsExposeSafeSupportExport() throws {
    let app = launch(scenario: "success")
    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))

    app.typeKey(",", modifierFlags: .command)

    XCTAssertTrue(app.buttons["settings.support.export"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testStructuredValueInspectorExposesJSONTree() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_VALUE_INSPECTOR": "1"])

    XCTAssertTrue(
      app.descendants(matching: .any)["value.inspector"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["value.inspector.tree"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testMarkedTextSurvivesPresentationUpdate() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_IME": "1"])

    let status = app.staticTexts["query.status"]
    XCTAssertTrue(status.waitForExistence(timeout: 10))
    let preserved = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value == 'IME composition preserved'"), object: status)
    XCTAssertEqual(XCTWaiter.wait(for: [preserved], timeout: 10), .completed)
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
