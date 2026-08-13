import AppKit
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
  func testProductionWorkbenchPassesAccessibilityAudit() throws {
    for fixture in [
      "TABLEROCK_FIXTURE_NATIVE_WORKBENCH",
      "TABLEROCK_FIXTURE_NATIVE_WORKBENCH_QUERY",
    ] {
      let app = launch(scenario: "success", environment: [fixture: "1"])
      app.windows["window.workbench"].hover()
      try app.performAccessibilityAudit(for: .all) { issue in
        // macOS exposes its empty system Touch Bar as an application sibling but
        // gives it no public AppKit labeling surface. Keep every app-owned issue fatal.
        let systemTouchBarIssue =
          issue.auditType == .sufficientElementDescription
          && issue.element?.elementType == .touchBar
          && issue.element?.children(matching: .any).count == 0
        let fullScreenGlyph =
          app.windows["window.workbench"].buttons["_XCUI:FullScreenWindow"]
          .children(matching: .group).firstMatch
        let systemFullScreenGlyphIssue =
          issue.auditType == .parentChild
          && issue.element?.elementType == .group
          && fullScreenGlyph.exists
          && issue.element?.frame == fullScreenGlyph.frame
        let labeledSwiftUISplitWrapper =
          issue.auditType == .sufficientElementDescription
          && issue.element?.elementType == .group
          && ["Query editor pane", "Query result pane"].contains { label in
            issue.element?.children(matching: .group)
              .matching(NSPredicate(format: "label == %@", label)).count == 1
          }
        return systemTouchBarIssue || systemFullScreenGlyphIssue || labeledSwiftUISplitWrapper
      }
      app.terminate()
    }
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsDataPlane() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.descendants(matching: .any)["catalog.search"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["object.header"].exists)
    XCTAssertTrue(app.tables["results.grid"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["value.inspector"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["value.inspector.row-details"].exists)
    XCTAssertEqual(
      app.descendants(matching: .any)["value.inspector.kind"].value as? String,
      "TEXT · NOT NULL")
    XCTAssertTrue(app.descendants(matching: .any)["object.section"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["workbench.status"].exists)
    XCTAssertEqual(
      app.descendants(matching: .any).matching(identifier: "object.filter.active").count, 2)
    let selectedValue = app.descendants(matching: .any)["results.cell.2.1"]
    XCTAssertTrue(selectedValue.exists)
    XCTAssertEqual(selectedValue.value as? String, "Aster Works")
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsStructurePlane() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STRUCTURE": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["object.structure"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.descendants(matching: .any)["structure.columns"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["structure.details"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["structure.inspector"].exists)
    XCTAssertGreaterThan(
      app.descendants(matching: .any)
        .matching(NSPredicate(format: "label CONTAINS 'customers_pkey'"))
        .count,
      0)
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsSafeReview() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SAFE_REVIEW": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["mutation.review.entry"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.descendants(matching: .any)["change.review.safe"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["mutation.review.entry"].exists)
    XCTAssertTrue(app.buttons["mutation.apply"].isEnabled)
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsSafeRowEditor() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SAFE_EDIT": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.textFields["mutation.field.plan"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.textFields["mutation.field.seats"].exists)
    XCTAssertTrue(app.buttons["mutation.stage-review"].isEnabled)
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsDestructiveReview() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_DESTRUCTIVE_REVIEW": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["structure.change.sheet"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.descendants(matching: .any)["change.review.destructive"].exists)
    let confirmation = app.textFields["structure.change.confirmation"]
    XCTAssertTrue(confirmation.exists)
    let apply = app.buttons["structure.change.apply-review"]
    XCTAssertFalse(apply.isEnabled)
    confirmation.click()
    confirmation.typeText("APPLY")
    let enabled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: apply)
    XCTAssertEqual(XCTWaiter.wait(for: [enabled], timeout: 5), .completed)
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsClickHouseTruth() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_ENGINE": "clickhouse"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.descendants(matching: .any)["value.inspector"].exists)
    XCTAssertEqual(
      app.descendants(matching: .any)["value.inspector.kind"].value as? String,
      "STRING · NOT NULL")
    XCTAssertTrue(
      app.descendants(matching: .any)["workbench.context.connection"].label
        .localizedCaseInsensitiveContains("Atlas Events"))
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsRedisTruth() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_ENGINE": "redis"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["redis.key.view"].waitForExistence(timeout: 10))
    XCTAssertFalse(app.descendants(matching: .any)["value.inspector"].exists)
    XCTAssertTrue(
      app.descendants(matching: .any)["workbench.context.connection"].label
        .localizedCaseInsensitiveContains("Arbor Cache"))
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsLoadingState() throws {
    let loading = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STATE": "loading"])
    XCTAssertTrue(
      loading.descendants(matching: .any)["object.loading"].waitForExistence(timeout: 10))
    XCTAssertTrue(loading.descendants(matching: .any)["catalog.loading"].exists)
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsConnectionErrorState() throws {
    let connectionError = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STATE": "connection-error"])
    XCTAssertTrue(
      connectionError.descendants(matching: .any)["object.error"].waitForExistence(timeout: 10))
    XCTAssertTrue(connectionError.descendants(matching: .any)["catalog.error"].exists)
    XCTAssertEqual(
      connectionError.descendants(matching: .any)["connection.status"].value as? String,
      "Unavailable")
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsEmptyState() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STATE": "empty"])

    XCTAssertTrue(
      app.descendants(matching: .any)["results.grid.empty"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsLongIdentifierState() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STATE": "long-identifiers"])
    let title = "customer_engagement_retention_cohort_materialized_rollup_by_billing_region"

    XCTAssertTrue(app.staticTexts[title].waitForExistence(timeout: 10))
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsLargeResultState() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STATE": "large-result"])

    XCTAssertTrue(app.tables["results.grid"].waitForExistence(timeout: 20))
    XCTAssertTrue(
      app.staticTexts["1,500 of 48,224 rows · 86 ms"].waitForExistence(timeout: 20))
  }

  @MainActor
  func testNativeWorkbenchFixtureProjectsQueryErrorState() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STATE": "query-error"])

    XCTAssertTrue(
      app.descendants(matching: .any)["query.messages"].waitForExistence(timeout: 10))
    XCTAssertEqual(
      app.staticTexts["query.status"].value as? String,
      "Column monthly_revenue_total does not exist at line 5, column 3.")
    XCTAssertEqual(app.alerts.count, 0)
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsQueryPlane() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_QUERY": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["query.header"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.textViews["query.editor"].exists)
    XCTAssertTrue(app.descendants(matching: .any)["query.result-section"].exists)
    XCTAssertTrue(app.tables["results.grid"].exists)
    XCTAssertTrue(app.buttons["query.run"].exists)
    let customersTab = app.buttons.matching(
      NSPredicate(format: "label BEGINSWITH 'customers'")
    ).firstMatch
    let queryTab = app.buttons.matching(
      NSPredicate(format: "label BEGINSWITH 'Revenue by region'")
    ).firstMatch
    let ordersTab = app.buttons.matching(
      NSPredicate(format: "label BEGINSWITH 'orders'")
    ).firstMatch
    XCTAssertTrue(customersTab.waitForExistence(timeout: 10))
    XCTAssertTrue(queryTab.exists)
    XCTAssertTrue(ordersTab.exists)
    XCTAssertLessThan(customersTab.frame.minX, queryTab.frame.minX)
    XCTAssertLessThan(queryTab.frame.minX, ordersTab.frame.minX)
    let status = app.staticTexts["query.status"]
    XCTAssertTrue(status.exists)
    XCTAssertEqual(status.value as? String, "100 rows · 126 ms · success")
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsConnectionBrowser() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_CONNECTIONS": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["toolbar.connection"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["sidebar.profile.04040404040404040404040404040404"]
        .waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["profile.04040404040404040404040404040404"]
        .waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["profile.05050505050505050505050505050505"]
        .waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["profile.06060606060606060606060606060606"]
        .waitForExistence(timeout: 10))
    XCTAssertTrue(app.buttons["profile.add"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.buttons["profile.url-import"].waitForExistence(timeout: 10))
    XCTAssertFalse(app.descendants(matching: .any)["connection.status"].exists)
  }

  @MainActor
  func testNativeWorkbenchFixtureOwnsConnectionSetup() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SETUP": "1"])

    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.descendants(matching: .any)["connection.setup"].exists)
    XCTAssertEqual(app.textFields["profile.editor.name"].value as? String, "Northstar Analytics")
    XCTAssertEqual(app.textFields["profile.editor.host"].value as? String, "analytics.internal")
    XCTAssertTrue(app.buttons["profile.editor.test"].exists)
    XCTAssertTrue(app.buttons["profile.editor.save"].exists)
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
  func testFindReplaceRunsThroughEditorSheet() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_ACTIVE_QUERY": "1"])
    let editor = app.textViews["query.editor"]
    XCTAssertTrue(editor.waitForExistence(timeout: 10))

    app.activate()
    app.typeKey("f", modifierFlags: [.command, .option])
    let pattern = app.textFields["find-replace.pattern"]
    XCTAssertTrue(pattern.waitForExistence(timeout: 10))
    pattern.click()
    pattern.typeText("SELECT")
    let replacement = app.textFields["find-replace.replacement"]
    replacement.click()
    replacement.typeText("VALUES")
    app.buttons["find-replace.replace-all"].click()

    XCTAssertTrue(app.staticTexts["find-replace.status"].waitForExistence(timeout: 5))
    XCTAssertTrue((editor.value as? String ?? "").contains("VALUES"))
    app.buttons["find-replace.dismiss"].click()
  }

  @MainActor
  func testNamedParametersRequireSheetBeforeExecution() throws {
    let app = launch(scenario: "success")
    connectTemporarily(app)
    let editor = app.textViews["query.editor"]
    XCTAssertTrue(editor.waitForExistence(timeout: 10))
    editor.click()
    editor.typeKey("a", modifierFlags: .command)
    editor.typeText("SELECT :id")
    XCTAssertTrue((editor.value as? String ?? "").contains(":id"))
    app.buttons["query.run"].click()

    let value = app.textFields["query-parameters.value.id"]
    XCTAssertTrue(value.waitForExistence(timeout: 10))
    value.click()
    value.typeText("42 OR 1=1")
    XCTAssertFalse(app.tables["results.grid"].exists)
    app.buttons["query-parameters.run"].click()

    let status = app.staticTexts["query.status"]
    let completed = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS 'write ok'"), object: status)
    XCTAssertEqual(XCTWaiter.wait(for: [completed], timeout: 10), .completed)
    XCTAssertFalse(app.descendants(matching: .any)["query-parameters.sheet"].exists)
  }

  @MainActor
  func testProfileCreationSavesAndAppearsThroughUserControls() throws {
    let app = launch(scenario: "success")

    let name = openProfileEditor(in: app)
    name.click()
    name.typeText("Created fixture")

    let save = app.buttons["profile.editor.save"]
    XCTAssertTrue(save.isEnabled)
    app.typeKey(.return, modifierFlags: [])

    let created = app.buttons["profile.09090909090909090909090909090909"]
    XCTAssertTrue(created.waitForExistence(timeout: 15))
  }

  @MainActor
  func testConnectionUrlImportRequiresReviewBeforeSave() throws {
    let app = launch(scenario: "success")

    let importButton = app.buttons["profile.url-import"]
    XCTAssertTrue(importButton.waitForExistence(timeout: 10))
    importButton.click()

    let input = app.secureTextFields["profile.url-import.input"]
    XCTAssertTrue(input.waitForExistence(timeout: 10))
    input.click()
    input.typeText("postgresql://fixture:secret@db.example:5433/app")
    app.buttons["profile.url-import.review"].click()

    let name = app.textFields["profile.editor.name"]
    XCTAssertTrue(name.waitForExistence(timeout: 10))
    XCTAssertEqual(name.value as? String, "app on db.example")
    XCTAssertEqual(app.textFields["profile.editor.host"].value as? String, "db.example")
    XCTAssertEqual(app.textFields["profile.editor.port"].value as? String, "5433")
    XCTAssertEqual(app.textFields["profile.editor.database"].value as? String, "app")
    XCTAssertEqual(app.textFields["profile.editor.username"].value as? String, "fixture")
    XCTAssertTrue(app.buttons["profile.editor.save"].isEnabled)
  }

  @MainActor
  func testExternalUrlRequiresAuthorityBeforeTemporaryConnect() throws {
    let encoded =
      "postgresql%3A%2F%2Ffixture%3Asecret%40db.example%3A5433%2Fapp"
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_EXTERNAL_URL": "tablerock://open?url=\(encoded)"])

    let summary = app.staticTexts["external-url.summary"]
    XCTAssertTrue(summary.waitForExistence(timeout: 10))
    XCTAssertFalse((summary.value as? String ?? summary.label).contains("secret"))
    XCTAssertFalse(app.descendants(matching: .any)["connection.status"].exists)

    let connect = app.buttons["external-url.connect-temporary"]
    XCTAssertTrue(connect.exists)
    connect.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["connection.status"]
        .waitForExistence(timeout: 10))
  }

  @MainActor
  func testTemporaryConnectionOpensThroughUserControl() throws {
    let app = launch(scenario: "success")

    connectTemporarily(app)

    XCTAssertTrue(
      app.descendants(matching: .any)["connection.status"]
        .waitForExistence(timeout: 10))
  }

  @MainActor
  func testCatalogRefreshLoadsHierarchyThroughUserControl() throws {
    let app = launch(scenario: "success")

    connectTemporarily(app)

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
    let restoredWindows = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "count == 1 OR count == 2"), object: windows)
    XCTAssertEqual(XCTWaiter.wait(for: [restoredWindows], timeout: 10), .completed)
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
  func testSettingsExportsSafeSupportBundle() throws {
    let root = FileManager.default.temporaryDirectory
      .appendingPathComponent("TableRock-XCUITest-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let output = root.appendingPathComponent("support.txt")
    let app = launch(
      scenario: "success", root: root,
      environment: ["TABLEROCK_TEST_SAVE_FILE": output.path])
    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))

    app.typeKey(",", modifierFlags: .command)

    let export = app.buttons["settings.support.export"]
    XCTAssertTrue(export.waitForExistence(timeout: 10))
    export.click()
    let outcome = app.staticTexts["settings.support.outcome"]
    let exported = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS 'Exported'"), object: outcome)
    XCTAssertEqual(XCTWaiter.wait(for: [exported], timeout: 10), .completed)

    let payload = try String(contentsOf: output, encoding: .utf8)
    XCTAssertTrue(payload.contains("schema=1\n"))
    XCTAssertTrue(payload.contains("diagnostics.count=0\n"))
    XCTAssertFalse(payload.localizedCaseInsensitiveContains("password"))
    XCTAssertFalse(payload.localizedCaseInsensitiveContains("statement"))
  }

  @MainActor
  func testStructuredValueInspectorExposesJSONTree() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_VALUE_INSPECTOR": "1"])

    XCTAssertTrue(
      app.descendants(matching: .any)["value.inspector"].waitForExistence(timeout: 10))
    app.descendants(matching: .any)["value.inspector"].swipeUp()
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
    app.descendants(matching: .any)["value.inspector"].swipeUp()
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
  func testLoadedRowQuickFilterIsExplicitAndOperable() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_QUICK_FILTER": "1"])

    activateAndClick(app.buttons["query.quick-filter.open"], in: app)
    let filter = app.textFields["results.quick-filter"]
    XCTAssertTrue(filter.waitForExistence(timeout: 10))
    let status = app.staticTexts["results.quick-filter.status"]
    XCTAssertEqual(status.value as? String, "Loaded rows only · 3/3")
    filter.click()
    filter.typeText("Grace")
    let cell = app.descendants(matching: .any)["results.cell.0.1"]
    XCTAssertTrue(cell.waitForExistence(timeout: 5))
    XCTAssertEqual(cell.value as? String, "Grace")
    XCTAssertEqual(status.value as? String, "Loaded rows only · 1/3")
  }

  @MainActor
  func testObjectSortAndFilterControlsOperate() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_OBJECT_TABS": "1"])

    let addSort = app.descendants(matching: .any)["object.sort.add"]
    XCTAssertTrue(addSort.waitForExistence(timeout: 10))
    app.activate()
    addSort.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    let idColumn = app.menuItems["id"]
    if !idColumn.waitForExistence(timeout: 5) {
      app.typeKey(.escape, modifierFlags: [])
      app.activate()
      addSort.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    }
    XCTAssertTrue(idColumn.waitForExistence(timeout: 5))
    idColumn.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()

    let sorted = app.descendants(matching: .any)["object.sort.add"]
    let ascending = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "label == 'Sorted by id ascending'"), object: sorted)
    XCTAssertEqual(XCTWaiter.wait(for: [ascending], timeout: 5), .completed)
    sorted.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    let direction = app.menuItems["id, ascending; change direction"]
    XCTAssertTrue(direction.waitForExistence(timeout: 5))
    direction.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    let updatedSort = app.descendants(matching: .any)["object.sort.add"]
    XCTAssertTrue(updatedSort.waitForExistence(timeout: 5))
    let descending = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "label == 'Sorted by id descending'"), object: updatedSort)
    XCTAssertEqual(XCTWaiter.wait(for: [descending], timeout: 5), .completed)
    updatedSort.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    XCTAssertTrue(app.menuItems["id, descending; change direction"].waitForExistence(timeout: 5))
    app.typeKey(.escape, modifierFlags: [])

    let openFilter = app.descendants(matching: .any)["object.filter.editor.open"]
    XCTAssertTrue(openFilter.waitForExistence(timeout: 5))
    openFilter.click()
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

    let moreFilters = app.descendants(matching: .any)["object.filter.more"]
    XCTAssertTrue(moreFilters.waitForExistence(timeout: 5))
    moreFilters.click()
    let rawWhere = app.descendants(matching: .any)["object.raw-where.editor"]
    XCTAssertTrue(rawWhere.waitForExistence(timeout: 5))
    rawWhere.click()
    rawWhere.typeText("id > 1")
    let applyRawWhere = app.buttons["object.raw-where.apply"]
    XCTAssertTrue(applyRawWhere.isEnabled)
    applyRawWhere.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["object.raw-where.active"].waitForExistence(timeout: 5))
    let presetName = app.descendants(matching: .any)["object.filter-preset.name"]
    XCTAssertTrue(presetName.waitForExistence(timeout: 5))
    presetName.click()
    presetName.typeText("active")
    app.descendants(matching: .any)["object.filter-preset.save"].click()
    let clearRawWhere = app.buttons["object.raw-where.clear"]
    XCTAssertTrue(clearRawWhere.isHittable)
    clearRawWhere.click()
    XCTAssertFalse(app.descendants(matching: .any)["object.raw-where.active"].exists)
    let loadPreset = app.descendants(matching: .any)["object.filter-preset.load"]
    XCTAssertTrue(loadPreset.isEnabled)
    loadPreset.click()
    let savedPreset = app.menuItems.matching(
      NSPredicate(format: "identifier BEGINSWITH 'object.filter-preset.load.'")
    ).firstMatch
    XCTAssertTrue(savedPreset.waitForExistence(timeout: 5))
    savedPreset.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    XCTAssertTrue(
      app.descendants(matching: .any)["object.raw-where.active"].waitForExistence(timeout: 10))
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

    let close = app.buttons["Close Orders"]
    XCTAssertTrue(close.waitForExistence(timeout: 10))
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
      object: close)
    XCTAssertEqual(XCTWaiter.wait(for: [removed], timeout: 10), .completed)
    XCTAssertTrue(app.buttons["Close Users"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testQuickSwitcherSearchesAndActivatesCurrentItems() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_QUERY_TABS": "1"])
    let editor = app.textViews["query.editor"]
    XCTAssertTrue(editor.waitForExistence(timeout: 10))
    XCTAssertTrue((editor.value as? String ?? "").contains("SELECT 2"))

    app.activate()
    app.typeKey("o", modifierFlags: [.command, .shift])
    let search = app.textFields["quick-switch.search"]
    XCTAssertTrue(search.waitForExistence(timeout: 10))
    search.click()
    search.typeText("Users")
    let users = app.buttons.matching(
      NSPredicate(
        format: "identifier BEGINSWITH 'quick-switch.item.' AND label BEGINSWITH 'Users'")
    )
    .firstMatch
    XCTAssertTrue(users.waitForExistence(timeout: 5))
    users.click()

    let switched = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS 'SELECT 1'"), object: editor)
    XCTAssertEqual(XCTWaiter.wait(for: [switched], timeout: 10), .completed)
  }

  @MainActor
  func testExplainRunsThroughRustIntentAndOpensPlanPlane() throws {
    let app = launch(scenario: "success")
    connectTemporarily(app)
    XCTAssertTrue(
      app.descendants(matching: .any)["connection.status"].waitForExistence(timeout: 10))

    let explain = app.menuItems["Explain Query"]
    XCTAssertTrue(explain.waitForExistence(timeout: 10))
    XCTAssertTrue(explain.isEnabled)
    explain.click()
    let plan = app.staticTexts["explain.plan"]
    XCTAssertTrue(plan.waitForExistence(timeout: 10))
    XCTAssertTrue((plan.value as? String ?? plan.label).contains("Seq Scan on fixture"))
    XCTAssertTrue(app.descendants(matching: .any)["explain.copy"].exists)
  }

  @MainActor
  func testPostgresActivityRefreshAndCancelRequireAuthority() throws {
    let app = launch(scenario: "success")
    connectTemporarily(app)
    XCTAssertTrue(
      app.descendants(matching: .any)["connection.status"].waitForExistence(timeout: 10))

    let command = app.menuItems["PostgreSQL Activity…"]
    XCTAssertTrue(command.waitForExistence(timeout: 10))
    XCTAssertTrue(command.isEnabled)
    command.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["postgres.activity.row.4242"]
        .waitForExistence(timeout: 10))

    app.buttons["postgres.activity.cancel.4242"].click()
    let confirm = app.buttons["postgres.activity.confirm"]
    XCTAssertTrue(confirm.waitForExistence(timeout: 10))
    confirm.click()
    let outcome = app.descendants(matching: .any)["postgres.activity.outcome"]
    XCTAssertTrue(outcome.waitForExistence(timeout: 10))
    XCTAssertTrue((outcome.value as? String ?? outcome.label).contains("acknowledged"))
  }

  @MainActor
  func testPostgresRelationshipsOpenFromSelectedObject() throws {
    let app = launch(scenario: "success")
    connectTemporarily(app)
    let refresh = app.buttons["catalog.refresh"]
    XCTAssertTrue(refresh.waitForExistence(timeout: 10))
    refresh.click()
    let table = app.staticTexts["fixture_table"]
    XCTAssertTrue(table.waitForExistence(timeout: 10))
    table.doubleClick()

    let command = app.menuItems["Relation Lens…"]
    XCTAssertTrue(command.waitForExistence(timeout: 10))
    XCTAssertTrue(command.isEnabled)
    command.click()
    XCTAssertTrue(app.staticTexts["Self-reference"].waitForExistence(timeout: 10))
    XCTAssertTrue(app.descendants(matching: .any)["relation.lens.open"].exists)
  }

  @MainActor
  func testRedisPubSubShowsGapAndCancels() throws {
    let app = launch(
      scenario: "success",
      environment: ["TABLEROCK_FIXTURE_REDIS_PUBSUB_UI": "1"])
    XCTAssertTrue(app.windows["window.workbench"].waitForExistence(timeout: 10))

    let command = app.menuItems["Redis Pub/Sub…"]
    XCTAssertTrue(command.waitForExistence(timeout: 10))
    XCTAssertTrue(command.isEnabled)
    command.click()
    let subscribe = app.buttons["redis.pubsub.subscribe"]
    XCTAssertTrue(subscribe.waitForExistence(timeout: 10))
    XCTAssertTrue(subscribe.isEnabled)
    subscribe.click()
    XCTAssertTrue(app.staticTexts["updates:users · fixture message"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["redis.pubsub.gap"].waitForExistence(timeout: 10))
    let cancel = app.buttons["redis.pubsub.cancel"]
    XCTAssertTrue(cancel.isEnabled)
    cancel.click()
    let status = app.descendants(matching: .any)["redis.pubsub.status"]
    XCTAssertTrue(status.waitForExistence(timeout: 10))
    let cancelled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS[c] 'cancelled'"), object: status)
    XCTAssertEqual(XCTWaiter.wait(for: [cancelled], timeout: 10), .completed)
  }

  @MainActor
  func testStructureChangeRequiresFrozenReviewAndConfirmation() throws {
    let app = launch(scenario: "success")
    connectTemporarily(app)
    let refresh = app.buttons["catalog.refresh"]
    activateAndClick(refresh, in: app)
    let table = app.staticTexts["fixture_table"]
    XCTAssertTrue(table.waitForExistence(timeout: 10))
    app.activate()
    table.doubleClick()
    let structure = app.radioButtons["Structure"]
    activateAndClick(structure, in: app)
    let actions = app.descendants(matching: .any)["structure.actions"]
    activateAndClick(actions, in: app)
    let open = app.descendants(matching: .any)["structure.change.open"]
    if !open.waitForExistence(timeout: 2) {
      app.activate()
      actions.click()
    }
    XCTAssertTrue(open.waitForExistence(timeout: 10))
    activateAndClick(open, in: app)
    let object = app.textFields["structure.change.object"]
    activateAndClick(object, in: app)
    object.typeText("reviewed_column")
    let definition = app.textFields["structure.change.definition"]
    activateAndClick(definition, in: app)
    definition.typeText("text")
    activateAndClick(app.buttons["structure.change.review"], in: app)
    let preview = app.descendants(matching: .any)["structure.change.preview"]
    XCTAssertTrue(preview.waitForExistence(timeout: 10))
    let previewValue = app.descendants(matching: .any)["change.review.entry.preview"]
    XCTAssertTrue(previewValue.waitForExistence(timeout: 10))
    XCTAssertTrue((previewValue.value as? String ?? previewValue.label).contains("reviewed_column"))
    activateAndClick(app.buttons["structure.change.apply-review"], in: app)
    XCTAssertTrue(
      app.descendants(matching: .any)["structure.change.outcome"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testTableOperationRequiresExactTargetConfirmation() throws {
    let app = launch(scenario: "success")
    connectTemporarily(app)
    let refresh = app.buttons["catalog.refresh"]
    activateAndClick(refresh, in: app)
    let table = app.staticTexts["fixture_table"]
    XCTAssertTrue(table.waitForExistence(timeout: 10))
    app.activate()
    table.doubleClick()
    let structure = app.radioButtons["Structure"]
    activateAndClick(structure, in: app)
    let actions = app.descendants(matching: .any)["structure.actions"]
    activateAndClick(actions, in: app)
    let open = app.descendants(matching: .any)["table-operation.open"]
    if !open.waitForExistence(timeout: 2) {
      app.activate()
      actions.click()
    }
    XCTAssertTrue(open.waitForExistence(timeout: 10))
    activateAndClick(open, in: app)
    let review = app.buttons["table-operation.review"]
    activateAndClick(review, in: app)
    XCTAssertTrue(
      app.descendants(matching: .any)["table-operation.preview"].waitForExistence(timeout: 10))
    let apply = app.buttons["table-operation.apply"]
    XCTAssertFalse(apply.isEnabled)
    let confirmation = app.textFields["table-operation.confirmation"]
    app.activate()
    confirmation.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
    confirmation.typeText("fixture_table")
    let enabled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: apply)
    XCTAssertEqual(XCTWaiter.wait(for: [enabled], timeout: 5), .completed)
    apply.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["table-operation.progress"].waitForExistence(timeout: 10))
    XCTAssertTrue(
      app.descendants(matching: .any)["table-operation.cancel-unavailable"].exists)
    XCTAssertTrue(
      app.descendants(matching: .any)["table-operation.outcome"].waitForExistence(timeout: 10))
  }

  @MainActor
  func testPostgresRolesSearchAndInspectMembership() throws {
    let app = launch(scenario: "success")
    connectTemporarily(app)

    let command = app.menuItems["PostgreSQL Roles and Privileges…"]
    XCTAssertTrue(command.waitForExistence(timeout: 10))
    let enabled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: command)
    XCTAssertEqual(XCTWaiter.wait(for: [enabled], timeout: 5), .completed)
    command.click()
    let search = app.textFields["postgres.roles.search"]
    XCTAssertTrue(search.waitForExistence(timeout: 10))
    search.click()
    search.typeText("reader")
    XCTAssertTrue(app.staticTexts["reader"].firstMatch.exists)
    XCTAssertTrue(app.staticTexts["Current user: fixture"].exists)
    let role = app.textFields["postgres.roles.change.role"]
    role.click()
    role.typeText("reader")
    let member = app.textFields["postgres.roles.change.subject"]
    member.click()
    member.typeText("analyst")
    app.buttons["postgres.roles.change.review"].click()
    let apply = app.sheets.buttons["Apply Role Change"].firstMatch
    XCTAssertTrue(apply.waitForExistence(timeout: 10))
    apply.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["postgres.roles.change.outcome"]
        .waitForExistence(timeout: 10))
  }

  @MainActor
  func testPostgresBackupRequiresToolFileAndReview() throws {
    let root = FileManager.default.temporaryDirectory
      .appendingPathComponent("TableRock-PostgresTool-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let output = root.appendingPathComponent("fixture.dump")
    let app = launch(
      scenario: "success", root: root,
      environment: ["TABLEROCK_TEST_SAVE_FILE": output.path])
    connectTemporarily(app)
    XCTAssertTrue(
      app.descendants(matching: .any)["connection.status"].waitForExistence(timeout: 10))

    let command = app.menuItems["PostgreSQL Backup and Restore…"]
    XCTAssertTrue(command.waitForExistence(timeout: 10))
    command.click()
    XCTAssertTrue(
      app.descendants(matching: .any)["postgres.tools.probe-result"]
        .waitForExistence(timeout: 10))
    app.buttons["postgres.tools.choose-file"].click()
    app.buttons["postgres.tools.start"].click()
    let confirm = app.sheets.buttons["Create Backup"].firstMatch
    XCTAssertTrue(confirm.waitForExistence(timeout: 10))
    confirm.click()
    let status = app.descendants(matching: .any)["postgres.tools.status"]
    XCTAssertTrue(status.waitForExistence(timeout: 10))
    XCTAssertTrue(
      (status.value as? String ?? status.label).localizedCaseInsensitiveContains("succeeded"))
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

    activateAndClick(app.descendants(matching: .any)["results.export.more"], in: app)
    clickOpenMenuItem(app.descendants(matching: .any)["results.export.csv"])

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

    openCsvImport(in: app)
    let stage = app.descendants(matching: .any)["import.csv.stage"]
    XCTAssertTrue(stage.waitForExistence(timeout: 10))
    stage.click()
    let apply = app.descendants(matching: .any)["import.csv.apply"]
    XCTAssertTrue(apply.waitForExistence(timeout: 10))
    let applyEnabled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: apply)
    XCTAssertEqual(XCTWaiter.wait(for: [applyEnabled], timeout: 10), .completed)
    apply.click()

    let progress = app.descendants(matching: .any)["import.csv.progress"]
    XCTAssertTrue(progress.waitForExistence(timeout: 10))

    let outcome = app.descendants(matching: .any)["import.csv.outcome"]
    let applied = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS '1 applied'"), object: outcome)
    XCTAssertEqual(XCTWaiter.wait(for: [applied], timeout: 10), .completed)
  }

  @MainActor
  func testCsvImportCanCancelFromProgressState() throws {
    let root = FileManager.default.temporaryDirectory
      .appendingPathComponent("TableRock-XCUITest-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let input = root.appendingPathComponent("cancel.csv")
    try "id,name\n2,Grace\n".write(to: input, atomically: true, encoding: .utf8)
    let app = launch(
      scenario: "success", root: root,
      environment: [
        "TABLEROCK_FIXTURE_DATA_MOVEMENT_UI": "1",
        "TABLEROCK_TEST_OPEN_FILE": input.path,
      ])

    openCsvImport(in: app)
    let stage = app.descendants(matching: .any)["import.csv.stage"]
    XCTAssertTrue(stage.waitForExistence(timeout: 10))
    stage.click()
    let apply = app.descendants(matching: .any)["import.csv.apply"]
    XCTAssertTrue(apply.waitForExistence(timeout: 10))
    let applyEnabled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "enabled == true"), object: apply)
    XCTAssertEqual(XCTWaiter.wait(for: [applyEnabled], timeout: 10), .completed)
    apply.click()
    let cancel = app.descendants(matching: .any)["import.csv.cancel"]
    XCTAssertTrue(cancel.waitForExistence(timeout: 10))
    cancel.click()

    let outcome = app.descendants(matching: .any)["import.csv.outcome"]
    let cancelled = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "value CONTAINS[c] 'cancelled'"), object: outcome)
    XCTAssertEqual(XCTWaiter.wait(for: [cancelled], timeout: 10), .completed)
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
    root providedRoot: URL? = nil,
    environment: [String: String] = [:]
  ) -> XCUIApplication {
    let app = XCUIApplication()
    let root =
      providedRoot
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
    addTeardownBlock { app.terminate() }
    relaunchApplication(app)
    assertWorkbenchWindowIsFrontmost(app)
    assertWorkbenchFitsVisibleScreen(app)
    return app
  }

  @MainActor
  private func assertWorkbenchFitsVisibleScreen(_ app: XCUIApplication) {
    let workbench = app.windows["window.workbench"]
    let frame = workbench.frame
    let tolerance: CGFloat = 1
    let fitsAvailableScreen = NSScreen.screens.contains { screen in
      frame.width <= screen.visibleFrame.width + tolerance
        && frame.height <= screen.visibleFrame.height + tolerance
    }
    XCTAssertTrue(
      fitsAvailableScreen,
      "Workbench frame \(frame) exceeds every visible screen frame"
    )
  }

  @MainActor
  private func assertWorkbenchWindowIsFrontmost(_ app: XCUIApplication) {
    let workbench = app.windows["window.workbench"]
    for _ in 0..<3 {
      app.activate()
      NSRunningApplication.runningApplications(withBundleIdentifier: "app.tablerock.TableRock")
        .max(by: { $0.processIdentifier < $1.processIdentifier })?
        .activate(options: [.activateAllWindows])
      if workbench.waitForExistence(timeout: 10) {
        return
      }
    }
    XCTFail("TableRock launched without a frontmost workbench window")
  }

  @MainActor
  private func connectTemporarily(_ app: XCUIApplication) {
    for attempt in 0..<2 {
      if attempt > 0 {
        restartApplication(app)
      }
      assertWorkbenchWindowIsFrontmost(app)
      let opens = app.buttons.matching(identifier: "connection.direct.open")
      if !opens.firstMatch.waitForExistence(timeout: 5) {
        continue
      }
      let open = opens.allElementsBoundByIndex.first(where: \.isHittable) ?? opens.firstMatch
      app.activate()
      open.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
      let connect = app.buttons["connection.direct.connect"]
      guard connect.waitForExistence(timeout: 5), connect.isEnabled else { continue }
      app.activate()
      let hittable = XCTNSPredicateExpectation(
        predicate: NSPredicate(format: "hittable == true"), object: connect)
      guard XCTWaiter.wait(for: [hittable], timeout: 3) == .completed else { continue }
      connect.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()

      let dismissed = XCTNSPredicateExpectation(
        predicate: NSPredicate(format: "exists == false"), object: connect)
      if XCTWaiter.wait(for: [dismissed], timeout: 3) != .completed,
        connect.exists,
        connect.isEnabled
      {
        app.activate()
        connect.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
      }
      let connected = XCTNSPredicateExpectation(
        predicate: NSPredicate(format: "exists == false"), object: connect)
      guard XCTWaiter.wait(for: [connected], timeout: 10) == .completed else { continue }
      let status = app.descendants(matching: .any)["connection.status"]
      guard status.waitForExistence(timeout: 5) else { continue }
      app.activate()
      let reachable = XCTNSPredicateExpectation(
        predicate: NSPredicate(format: "hittable == true"), object: status)
      if XCTWaiter.wait(for: [reachable], timeout: 3) == .completed {
        return
      }
    }
    XCTFail("Temporary connection did not reach a focused connected workbench")
  }

  @MainActor
  private func restartApplication(_ app: XCUIApplication) {
    relaunchApplication(app)
    assertWorkbenchWindowIsFrontmost(app)
  }

  @MainActor
  private func relaunchApplication(_ app: XCUIApplication) {
    if app.state != .notRunning {
      app.terminate()
      let stopped = XCTNSPredicateExpectation(
        predicate: NSPredicate(
          format: "state == %d", XCUIApplication.State.notRunning.rawValue),
        object: app)
      XCTAssertEqual(XCTWaiter.wait(for: [stopped], timeout: 5), .completed)
    }
    app.launch()
  }

  @MainActor
  private func openProfileEditor(in app: XCUIApplication) -> XCUIElement {
    let name = app.textFields["profile.editor.name"]
    for attempt in 0..<2 {
      if attempt > 0 {
        restartApplication(app)
      }
      assertWorkbenchWindowIsFrontmost(app)
      let add = app.buttons["profile.add"]
      guard add.waitForExistence(timeout: 5), add.isHittable else { continue }
      app.activate()
      add.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
      if name.waitForExistence(timeout: 5) {
        return name
      }
    }
    XCTFail("Profile editor did not open in a stable workbench window")
    return name
  }

  @MainActor
  private func activateAndClick(
    _ element: XCUIElement,
    in app: XCUIApplication,
    timeout: TimeInterval = 10
  ) {
    assertWorkbenchWindowIsFrontmost(app)
    app.activate()
    XCTAssertTrue(element.waitForExistence(timeout: timeout))
    let hittable = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "hittable == true"), object: element)
    XCTAssertEqual(XCTWaiter.wait(for: [hittable], timeout: timeout), .completed)
    element.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
  }

  @MainActor
  private func openCsvImport(in app: XCUIApplication) {
    activateAndClick(app.descendants(matching: .any)["object.actions"], in: app)
    clickOpenMenuItem(app.descendants(matching: .any)["import.csv.open"])
  }

  @MainActor
  private func clickOpenMenuItem(
    _ element: XCUIElement,
    timeout: TimeInterval = 10
  ) {
    XCTAssertTrue(element.waitForExistence(timeout: timeout))
    let hittable = XCTNSPredicateExpectation(
      predicate: NSPredicate(format: "hittable == true"), object: element)
    XCTAssertEqual(XCTWaiter.wait(for: [hittable], timeout: timeout), .completed)
    element.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
  }
}
