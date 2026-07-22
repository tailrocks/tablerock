import AppKit
import TableRockFeature
import XCTest

@testable import TableRock

@MainActor
final class BridgeModelScenarioTests: XCTestCase {
  func testVimTextViewModeTransitionsMotionsDeleteAndUndo() {
    let editor = VimTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 200))
    editor.string = "alpha\nbeta\n"
    editor.vimEnabled = true
    editor.setSelectedRange(NSRange(location: 6, length: 0))

    editor.keyDown(with: keyEvent("\u{1b}", keyCode: 53))
    XCTAssertEqual(editor.vimMode, "normal")
    editor.keyDown(with: keyEvent("l", keyCode: 37))
    XCTAssertEqual(editor.selectedRange().location, 7)
    editor.keyDown(with: keyEvent("d", keyCode: 2))
    XCTAssertEqual(editor.string, "alpha\n")
    editor.keyDown(with: keyEvent("u", keyCode: 32))
    XCTAssertEqual(editor.string, "alpha\nbeta\n")
    editor.keyDown(with: keyEvent("i", keyCode: 34))
    XCTAssertEqual(editor.vimMode, "insert")
  }

  func testVimEscapeDoesNotStealMarkedTextComposition() {
    let editor = VimTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 200))
    editor.vimEnabled = true
    editor.setMarkedText(
      "あ", selectedRange: NSRange(location: 1, length: 0),
      replacementRange: NSRange(location: NSNotFound, length: 0))
    XCTAssertTrue(editor.hasMarkedText())

    editor.keyDown(with: keyEvent("\u{1b}", keyCode: 53))

    XCTAssertEqual(editor.vimMode, "insert")
  }

  func testImportErrorSummaryCopiesOnlyBoundedSafeRows() {
    let pasteboard = ImportErrorPasteboard()
    let model = BridgeModel(
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
    XCTAssertEqual(catalog.map(\.name), ["public", "fixture_table"])
    XCTAssertEqual(catalog[1].parentIdBytes, catalog[0].idBytes)
  }

  func testPostgresActivityUsesTypedRowsAndConfirmedSignalOutcome() async {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = BridgeModel(client: backend)

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
    let model = BridgeModel(client: backend)

    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let table = try XCTUnwrap(model.catalogSnapshot?.last)
    let nodeKey = table.idBytes.map { String(format: "%02x", $0) }.joined()
    await model.openCatalogObject(nodeKey: nodeKey)
    await model.showPostgresRelationships()

    XCTAssertTrue(model.postgresRelationshipsPresented)
    XCTAssertEqual(model.postgresRelationshipSnapshot?.edges.count, 2)
    XCTAssertEqual(
      model.postgresRelationshipSnapshot?.edges.filter {
        $0.fromTable == $0.toTable
      }.count, 1)
    XCTAssertNil(model.postgresRelationshipsError)
  }

  func testPostgresRolesUseTypedMembershipAndPrivilegeSnapshot() async {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = BridgeModel(client: backend)

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
    let model = BridgeModel(client: backend)
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
    let model = BridgeModel(client: backend)

    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let table = try XCTUnwrap(model.catalogSnapshot?.last)
    let nodeKey = table.idBytes.map { String(format: "%02x", $0) }.joined()
    await model.openCatalogObject(nodeKey: nodeKey)
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
    let model = BridgeModel(client: ScriptedWorkbenchBackend(scenario: "success"))

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
    let model = BridgeModel(client: ScriptedWorkbenchBackend(scenario: "success"))
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
    let model = BridgeModel(client: backend)
    await model.connectByParams()
    let session = Data(repeating: 1, count: 16)
    model.catalogSnapshot = try await backend.refreshCatalog(session: session, parentNodeId: nil)
    let table = try XCTUnwrap(model.catalogSnapshot?.last)
    let nodeKey = table.idBytes.map { String(format: "%02x", $0) }.joined()
    await model.openCatalogObject(nodeKey: nodeKey)
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
    XCTAssertEqual(model.tableOperationOutcome, "Table operation applied")
  }

  func testPostgresBackupUsesProbeReviewAndSupervisedStatus() async {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let model = BridgeModel(client: backend)

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
    let model = BridgeModel()
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
    let model = BridgeModel(client: backend)

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
    let model = BridgeModel(client: backend)

    await model.connectByParams()

    XCTAssertNotNil(model.sessionHex)
    XCTAssertNil(model.connectError)
  }

  func testScriptedCancellationPublishesSemanticOutcome() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "slow-until-cancelled")
    let model = BridgeModel(client: backend)
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
    let first = BridgeModel(client: backend)
    let second = BridgeModel(client: backend)

    first.queryText = "SELECT first;"
    second.queryText = "SELECT second;"

    XCTAssertNotEqual(first.windowId, second.windowId)
    XCTAssertEqual(first.queryText, "SELECT first;")
    XCTAssertEqual(second.queryText, "SELECT second;")
    XCTAssertFalse(first.queryTabs[0] === second.queryTabs[0])
  }
}

@MainActor
private func keyEvent(_ characters: String, keyCode: UInt16) -> NSEvent {
  NSEvent.keyEvent(
    with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0,
    windowNumber: 0, context: nil, characters: characters,
    charactersIgnoringModifiers: characters, isARepeat: false, keyCode: keyCode)!
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
