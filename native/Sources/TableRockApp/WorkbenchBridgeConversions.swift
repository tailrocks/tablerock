import Foundation
import TableRockBridge
import TableRockFeature

// Generated UniFFI records stop here. Presentation consumes only immutable
// application facts from TableRockFeature.
extension BridgeProfileItem {
  var workbench: WorkbenchProfileItem {
    .init(
      idBytes: idBytes, revision: revision, name: name, engine: engine, group: group,
      favorite: favorite, savedOrder: savedOrder, host: host, port: port, context: context,
      safetyMode: safetyMode, environment: environment, productionWarning: productionWarning,
      dangerousPlaintext: dangerousPlaintext, connected: connected)
  }
}
extension BridgeProfileDraft {
  var workbench: WorkbenchProfileDraft {
    .init(
      idBytes: idBytes, revision: revision, engine: engine, name: name, group: group,
      environment: environment, host: host, port: port, database: database, username: username,
      passwordSource: passwordSource, passwordValue: passwordValue,
      passwordReference: passwordReference, hasStoredPassword: hasStoredPassword,
      plaintextAcknowledged: plaintextAcknowledged, tlsMode: tlsMode, safetyMode: safetyMode)
  }
}
extension WorkbenchProfileDraft {
  var bridgeRecord: BridgeProfileDraft {
    .init(
      idBytes: idBytes, revision: revision, engine: engine, name: name, group: group,
      environment: environment, host: host, port: port, database: database, username: username,
      passwordSource: passwordSource, passwordValue: passwordValue,
      passwordReference: passwordReference, hasStoredPassword: hasStoredPassword,
      plaintextAcknowledged: plaintextAcknowledged, tlsMode: tlsMode, safetyMode: safetyMode)
  }
}
extension BridgeProfileGroup {
  var workbench: WorkbenchProfileGroup { .init(name: name, alphabetical: alphabetical) }
}
extension BridgeConnectionTestReport {
  var workbench: WorkbenchConnectionTestReport {
    .init(identity: identity, tlsOutcome: tlsOutcome, elapsedMillis: elapsedMillis)
  }
}
extension BridgeCatalogNode {
  var workbench: WorkbenchCatalogNode {
    .init(
      idBytes: idBytes, parentIdBytes: parentIdBytes, depth: depth, name: name, kind: kind,
      childrenState: childrenState, expandable: expandable)
  }
}
extension BridgeHistoryItem {
  var workbench: WorkbenchHistoryItem {
    .init(
      historyId: historyId, engine: engine, databaseName: databaseName, schemaName: schemaName,
      statementText: statementText, outcome: outcome, createdAt: createdAt)
  }
}
extension BridgeSavedQueryItem {
  var workbench: WorkbenchSavedQueryItem {
    .init(
      queryId: queryId, name: name, engine: engine, statementText: statementText,
      updatedAt: updatedAt)
  }
}
extension BridgeSqlFile {
  var workbench: WorkbenchSQLFile {
    .init(path: path, statementText: statementText, modifiedNanos: modifiedNanos, len: len)
  }
}
extension BridgeWorkspaceTab {
  var workbench: WorkbenchWorkspaceTab { .init(title: title, statementText: statementText) }
}
extension WorkbenchWorkspaceTab {
  var bridgeRecord: BridgeWorkspaceTab { .init(title: title, statementText: statementText) }
}
extension BridgeSessionIntent {
  var workbench: WorkbenchSessionIntent {
    .init(database: database, schema: schema, selectedTab: selectedTab, tabs: tabs.map(\.workbench))
  }
}
extension WorkbenchSessionIntent {
  var bridgeRecord: BridgeSessionIntent {
    .init(
      database: database, schema: schema, selectedTab: selectedTab, tabs: tabs.map(\.bridgeRecord))
  }
}
extension BridgeNativeWindowIntent {
  var workbench: WorkbenchNativeWindowIntent {
    .init(profileId: profileId, intent: intent.workbench)
  }
}
extension BridgeSessionHealth {
  var workbench: WorkbenchSessionHealth {
    .init(
      state: state, serverReachable: serverReachable, elapsedMillis: elapsedMillis,
      authenticationStopped: authenticationStopped)
  }
}
extension BridgeReconnectPlan {
  var workbench: WorkbenchReconnectPlan {
    .init(action: action, delayMillis: delayMillis, restoreLastContext: restoreLastContext)
  }
}
extension BridgeReconnectAttempt {
  var workbench: WorkbenchReconnectAttempt { .init(state: state, sessionId: sessionId) }
}
extension BridgeCsvRow { var workbench: WorkbenchCSVRow { .init(cells: cells) } }
extension BridgeCsvImportPreview {
  var workbench: WorkbenchCSVImportPreview {
    .init(
      path: path, headers: headers, rows: rows.map(\.workbench), totalRows: totalRows,
      formulaLikeCells: formulaLikeCells)
  }
}
extension BridgeCsvImportReview {
  var workbench: WorkbenchCSVImportReview {
    .init(
      tokenId: tokenId, target: target, rowCount: rowCount, columnCount: columnCount,
      formulaLikeCells: formulaLikeCells, expiresAtMs: expiresAtMs)
  }
}
extension BridgeRedisKeyView {
  var workbench: WorkbenchRedisKeyView { .init(kind: kind, lines: lines, nextSkip: nextSkip) }
}
extension BridgeRedisOverview {
  var workbench: WorkbenchRedisOverview { .init(sampledAtMs: sampledAtMs, lines: lines) }
}
extension BridgePostgresActivityRow {
  var workbench: WorkbenchPostgresActivityRow {
    .init(
      pid: pid, user: user, application: application, state: state, queryPreview: queryPreview)
  }
}
extension BridgeBackendSignalOutcome {
  var workbench: WorkbenchBackendSignalOutcome {
    .init(kind: kind, pid: pid, acknowledged: acknowledged)
  }
}
extension BridgeRelationColumn {
  var workbench: WorkbenchRelationColumn {
    .init(
      name: name, dataType: dataType, nullable: nullable, defaultExpression: defaultExpression,
      comment: comment, primaryKey: primaryKey, sortingKey: sortingKey)
  }
}
extension BridgeRelationIndex {
  var workbench: WorkbenchRelationIndex { .init(kind: kind, name: name, definition: definition) }
}
extension BridgeRelationConstraint {
  var workbench: WorkbenchRelationConstraint {
    .init(kind: kind, name: name, definition: definition)
  }
}
extension BridgeRelationFact {
  var workbench: WorkbenchRelationFact { .init(name: name, value: value) }
}
extension BridgeRelationStructure {
  var workbench: WorkbenchRelationStructure {
    .init(
      engine: engine, namespace: namespace, relation: relation, columns: columns.map(\.workbench),
      indexes: indexes.map(\.workbench), constraints: constraints.map(\.workbench),
      facts: facts.map(\.workbench), ddl: ddl)
  }
}
extension ApplyOutcome {
  var workbench: WorkbenchApplyOutcome {
    .init(
      transaction: transaction, changeCount: changeCount, appliedCount: appliedCount,
      conflictCount: conflictCount, failedCount: failedCount)
  }
}
extension CancelOutcome {
  var workbench: WorkbenchCancelOutcome { .init(core: core, runtime: runtime) }
}
extension WorkbenchOpenParams {
  var bridgeRecord: OpenParams {
    .init(
      engine: engine, host: host, port: port, database: database, user: user, password: password,
      tlsMode: tlsMode)
  }
}
extension PageV1Envelope {
  var workbench: WorkbenchPageEnvelope {
    .init(
      encodingVersion: encodingVersion, resultId: resultId, revision: revision, engine: engine,
      startRow: startRow, rowCount: rowCount, columnCount: columnCount, arenaByteLen: arenaByteLen,
      columnTextByteLen: columnTextByteLen, delivery: delivery, warnings: warnings)
  }
}
extension PageV1Column {
  var workbench: WorkbenchColumn {
    .init(name: name, engine: engine, engineType: engineType, nullable: nullable)
  }
}
extension PageV1Cell {
  var workbench: WorkbenchCell {
    .init(
      display: display, kind: kind, truncation: truncation, originalByteCount: originalByteCount,
      bytes: bytes)
  }
}
extension PageV1Table {
  var workbench: WorkbenchTable {
    .init(
      columns: columns, rows: rows, columnMetadata: columnMetadata.map(\.workbench),
      cells: cells.map { $0.map(\.workbench) })
  }
}
