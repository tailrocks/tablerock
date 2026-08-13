import TableRockFeature
import XCTest

@testable import TableRock
@testable import TableRockPresentation

@MainActor
final class WorkbenchPresentationStoreScenarioTests: XCTestCase {
  func testImportErrorSummaryCopiesOnlyBoundedSafeRows() {
    let pasteboard = ImportErrorPasteboard()
    let model = WorkbenchPresentationStore(
      client: ScriptedWorkbenchBackend(scenario: "success"),
      dependencies: AppDependencies(pasteboard: pasteboard))
    model.csvImportProgress = WorkbenchCSVImportProgress(
      operationId: Data(repeating: 1, count: 16), phase: "partial",
      completedRows: 5, totalRows: 10, appliedRows: 4, conflictRows: 0,
      failedRows: 1, errors: ["row 6: apply failed"], errorsTruncated: true,
      summary: "4 applied · 1 failed")

    model.copyCsvImportErrors()

    XCTAssertEqual(pasteboard.values, ["row 6: apply failed\n… additional errors omitted"])
    XCTAssertEqual(model.csvImportErrorCopyOutcome, "Copied 1 import errors")
  }

  func testTestFilePanelsConfineOpenAndSavePathsToIsolatedRoot() throws {
    let base = FileManager.default.temporaryDirectory
      .appendingPathComponent("TableRock-FilePanels-\(UUID().uuidString)", isDirectory: true)
    let root = base.appendingPathComponent("root", isDirectory: true)
    let outside = base.appendingPathComponent("outside", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: base) }

    let insideOpen = root.appendingPathComponent("input.csv")
    let insideSave = root.appendingPathComponent("output.csv")
    let escape = root.appendingPathComponent("escape", isDirectory: true)
    try FileManager.default.createSymbolicLink(at: escape, withDestinationURL: outside)
    let escapedSave = escape.appendingPathComponent("escaped.csv")
    let request = AppFilePanelRequest(
      title: "Fixture", prompt: "Choose", allowedExtensions: ["csv"])

    let allowed = TestFilePanelPort(
      root: root, openPath: insideOpen.path, savePath: insideSave.path)
    XCTAssertEqual(allowed.chooseOpenFile(request), insideOpen)
    XCTAssertEqual(allowed.chooseSaveFile(request), insideSave)

