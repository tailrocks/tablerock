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
  func testProfileCreationSavesAndAppearsThroughUserControls() throws {
    let app = launch(scenario: "success")

    let add = app.buttons["profile.add"]
    XCTAssertTrue(add.waitForExistence(timeout: 10))
    add.click()

    let name = app.textFields["profile.editor.name"]
    XCTAssertTrue(name.waitForExistence(timeout: 10))
    name.click()
    name.typeText("Created fixture")

    let save = app.buttons["profile.editor.save"]
    XCTAssertTrue(save.isEnabled)
    app.typeKey(.return, modifierFlags: [])

    let created = app.buttons["profile.09090909090909090909090909090909"]
    XCTAssertTrue(created.waitForExistence(timeout: 15))
  }

  @MainActor
  func testTemporaryConnectionOpensThroughUserControl() throws {
    let app = launch(scenario: "success")

    let connect = app.buttons["connection.direct.connect"]
    XCTAssertTrue(connect.waitForExistence(timeout: 10))
    XCTAssertTrue(connect.isEnabled)
    connect.click()

    XCTAssertTrue(
      app.descendants(matching: .any)["connection.status"]
        .waitForExistence(timeout: 10))
  }

  @MainActor
  func testCatalogRefreshLoadsHierarchyThroughUserControl() throws {
    let app = launch(scenario: "success")

    let connect = app.buttons["connection.direct.connect"]
    XCTAssertTrue(connect.waitForExistence(timeout: 10))
    connect.click()

    let refresh = app.buttons["catalog.refresh"]
    XCTAssertTrue(refresh.waitForExistence(timeout: 10))
    refresh.click()

    XCTAssertTrue(app.outlines["catalog.outline"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.staticTexts["public"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.staticTexts["fixture_table"].waitForExistence(timeout: 10))
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
  func testGridSelectionOpensValueInspector() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_SELECTABLE_INSPECTOR": "1"])

    let cell = app.descendants(matching: .any)["results.cell.0.0"]
    XCTAssertTrue(cell.waitForExistence(timeout: 10))
    XCTAssertFalse(app.descendants(matching: .any)["value.inspector"].exists)
    cell.click()

    XCTAssertTrue(
      app.descendants(matching: .any)["value.inspector"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["value.inspector.tree"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testResultPagingAppendsThroughUserControl() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_RESULT_PAGING": "1"])

    let nextPage = app.buttons["results.next-page"]
    XCTAssertTrue(nextPage.waitForExistence(timeout: 10))
    let status = app.staticTexts["query.status"]
    XCTAssertEqual(status.value as? String, "result · 1 column · 500 rows loaded")
    XCTAssertTrue(nextPage.isHittable)
    nextPage.click()

    let exhausted = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "exists == false"), object: nextPage)
    XCTAssertEqual(XCTWaiter.wait(for: [exhausted], timeout: 15), .completed)
    XCTAssertEqual(status.value as? String, "result · 1 column · 501 rows loaded")
  }

  @MainActor
  func testObjectSortAndFilterControlsOperate() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_OBJECT_TABS": "1"])

    let addSort = app.descendants(matching: .any)["object.sort.add"]
    XCTAssertTrue(addSort.waitForExistence(timeout: 10))
    addSort.click()
    let idColumn = app.menuItems["id"]
    XCTAssertTrue(idColumn.waitForExistence(timeout: 5))
    idColumn.click()

    let sort = app.descendants(matching: .any)["object.sort.active.id"]
    XCTAssertTrue(sort.waitForExistence(timeout: 5))
    let direction = app.buttons["id, ascending; change direction"]
    XCTAssertTrue(direction.exists)
    direction.click()
    XCTAssertTrue(app.buttons["id, descending; change direction"].waitForExistence(timeout: 5))

    let value = app.descendants(matching: .any)["object.filter.value"]
    XCTAssertTrue(value.waitForExistence(timeout: 5))
    value.click()
    value.typeText("2")
    let addFilter = app.descendants(matching: .any)["object.filter.add"]
    XCTAssertTrue(addFilter.isEnabled)
    addFilter.click()

    let filter = app.descendants(matching: .any)["object.filter.active"]
    XCTAssertTrue(filter.waitForExistence(timeout: 5))
    XCTAssertEqual(filter.label, "id Equals 2")

    let rawWhere = app.descendants(matching: .any)["object.raw-where.editor"]
    XCTAssertTrue(rawWhere.waitForExistence(timeout: 5))
    rawWhere.click()
    rawWhere.typeText("id > 1")
    let applyRawWhere = app.buttons["object.raw-where.apply"]
    XCTAssertTrue(applyRawWhere.isEnabled)
    applyRawWhere.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["object.raw-where.active"].waitForExistence(timeout: 5))
    let clearRawWhere = app.buttons["object.raw-where.clear"]
    XCTAssertTrue(clearRawWhere.isHittable)
    clearRawWhere.click()
    XCTAssertFalse(app.descendants(matching: .any)["object.raw-where.active"].exists)
  }

  @MainActor
  func testDirtyQueryTabRequiresDiscardConfirmation() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_QUERY_TABS": "1"])

    let editor = app.textViews["query.editor"]
    XCTAssertTrue(editor.waitForExistence(timeout: 10))
    editor.click()
    editor.typeText(" -- dirty")

    let actions = app.descendants(matching: .any)["Actions for Orders"]
    XCTAssertTrue(actions.waitForExistence(timeout: 10))
    actions.click()
    let close = app.descendants(matching: .any)["query.tab.close"]
    XCTAssertTrue(close.waitForExistence(timeout: 5))
    close.click()

    XCTAssertTrue(
      app.staticTexts["Close query tab with unsaved changes?"]
        .waitForExistence(timeout: 10))
    let discard = app.descendants(matching: .any)["query.tab.discard-close"]
    XCTAssertTrue(discard.waitForExistence(timeout: 10))
    XCTAssertTrue(app.buttons["Cancel"].exists)
    discard.click()

    let removed = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "exists == false"),
      object: app.descendants(matching: .any)["Actions for Orders"])
    XCTAssertEqual(XCTWaiter.wait(for: [removed], timeout: 10), .completed)
    XCTAssertTrue(
      app.descendants(matching: .any)["Actions for Users"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testLoadedResultExportsThroughUserControls() throws {
    let root = FileManager.default.temporaryDirectory
      .appendingPathComponent("TableRock-XCUITest-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let output = root.appendingPathComponent("result.csv")
    let app = launch(
      scenario: "success", root: root,
      environment: [
        "TABLEROCK_FIXTURE_DATA_MOVEMENT_UI": "1",
        "TABLEROCK_TEST_SAVE_FILE": output.path,
      ])

    let export = app.descendants(matching: .any)["results.export"]
    XCTAssertTrue(export.waitForExistence(timeout: 10))
    export.click()
    let csv = app.descendants(matching: .any)["results.export.csv"]
    XCTAssertTrue(csv.waitForExistence(timeout: 5))
    csv.click()

    let outcome = app.staticTexts["results.copy.outcome"]
    let exported = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS 'Exported 14 bytes'"), object: outcome)
    XCTAssertEqual(XCTWaiter.wait(for: [exported], timeout: 10), .completed)
    XCTAssertEqual(try String(contentsOf: output, encoding: .utf8), "id,name\n1,Ada\n")
  }

  @MainActor
  func testCsvImportReviewsAndAppliesThroughUserControls() throws {
    let root = FileManager.default.temporaryDirectory
      .appendingPathComponent("TableRock-XCUITest-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let input = root.appendingPathComponent("input.csv")
    try "id,name\n2,Grace\n".write(to: input, atomically: true, encoding: .utf8)
    let app = launch(
      scenario: "success", root: root,
      environment: [
        "TABLEROCK_FIXTURE_DATA_MOVEMENT_UI": "1",
        "TABLEROCK_TEST_OPEN_FILE": input.path,
      ])

    let open = app.buttons["import.csv.open"]
    XCTAssertTrue(open.waitForExistence(timeout: 10))
    open.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["import.csv.sheet"].waitForExistence(timeout: 10))
    let stage = app.descendants(matching: .any)["import.csv.stage"]
    XCTAssertTrue(stage.waitForExistence(timeout: 10))
    stage.click()
    let apply = app.descendants(matching: .any)["import.csv.apply"]
    XCTAssertTrue(apply.waitForExistence(timeout: 10))
    apply.click()

    let outcome = app.descendants(matching: .any)["import.csv.outcome"]
    let applied = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS '1 applied'"), object: outcome)
    XCTAssertEqual(XCTWaiter.wait(for: [applied], timeout: 10), .completed)
  }

  @MainActor
  func testMarkedTextSurvivesPresentationUpdate() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_IME": "1"])

    let status = app.staticTexts["app.status"]
    XCTAssertTrue(status.waitForExistence(timeout: 10))
    let preserved = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value == 'IME composition preserved'"), object: status)
    XCTAssertEqual(XCTWaiter.wait(for: [preserved], timeout: 10), .completed)
  }

  @MainActor
  private func launch(
    scenario: String,
    root providedRoot: URL? = nil,
    environment: [String: String] = [:]
  ) -> XCUIApplication {
    let app = XCUIApplication()
    let root = providedRoot
      ?? FileManager.default.temporaryDirectory
        .appendingPathComponent("TableRock-XCUITest-\(UUID().uuidString)", isDirectory: true)
    addTeardownBlock { try? FileManager.default.removeItem(at: root) }
    app.launchEnvironment = [
      "TABLEROCK_TEST_MODE": "1",
      "TABLEROCK_TEST_ROOT": root.path,
      "TABLEROCK_TEST_BACKEND": "scripted",
      "TABLEROCK_TEST_SCENARIO": scenario,
    ].merging(environment) { _, fixture in fixture }
    app.launchArguments = ["-ApplePersistenceIgnoreState", "YES"]
    app.launch()
    return app
  }
}
