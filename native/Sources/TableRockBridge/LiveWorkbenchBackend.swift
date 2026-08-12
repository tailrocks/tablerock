import Foundation
import TableRockFeature

private actor LiveWorkbenchBackend: WorkbenchBackend {
  private let bridge: TableRockBridge
  private var eventCursor: UInt64 = 0

  init(persistencePath: String) throws {
    let bridge = TableRockBridge.create()
    try bridge.ensureRuntime()
    try bridge.configurePersistence(path: persistencePath)
    self.bridge = bridge
  }

  func listProfiles() throws -> [WorkbenchProfileItem] {
    try bridge.listProfiles().map(\.workbench)
  }
  func searchProfiles(_ search: String?) throws -> [WorkbenchProfileItem] {
    try bridge.searchProfiles(search: search).map(\.workbench)
  }
  func profileDraft(id: Data) throws -> WorkbenchProfileDraft {
    try bridge.getProfileDraft(profileId: id).workbench
  }
  func parseConnectionUrl(_ input: String) throws -> WorkbenchProfileDraft {
    try bridge.parseConnectionUrlDraft(input: input).workbench
  }
  func saveProfile(_ draft: WorkbenchProfileDraft) throws -> Data {
    try bridge.saveProfile(draft: draft.bridgeRecord)
  }

  func prepareSampleDatabase(dataRoot: String) throws -> WorkbenchProfileDraft {
    try bridge.prepareSampleDatabase(dataRoot: dataRoot).workbench
  }
  func deleteProfile(id: Data, revision: UInt64) throws {
    try bridge.deleteProfile(profileId: id, expectedRevision: revision)
  }
  func testProfile(id: Data, secretOverride: Data?) throws -> WorkbenchConnectionTestReport {
    try bridge.testProfileWithSecret(profileId: id, secretOverride: secretOverride).workbench
  }
  func listProfileGroups() throws -> [WorkbenchProfileGroup] {
    try bridge.listProfileGroups().map(\.workbench)
  }
  func createProfileGroup(_ name: String) throws { try bridge.createProfileGroup(name: name) }
  func renameProfileGroup(_ oldName: String, _ newName: String) throws -> UInt32 {
    try bridge.renameProfileGroup(oldName: oldName, newName: newName)
  }
  func deleteProfileGroup(_ name: String) throws -> UInt32 {
    try bridge.deleteProfileGroup(name: name)
  }
  func setGroupAlphabetical(_ name: String, _ alphabetical: Bool) throws {
    try bridge.setProfileGroupAlphabetical(name: name, alphabetical: alphabetical)
  }
  func listHistory(_ search: String?) throws -> [WorkbenchHistoryItem] {
    try bridge.listHistory(search: search, limit: 100).map(\.workbench)
  }
  func setHistoryRetention(_ retention: String) throws {
    try bridge.setHistoryRetention(retention: retention)
  }
  func historyRetention() throws -> String { try bridge.historyRetention() }
  func listSavedQueries(engine: String?, search: String?) throws -> [WorkbenchSavedQueryItem] {
    try bridge.listSavedQueries(engine: engine, search: search).map(\.workbench)
  }
  func saveQuery(name: String, engine: String, statement: String) throws -> Int64 {
    try bridge.saveQuery(name: name, engine: engine, statementText: statement)
  }
  func deleteSavedQuery(_ id: Int64) throws -> Bool {
    try bridge.deleteSavedQuery(queryId: id)
  }
  func readSqlFile(path: String) throws -> WorkbenchSQLFile {
    try bridge.readSqlFile(path: path).workbench
  }
  func writeSqlFile(
    path: String,
    statement: String,
    expectedModifiedNanos: UInt64?,
    expectedLength: UInt64?,
    overwriteExternalChange: Bool
  ) throws -> WorkbenchSQLFile {
    try bridge.writeSqlFile(
      path: path,
      statementText: statement,
      expectedModifiedNanos: expectedModifiedNanos,
      expectedLen: expectedLength,
      overwriteExternalChange: overwriteExternalChange
    ).workbench
  }
  func putSessionIntent(profileId: Data, intent: WorkbenchSessionIntent) throws {
    try bridge.putSessionIntent(profileId: profileId, intent: intent.bridgeRecord)
  }
  func sessionIntent(profileId: Data) throws -> WorkbenchSessionIntent? {
    try bridge.getSessionIntent(profileId: profileId)?.workbench
  }
  func deleteSessionIntent(profileId: Data) throws {
    try bridge.deleteSessionIntent(profileId: profileId)
  }
  func putNativeWindowIntent(
    windowId: String, profileId: Data, intent: WorkbenchSessionIntent
  ) throws {
    try bridge.putNativeWindowIntent(
      windowId: windowId, profileId: profileId, intent: intent.bridgeRecord
    )
  }
  func nativeWindowIntent(windowId: String) throws -> WorkbenchNativeWindowIntent? {
    try bridge.getNativeWindowIntent(windowId: windowId)?.workbench
  }
  func deleteNativeWindowIntent(windowId: String) throws {
    try bridge.deleteNativeWindowIntent(windowId: windowId)
  }
  func setProfileFavorite(_ item: WorkbenchProfileItem, _ favorite: Bool) throws {
    try bridge.setProfileFavorite(
      profileId: item.idBytes,
      expectedRevision: item.revision,
      favorite: favorite
    )
  }
  func reorderProfiles(group: String?, profiles: [WorkbenchProfileItem]) throws {
    try bridge.reorderProfiles(
      group: group,
      ordered: profiles.map {
        BridgeProfileOrderItem(idBytes: $0.idBytes, expectedRevision: $0.revision)
      }
    )
  }
  func open(params: WorkbenchOpenParams) throws -> Data {
    try bridge.open(params: params.bridgeRecord)
  }
  func openProfile(id: Data, secretOverride: Data?) throws -> Data {
    try bridge.openProfileWithSecret(profileId: id, secretOverride: secretOverride)
  }
  func disconnect(session: Data) throws { try bridge.disconnect(sessionId: session) }
  func checkHealth(session: Data) throws -> WorkbenchSessionHealth {
    try bridge.checkSessionHealth(sessionId: session).workbench
  }
  func planReconnect(
    session: Data, attempt: UInt32, authenticationStopped: Bool
  ) throws -> WorkbenchReconnectPlan {
    try bridge.planSessionReconnect(
      sessionId: session, attempt: attempt,
      authenticationStopped: authenticationStopped
    ).workbench
  }
  func reconnect(session: Data, secretOverride: Data? = nil) throws -> WorkbenchReconnectAttempt {
    try bridge.reconnectSavedSessionWithSecret(
      sessionId: session, secretOverride: secretOverride
    ).workbench
  }
  func refreshCatalog(session: Data, parentNodeId: Data?) throws -> [WorkbenchCatalogNode] {
    try bridge.refreshCatalog(sessionId: session, parentNodeId: parentNodeId).map(\.workbench)
  }
  func submitCatalogBrowse(
    session: Data, nodeId: Data, sort: [WorkbenchBrowseSort], filters: [WorkbenchBrowseFilter],
    rawWhere: String?
  ) throws -> Data {
    try bridge.submitCatalogBrowseWithPlan(
      sessionId: session, catalogNodeId: nodeId,
      sort: sort.map {
        BridgeBrowseSort(column: $0.column, direction: $0.descending ? "desc" : "asc")
      },
      filters: filters.map {
        BridgeBrowseFilter(column: $0.column, operator: $0.operatorName, value: $0.value)
      }, rawWhere: rawWhere, rowCount: 500
    )
  }
  func listCatalogFilterPresets(session: Data, nodeId: Data) throws
    -> [WorkbenchSavedFilterPreset]
  {
    try bridge.listCatalogFilterPresets(sessionId: session, catalogNodeId: nodeId).map {
      WorkbenchSavedFilterPreset(
        name: $0.name,
        filters: $0.filters.map {
          WorkbenchBrowseFilter(
            column: $0.column, operatorName: $0.operator, value: $0.value)
        }, rawWhere: $0.rawWhere)
    }
  }
  func saveCatalogFilterPreset(
    session: Data, nodeId: Data, preset: WorkbenchSavedFilterPreset
  ) throws {
    try bridge.saveCatalogFilterPreset(
      sessionId: session, catalogNodeId: nodeId,
      preset: BridgeSavedFilterPreset(
        name: preset.name,
        filters: preset.filters.map {
          BridgeBrowseFilter(column: $0.column, operator: $0.operatorName, value: $0.value)
        }, rawWhere: preset.rawWhere))
  }
  func submit(session: Data, intent: String, statement: String?) throws -> Data {
    try bridge.submit(
      spec: SubmitSpec(
        intent: intent, sessionId: session, statement: statement,
        resultId: nil, startRow: nil, rowCount: 500, expectedRevision: 0
      ))
  }

  func inspectNamedParameters(statement: String) throws -> [String] {
    try bridge.inspectNamedParameters(statement: statement).names
  }

  func submitNamed(
    session: Data, statement: String, bindings: [WorkbenchQueryParameter]
  ) throws -> Data {
    try bridge.submitNamed(
      spec: SubmitSpec(
        intent: "execute", sessionId: session, statement: statement,
        resultId: nil, startRow: nil, rowCount: 500, expectedRevision: 0),
      bindings: bindings.map {
        BridgeQueryParameter(
          name: $0.name, kind: $0.kind, value: $0.kind == "null" ? nil : $0.value)
      })
  }

  func finish(operationId: Data) async throws -> WorkbenchOperation {
    let bridge = bridge
    try await Task.detached { try bridge.pump(operationId: operationId) }.value
    var page: Data?
    var outcome: String?
    var historyFailed = false
    for _ in 0..<64 {
      let batch = try bridge.nextEvents(cursor: eventCursor, maximum: 64)
      eventCursor = batch.nextCursor
      for event in batch.events where event.operationId == operationId {
        if event.kind == "page" { page = event.pageBytes }
        if event.kind == "history_failed" { historyFailed = true }
        if event.kind == "terminal" { outcome = event.outcome ?? "ok" }
      }
      if outcome != nil || batch.events.isEmpty { break }
    }
    guard let page else {
      return WorkbenchOperation(
        table: nil, envelope: nil, outcome: outcome, historyFailed: historyFailed
      )
    }
    let decoded = try await Task.detached {
      (try PageV1.decodeTable(page), try PageV1.decodeEnvelope(page))
    }.value
    return WorkbenchOperation(
      table: decoded.0.workbench, envelope: decoded.1.workbench,
      outcome: outcome, historyFailed: historyFailed
    )
  }

  func cancel(operationId: Data) throws -> WorkbenchCancelOutcome {
    try bridge.cancel(operationId: operationId).workbench
  }

  func fetchPage(resultId: Data, startRow: UInt64, revision: UInt64) async throws
    -> (WorkbenchTable, WorkbenchPageEnvelope)
  {
    let bytes = try bridge.fetchPage(
      resultId: resultId, startRow: startRow, revision: revision)
    return try await Task.detached {
      (try PageV1.decodeTable(bytes).workbench, try PageV1.decodeEnvelope(bytes).workbench)
    }.value
  }

  func formatResultCopy(
    resultId: Data, revision: UInt64, scope: String,
    row: UInt64?, column: UInt32?, format: String
  ) throws -> String {
    try bridge.formatResultCopy(
      resultId: resultId, revision: revision, scope: scope,
      row: row, column: column, format: format
    )
  }

  func exportLoadedResult(
    resultId: Data, revision: UInt64, format: String, path: String
  ) throws -> UInt64 {
    try bridge.exportLoadedResult(
      resultId: resultId, revision: revision, format: format, path: path
    )
  }

  func startStreamExport(sessionId: Data, statement: String, format: String, path: String) throws
    -> Data
  {
    try bridge.startStreamExport(
      request: BridgeStreamExportRequest(
        sessionId: sessionId, statement: statement, format: format, path: path))
  }

  func startCatalogStreamExport(
    resultId: Data, revision: UInt64, format: String, path: String
  ) throws -> Data {
    try bridge.startCatalogStreamExport(
      request: BridgeCatalogStreamExportRequest(
        resultId: resultId, revision: revision, format: format, path: path))
  }

  func streamExportProgress(operationId: Data) throws -> WorkbenchStreamExportProgress {
    try bridge.streamExportProgress(operationId: operationId).workbench
  }

  func cancelStreamExport(operationId: Data) throws -> Bool {
    try bridge.cancelStreamExport(operationId: operationId)
  }

  func dismissStreamExport(operationId: Data) throws -> Bool {
    try bridge.dismissStreamExport(operationId: operationId)
  }

  func exportSupportBundle(path: String) throws -> UInt64 {
    try bridge.exportSupportBundle(path: path)
  }

  func previewCsvImport(path: String) throws -> WorkbenchCSVImportPreview {
    try bridge.previewCsvImport(path: path).workbench
  }

  func stageCsvImport(
    sessionId: Data, catalogNodeId: Data, path: String,
    mappedColumns: [String], mappedTypes: [String], expectedFingerprint: String, nowMs: UInt64
  ) throws -> WorkbenchCSVImportReview {
    try bridge.stageCsvImport(
      request: BridgeCsvImportRequest(
        sessionId: sessionId, catalogNodeId: catalogNodeId, path: path,
        mappedColumns: mappedColumns, mappedTypes: mappedTypes,
        expectedFingerprint: expectedFingerprint, nowMs: nowMs)
    ).workbench
  }

  func startCsvImportApply(tokenId: Data, nowMs: UInt64, sessionId: Data) throws -> Data {
    try bridge.startCsvImportApply(tokenId: tokenId, nowMs: nowMs, sessionId: sessionId)
  }

  func csvImportProgress(operationId: Data) throws -> WorkbenchCSVImportProgress {
    try bridge.csvImportProgress(operationId: operationId).workbench
  }

  func cancelCsvImport(operationId: Data) throws -> Bool {
    try bridge.cancelCsvImport(operationId: operationId)
  }

  func dismissCsvImport(operationId: Data) throws -> Bool {
    try bridge.dismissCsvImport(operationId: operationId)
  }

  func relationStructure(sessionId: Data, catalogNodeId: Data) throws
    -> WorkbenchRelationStructure
  {
    try bridge.relationStructure(
      sessionId: sessionId, catalogNodeId: catalogNodeId
    ).workbench
  }
  func redisKeyView(
    sessionId: Data, catalogNodeId: Data, collectionSkip: UInt64
  ) throws -> WorkbenchRedisKeyView {
    try bridge.redisKeyView(
      sessionId: sessionId, catalogNodeId: catalogNodeId,
      collectionSkip: collectionSkip
    ).workbench
  }

  func redisOverview(sessionId: Data) throws -> WorkbenchRedisOverview {
    try bridge.redisOverview(sessionId: sessionId).workbench
  }

  func startRedisSubscription(sessionId: Data, selector: String, pattern: Bool) throws -> Data {
    try bridge.startRedisSubscription(
      sessionId: sessionId, selector: selector, pattern: pattern)
  }

  func redisSubscriptionStatus(operationId: Data) throws -> WorkbenchRedisSubscriptionStatus {
    try bridge.redisSubscriptionStatus(operationId: operationId).workbench
  }

  func cancelRedisSubscription(operationId: Data) throws -> Bool {
    try bridge.cancelRedisSubscription(operationId: operationId)
  }

  func stageDdlChange(
    sessionId: Data, catalogNodeId: Data, kind: String, objectName: String,
    definition: String, nowMs: UInt64
  ) throws -> WorkbenchDdlChangeReview {
    try bridge.stageDdlChange(
      request: BridgeDdlChangeRequest(
        sessionId: sessionId, catalogNodeId: catalogNodeId, kind: kind,
        objectName: objectName, definition: definition, nowMs: nowMs)
    ).workbench
  }

  func applyDdlChange(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool
  ) throws -> String {
    try bridge.applyDdlChange(
      tokenId: tokenId, sessionId: sessionId, nowMs: nowMs, confirmed: confirmed)
  }

  func revokeDdlChange(tokenId: Data) throws -> Bool {
    try bridge.revokeDdlChange(tokenId: tokenId)
  }

  func stageTableOperation(
    sessionId: Data, catalogNodeId: Data, kind: String, newName: String, nowMs: UInt64
  ) throws -> WorkbenchTableOperationReview {
    try bridge.stageTableOperation(
      request: BridgeTableOperationRequest(
        sessionId: sessionId, catalogNodeId: catalogNodeId, kind: kind,
        newName: newName, nowMs: nowMs)
    ).workbench
  }

  func startTableOperation(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmation: String
  ) throws -> Data {
    try bridge.startTableOperation(
      tokenId: tokenId, sessionId: sessionId, nowMs: nowMs, confirmation: confirmation)
  }

  func tableOperationStatus(operationId: Data) throws -> WorkbenchTableOperationStatus {
    try bridge.tableOperationStatus(operationId: operationId).workbench
  }

  func dismissTableOperation(operationId: Data) throws -> Bool {
    try bridge.dismissTableOperation(operationId: operationId)
  }

  func revokeTableOperation(tokenId: Data) throws -> Bool {
    try bridge.revokeTableOperation(tokenId: tokenId)
  }

  func postgresActivity(sessionId: Data) throws -> [WorkbenchPostgresActivityRow] {
    try bridge.postgresActivity(sessionId: sessionId).map(\.workbench)
  }

  func postgresRelationships(sessionId: Data, catalogNodeId: Data) throws
    -> WorkbenchRelationshipSnapshot
  {
    try bridge.postgresRelationships(
      sessionId: sessionId, catalogNodeId: catalogNodeId
    ).workbench
  }

  func submitPostgresRelationBrowse(
    sessionId: Data, catalogNodeId: Data, selectedColumn: String, cell: WorkbenchCell
  ) throws -> WorkbenchRelationBrowseSubmission {
    try bridge.submitPostgresRelationBrowse(
      request: BridgeRelationBrowseRequest(
        sessionId: sessionId, catalogNodeId: catalogNodeId,
        selectedColumn: selectedColumn, cellKind: cell.kind,
        cellBytes: cell.bytes, cellTruncation: cell.truncation,
        rowCount: 500)
    ).workbench
  }

  func postgresRoles(sessionId: Data, catalogNodeId: Data?) throws -> WorkbenchRoleSnapshot {
    try bridge.postgresRoles(sessionId: sessionId, catalogNodeId: catalogNodeId).workbench
  }
  func stagePostgresRoleChange(
    sessionId: Data, catalogNodeId: Data?, kind: String, role: String,
    memberOrGrantee: String, privilege: String, nowMs: UInt64
  ) throws -> WorkbenchRoleChangeReview {
    try bridge.stagePostgresRoleChange(
      request: BridgeRoleChangeRequest(
        sessionId: sessionId, catalogNodeId: catalogNodeId, kind: kind, role: role,
        memberOrGrantee: memberOrGrantee, privilege: privilege, nowMs: nowMs)
    ).workbench
  }
  func applyPostgresRoleChange(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool
  ) throws -> String {
    try bridge.applyPostgresRoleChange(
      tokenId: tokenId, sessionId: sessionId, nowMs: nowMs, confirmed: confirmed)
  }
  func revokePostgresRoleChange(tokenId: Data) throws -> Bool {
    try bridge.revokePostgresRoleChange(tokenId: tokenId)
  }

  func signalPostgresBackend(sessionId: Data, kind: String, pid: Int32) throws
    -> WorkbenchBackendSignalOutcome
  {
    try bridge.signalPostgresBackend(sessionId: sessionId, kind: kind, pid: pid).workbench
  }

  func probePostgresTool(kind: String, explicitPath: String?) throws
    -> WorkbenchPostgresToolProbe
  { try bridge.probePostgresTool(kind: kind, explicitPath: explicitPath).workbench }

  func startPostgresTool(
    sessionId: Data, kind: String, toolPath: String, filePath: String, content: String,
    clean: Bool, noOwner: Bool
  ) throws -> Data {
    try bridge.startPostgresTool(
      request: BridgePostgresToolRequest(
        sessionId: sessionId, kind: kind, toolPath: toolPath, filePath: filePath,
        content: content, clean: clean, noOwner: noOwner))
  }

  func postgresToolStatus(operationId: Data) throws -> WorkbenchPostgresToolStatus {
    try bridge.postgresToolStatus(operationId: operationId).workbench
  }

  func cancelPostgresTool(operationId: Data) throws -> Bool {
    try bridge.cancelPostgresTool(operationId: operationId)
  }

  func applyReviewToken(tokenId: Data, nowMs: UInt64, sessionId: Data) throws
    -> WorkbenchApplyOutcome
  {
    try bridge.applyReviewToken(
      tokenId: tokenId, nowMs: nowMs, sessionId: sessionId, expectedRevision: 0
    ).workbench
  }

  func revokeReviewToken(tokenId: Data) throws -> Bool {
    try bridge.revokeReviewToken(tokenId: tokenId)
  }
}

public func makeLiveWorkbenchBackend(persistencePath: String) throws -> any WorkbenchBackend {
  try LiveWorkbenchBackend(persistencePath: persistencePath)
}