    let rejected = TestFilePanelPort(
      root: root, openPath: outside.appendingPathComponent("input.csv").path,
      savePath: escapedSave.path)
    XCTAssertNil(rejected.chooseOpenFile(request))
    XCTAssertNil(rejected.chooseSaveFile(request))
  }

  func testScriptedProfileCreatePersistsForInteractionTests() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let draft = WorkbenchProfileDraft(
      idBytes: nil, revision: 0, engine: "postgresql", name: "Created fixture",
      group: "", environment: "testing", host: "127.0.0.1", port: "5432",
      database: "postgres", username: "postgres", passwordSource: "prompt",
      passwordValue: "", passwordReference: nil, hasStoredPassword: false,
      plaintextAcknowledged: false, tlsMode: "verify_full", safetyMode: "confirm_writes")

    let id = try await backend.saveProfile(draft)
    let profiles = try await backend.listProfiles()
    let stored = try await backend.profileDraft(id: id)

    XCTAssertEqual(id, Data(repeating: 9, count: 16))
    XCTAssertEqual(profiles.map(\.name), ["Created fixture"])
    XCTAssertEqual(stored.idBytes, id)
    XCTAssertEqual(stored.revision, 1)
    XCTAssertEqual(stored.passwordValue, "")
  }

  func testScriptedConnectionHealthAndCatalogAreDeterministic() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let session = try await backend.open(
      params: WorkbenchOpenParams(
        engine: "postgresql", host: "127.0.0.1", port: 5432,
        database: "postgres", user: "postgres", password: "", tlsMode: "off"))

    let health = try await backend.checkHealth(session: session)
    let catalog = try await backend.refreshCatalog(session: session, parentNodeId: nil)

    XCTAssertTrue(health.serverReachable)
    XCTAssertEqual(health.state, "healthy")
    XCTAssertEqual(catalog.map(\.name), ["public", "customers", "fixture_table"])
    XCTAssertEqual(catalog[1].parentIdBytes, catalog[0].idBytes)
    XCTAssertEqual(catalog[2].parentIdBytes, catalog[0].idBytes)
  }

  func testSQLiteCatalogTableOpensAsObjectTab() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)
    model.formEngine = "sqlite"
    await model.connectByParams()
    let table = WorkbenchCatalogNode(
      idBytes: Data(repeating: 7, count: 16),
      parentIdBytes: Data(repeating: 6, count: 16),
      depth: 1, name: "artists", kind: "sqlite_table",
      childrenState: "unrequested", expandable: true)
    model.catalogSnapshot = [table]

    await model.openCatalogObject(nodeId: table.idBytes)

    XCTAssertEqual(model.activeObjectTab?.title, "artists")
    XCTAssertEqual(model.activeObjectTab?.kind, "sqlite_table")
    XCTAssertNotNil(model.activeObjectTab?.resultTable)
    XCTAssertNil(model.profileActionError)
  }

  func testPostgresActivityUsesTypedRowsAndConfirmedSignalOutcome() async {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()
    await model.showPostgresActivity()

    XCTAssertTrue(model.postgresActivityPresented)
    XCTAssertEqual(model.postgresActivityRows.map(\.pid), [4242])
    XCTAssertEqual(model.postgresActivityRows[0].queryPreview, "SELECT pg_sleep(30)")
    await model.signalPostgresBackend(kind: "cancel", pid: 4242)
    XCTAssertEqual(model.postgresActivityOutcome, "Cancel acknowledged for PID 4242")
    XCTAssertNil(model.postgresActivityError)
  }

  func testPostgresRelationshipsShowCycleAndOpenRelatedTarget() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let table = try XCTUnwrap(model.catalogSnapshot?.last)
    await model.openCatalogObject(nodeId: table.idBytes)
    await model.showPostgresRelationships()

    XCTAssertTrue(model.postgresRelationshipsPresented)
    XCTAssertEqual(model.postgresRelationshipSnapshot?.edges.count, 2)
    XCTAssertEqual(
      model.postgresRelationshipSnapshot?.edges.filter {
        $0.fromTable == $0.toTable
      }.count, 1)
    XCTAssertNil(model.postgresRelationshipsError)
  }

  func testRelationContinuumLoadsRelatedRowsThroughRustOwnedBrowse() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let source = try XCTUnwrap(
      model.catalogSnapshot?.first(where: { $0.name == "fixture_table" }))
    await model.openCatalogObject(nodeId: source.idBytes)
    model.selectedCell = NativeCellSelection(row: 0, column: 1)

    await model.openRelationContinuumFromSelection()

    let continuum = try XCTUnwrap(model.relationContinuum)
    XCTAssertEqual(continuum.edgeTitle, "fixture_table.customer_id → customers.id")
    XCTAssertEqual(continuum.directionWord, "outbound")
    XCTAssertEqual(continuum.relatedSchema, "public")
    XCTAssertEqual(continuum.relatedTable, "customers")
    XCTAssertEqual(continuum.relatedColumn, "id")
    XCTAssertEqual(continuum.columns, ["id", "name"])
    XCTAssertEqual(continuum.rows, [["42", "Ada"]])
    XCTAssertEqual(continuum.statusWord, "READY")
    XCTAssertNil(model.relationContinuumError)

    model.selectCell(row: 0, column: 0)
    XCTAssertNil(model.relationContinuum)
  }

  func testSelectedRowUpdateStagesReviewsRevokesAndApplies() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let table = try XCTUnwrap(
      model.catalogSnapshot?.first(where: { $0.name == "fixture_table" }))
    await model.openCatalogObject(nodeId: table.idBytes)
    model.selectCell(row: 0, column: 1)

    XCTAssertTrue(model.canEditSelectedRow)
    model.showSelectedRowEditor()
    let draft = try XCTUnwrap(model.rowEditDraft)
    XCTAssertEqual(draft.fields.map(\.column), ["customer_id"])
    XCTAssertEqual(draft.fields[0].original, "42")
    draft.fields[0].value = "43"

    await model.stageRowUpdate()
    XCTAssertEqual(model.activeObjectTab?.mutationReview?.lines[0].parameters, ["43", "1001"])
    XCTAssertEqual(model.changeLedgerEntryCount, 1)

    await model.backToRowEditor()
    XCTAssertNil(model.activeObjectTab?.mutationReview)
    XCTAssertNotNil(model.rowEditDraft)
    await model.stageRowUpdate()
    await model.applyRowUpdate()

    XCTAssertNil(model.rowEditDraft)
    XCTAssertNil(model.activeObjectTab?.mutationReview)
    XCTAssertEqual(model.activeObjectTab?.mutationOutcome, "Applied 1 update in one transaction.")
    XCTAssertEqual(model.changeLedgerEntryCount, 0)
  }

  func testPostgresRolesUseTypedMembershipAndPrivilegeSnapshot() async {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()
    await model.showPostgresRoles()

    XCTAssertTrue(model.postgresRolesPresented)
    XCTAssertEqual(model.postgresRoleSnapshot?.currentUser, "fixture")
    XCTAssertEqual(model.postgresRoleSnapshot?.effectiveRoles, ["fixture", "reader"])
    XCTAssertEqual(model.postgresRoleSnapshot?.memberships.first?.role, "reader")
    XCTAssertNil(model.postgresRolesError)

    model.postgresRoleChangeRole = "reader"
    model.postgresRoleChangeSubject = "analyst"
    await model.stagePostgresRoleChange()
    XCTAssertNotNil(model.postgresRoleChangeReview)
    await model.applyPostgresRoleChange()
    XCTAssertEqual(model.postgresRoleChangeOutcome, "Role change applied")
    XCTAssertNil(model.postgresRoleChangeReview)
  }

  func testRedisPubSubSurfacesMessagesGapsAndCancellation() async {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)
    model.formEngine = "redis"

    await model.connectByParams()
    model.showRedisSubscription()
    model.redisSubscriptionSelector = "updates:*"
    model.redisSubscriptionPattern = true
    await model.startRedisSubscription()

    XCTAssertTrue(model.redisSubscriptionPresented)
    XCTAssertTrue(model.redisSubscriptionIsActive)
    XCTAssertEqual(model.redisSubscriptionStatus?.messages, ["updates:users · fixture message"])
    XCTAssertEqual(model.redisSubscriptionStatus?.discontinuities, 1)
    await model.cancelRedisSubscription()
    XCTAssertEqual(model.redisSubscriptionStatus?.phase, "cancelled")
    XCTAssertFalse(model.redisSubscriptionIsActive)
    XCTAssertNil(model.redisSubscriptionError)
  }

  func testStructureChangeFreezesPreviewAndConsumesReview() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let table = try XCTUnwrap(model.catalogSnapshot?.last)
    await model.openCatalogObject(nodeId: table.idBytes)
    await model.loadObjectStructure()
    model.showDdlChange()
    model.ddlChangeKind = "add_column"
    model.ddlChangeObjectName = "reviewed_column"
    model.ddlChangeDefinition = "text"

    await model.stageDdlChange()

    XCTAssertTrue(model.ddlChangePresented)
    XCTAssertTrue(model.ddlChangeReview?.preview.contains("reviewed_column") == true)
    XCTAssertFalse(model.ddlChangeReview?.destructive ?? true)
    XCTAssertTrue(model.ddlChangeReview?.rollbackSummary.contains("does not automatically") == true)
    await model.applyDdlChange()
    XCTAssertNil(model.ddlChangeReview)
    XCTAssertEqual(model.ddlChangeOutcome, "Structure change applied")
    XCTAssertNil(model.ddlChangeError)
  }

  func testFindReplaceHonorsModesScopeAndZeroWidthRegex() {
    let model = WorkbenchPresentationStore(client: ScriptedWorkbenchBackend(scenario: "success"))

    model.queryText = "cat scatter CAT"
    model.queryEditorSelection = NSRange(location: 0, length: 3)
    model.showFindReplace()
    model.findPattern = "cat"
    model.findReplacement = "dog"
    model.findMode = "whole_word"
    model.replaceAllEditorMatches()
    XCTAssertEqual(model.queryText, "dog scatter dog")
    XCTAssertEqual(model.findStatus, "Replaced 2 matches")

    model.queryText = "one one one"
    model.queryEditorSelection = NSRange(location: 4, length: 3)
    model.showFindReplace()
    model.setFindScope("selection")
    model.findPattern = "one"
    model.findReplacement = "two"
    model.replaceAllEditorMatches()
    XCTAssertEqual(model.queryText, "one two one")
    XCTAssertEqual(model.queryEditorSelection, NSRange(location: 4, length: 3))

    model.queryText = "café"
    model.queryEditorSelection = NSRange(location: 0, length: 0)
    model.showFindReplace()
    model.findMode = "regular_expression"
    model.findPattern = "(?=é)"
    model.findReplacement = "!"
    model.replaceAllEditorMatches()
    XCTAssertEqual(model.queryText, "caf!é")
    XCTAssertEqual(model.findStatus, "Replaced 1 match")
  }

  func testNamedQueryParametersRequireTypedSheetBeforeRun() async {
    let model = WorkbenchPresentationStore(client: ScriptedWorkbenchBackend(scenario: "success"))
    await model.connectByParams()
    model.queryText = "SELECT :id::int"

    await model.runQuery()

    XCTAssertTrue(model.queryParametersPresented)
    XCTAssertEqual(model.queryParameterBindings.map(\.name), ["id"])
    XCTAssertNil(model.resultTable)
    model.queryParameterBindings[0].kind = "integer"
    model.queryParameterBindings[0].value = "42 OR 1=1"
    await model.runParameterizedQuery()
    XCTAssertTrue(model.queryParametersPresented)
    XCTAssertNotNil(model.queryParameterError)

    model.queryParameterBindings[0].value = "42"
    await model.runParameterizedQuery()
    XCTAssertFalse(model.queryParametersPresented)
    XCTAssertEqual(model.querySummary, "write ok · ok")
  }

  func testTableOperationRequiresFrozenTargetAndExactConfirmation() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)
    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let table = try XCTUnwrap(model.catalogSnapshot?.last)
    await model.openCatalogObject(nodeId: table.idBytes)
    model.showTableOperation()

    await model.stageTableOperation()

    let review = try XCTUnwrap(model.tableOperationReview)
    XCTAssertTrue(review.destructive)
    XCTAssertEqual(review.confirmation, "fixture_table")
    XCTAssertTrue(review.preview.contains("fixture_table"))
    model.tableOperationConfirmation = "wrong"
    await model.applyTableOperation()
    XCTAssertNotNil(model.tableOperationReview)
    XCTAssertNotNil(model.tableOperationError)

    model.tableOperationConfirmation = review.confirmation
    await model.applyTableOperation()
    XCTAssertNil(model.tableOperationReview)
    XCTAssertEqual(model.tableOperationStatus?.phase, "succeeded")
    XCTAssertEqual(model.tableOperationStatus?.cancellable, false)
    XCTAssertEqual(model.tableOperationOutcome, "truncate completed")
  }

  func testPostgresBackupUsesProbeReviewAndSupervisedStatus() async {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()
    await model.showPostgresTools()
    model.postgresToolFileUrl = URL(fileURLWithPath: "/tmp/tablerock-fixture.dump")
    model.requestStartPostgresTool()

    XCTAssertTrue(model.postgresToolsPresented)
    XCTAssertEqual(model.postgresToolProbe?.version, "PostgreSQL 18.4")
    XCTAssertTrue(model.postgresToolReviewRequested)
    await model.startPostgresTool()
    XCTAssertEqual(model.postgresToolStatus?.phase, "succeeded")
    XCTAssertEqual(model.postgresToolStatus?.kind, "dump")
    XCTAssertNil(model.postgresToolError)
  }

  func testDirtyAndRunningTabsRequireExplicitResolution() {
    let model = WorkbenchPresentationStore()
    model.addQueryTab()
    let tab = model.queryTabs.last!

    tab.statementText = "SELECT 2;"
    model.requestCloseQueryTab(tab)
    XCTAssertTrue(model.pendingQueryTabClose === tab)
    XCTAssertEqual(model.queryTabs.count, 2)

    model.pendingQueryTabClose = nil
    tab.isRunning = true
    model.requestCloseQueryTab(tab)
    XCTAssertNil(model.pendingQueryTabClose)
    XCTAssertEqual(model.queryTabs.count, 2)
    XCTAssertEqual(model.profileActionError, "Cancel the running query before closing its tab")
  }

  func testCorruptRestorationFailsClosed() async {
    let backend = ScriptedWorkbenchBackend(scenario: "restoration-corrupt")
    let model = WorkbenchPresentationStore(client: backend)

    await model.initialize()

    XCTAssertEqual(model.profileActionError, "Restored workspace intent was invalid")
    XCTAssertNil(model.profileActionOutcome)
    XCTAssertEqual(model.queryTabs.count, 1)
    XCTAssertEqual(model.queryText, "SELECT 1;")
  }

  func testScriptedFailureMatrixRejectsNamedFaults() async throws {
    let id = Data(repeating: 1, count: 16)
    let connection = ScriptedWorkbenchBackend(scenario: "connection-failure")
    let authentication = ScriptedWorkbenchBackend(scenario: "authentication-failure")
    let staleRevision = ScriptedWorkbenchBackend(scenario: "stale-result-revision")
    let staleEvent = ScriptedWorkbenchBackend(scenario: "stale-event")
    let cursor = ScriptedWorkbenchBackend(scenario: "cursor-resync")
    let columns = ScriptedWorkbenchBackend(scenario: "mismatched-next-page-columns")

    await XCTAssertThrowsErrorAsync {
      try await connection.openProfile(id: id, secretOverride: nil)
    }
    await XCTAssertThrowsErrorAsync {
      try await authentication.openProfile(id: id, secretOverride: nil)
    }
    await XCTAssertThrowsErrorAsync {
      try await staleRevision.fetchPage(resultId: id, startRow: 0, revision: 1)
    }
    await XCTAssertThrowsErrorAsync { try await staleEvent.finish(operationId: id) }
    await XCTAssertThrowsErrorAsync { try await cursor.finish(operationId: id) }
    await XCTAssertThrowsErrorAsync {
      try await columns.fetchPage(resultId: id, startRow: 0, revision: 1)
    }
  }

  func testScriptedDirectConnectionOpensWorkbench() async {
    let backend = ScriptedWorkbenchBackend(scenario: "slow-until-cancelled")
    let model = WorkbenchPresentationStore(client: backend)

    await model.connectByParams()

    XCTAssertNotNil(model.sessionHex)
    XCTAssertNil(model.connectError)
  }

  func testScriptedCancellationPublishesSemanticOutcome() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "slow-until-cancelled")
    let model = WorkbenchPresentationStore(client: backend)
    model.sessionData = Data(repeating: 1, count: 16)

    let query = Task { await model.runQuery() }
    for _ in 0..<100 where !model.isRunning {
      try await Task.sleep(for: .milliseconds(10))
    }
    XCTAssertTrue(model.isRunning)

    await model.cancel()
    await query.value

    XCTAssertEqual(model.cancelOutcome, "Requested")
    XCTAssertFalse(model.isRunning)
  }

  func testHistoryFailureRemainsVisibleAfterSuccessfulOperation() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "history-failure-after-page")
    let projection = try await backend.finish(operationId: Data(repeating: 1, count: 16))

    XCTAssertEqual(projection.outcome, "ok")
    XCTAssertTrue(projection.historyFailed)
  }

  func testWindowsShareBackendButOwnPresentationState() {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let first = WorkbenchPresentationStore(client: backend)
    let second = WorkbenchPresentationStore(client: backend)

    first.queryText = "SELECT first;"
    second.queryText = "SELECT second;"

    XCTAssertNotEqual(first.windowId, second.windowId)
    XCTAssertEqual(first.queryText, "SELECT first;")
    XCTAssertEqual(second.queryText, "SELECT second;")
    XCTAssertFalse(first.queryTabs[0] === second.queryTabs[0])
  }
}

@MainActor
private final class ImportErrorPasteboard: AppPasteboardPort {
  var values: [String] = []
  func write(_ representations: [AppPasteboardRepresentation]) throws {
    values.append(contentsOf: representations.map(\.value))
  }
}

@MainActor
private func XCTAssertThrowsErrorAsync<T>(
  _ expression: () async throws -> T,
  file: StaticString = #filePath,
  line: UInt = #line
) async {
  do {
    _ = try await expression()
    XCTFail("Expected error", file: file, line: line)
  } catch {}
}
