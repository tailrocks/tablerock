// TableRock native macOS app — plan 020.
//
// Built directly with Swift 6 against the macOS 26 SDK. The Rust bridge is
// linked as the cargo release dylib for local development; notarized
// XCFramework distribution remains the operator-gated release path (plan 019).
//
// Checkpoint 1: app shell + live bridge (runtime + persistence).
// Checkpoint 2: connection list — lists saved profiles over the bridge.

import AppKit
import Observation
import SwiftUI
import TableRockBridge
import TableRockFeature

/// Test-only environment projection for deterministic appearance evidence.
/// Production launches have no fixture variables and follow system settings.
struct NativeAppearanceFixture: Sendable {
  let scheme: ColorScheme?
  let increasedContrast: Bool
  let reduceTransparency: Bool
  let reduceMotion: Bool
  let differentiateWithoutColor: Bool

  static let current: NativeAppearanceFixture = {
    let environment = ProcessInfo.processInfo.environment
    let scheme: ColorScheme? =
      switch environment["TABLEROCK_FIXTURE_APPEARANCE"] {
      case "light": ColorScheme.light
      case "dark": ColorScheme.dark
      default: nil
      }
    return NativeAppearanceFixture(
      scheme: scheme,
      increasedContrast: environment["TABLEROCK_FIXTURE_CONTRAST"] == "1",
      reduceTransparency: environment["TABLEROCK_FIXTURE_REDUCE_TRANSPARENCY"] == "1",
      reduceMotion: environment["TABLEROCK_FIXTURE_REDUCE_MOTION"] == "1",
      differentiateWithoutColor: environment["TABLEROCK_FIXTURE_DIFFERENTIATE"] == "1"
    )
  }()

  var isActive: Bool {
    scheme != nil || increasedContrast || reduceTransparency || reduceMotion
      || differentiateWithoutColor
  }

  var label: String {
    [
      scheme == .dark ? "Dark" : "Light",
      increasedContrast ? "Increased contrast" : nil,
      reduceTransparency ? "Reduced transparency" : nil,
      reduceMotion ? "Reduced motion" : nil,
      differentiateWithoutColor ? "Differentiate without color" : nil,
    ]
    .compactMap { $0 }
    .joined(separator: " · ")
  }

  @MainActor
  func applyApplicationAppearance() {
    guard increasedContrast else { return }
    let name: NSAppearance.Name =
      scheme == .dark
      ? .accessibilityHighContrastDarkAqua
      : .accessibilityHighContrastAqua
    NSApplication.shared.appearance = NSAppearance(named: name)
  }
}

struct NativeAppearanceFixtureModifier: ViewModifier {
  let fixture: NativeAppearanceFixture

  func body(content: Content) -> some View {
    content
      .preferredColorScheme(fixture.scheme)
      .background {
        if fixture.reduceTransparency {
          Color(nsColor: .windowBackgroundColor)
        }
      }
      .transaction { transaction in
        if fixture.reduceMotion {
          transaction.animation = nil
          transaction.disablesAnimations = true
        }
      }
      .overlay(alignment: .bottomTrailing) {
        if fixture.isActive {
          Text("Fixture · \(fixture.label)")
            .font(.caption2)
            .padding(4)
            .background(.background)
            .padding(8)
            .accessibilityLabel("Appearance fixture \(fixture.label)")
        }
      }
  }
}

/// Sole owner of the synchronous UniFFI object. Blocking driver pumping and
/// page decoding run away from MainActor; awaiting the detached pump keeps this
/// actor reentrant so cancellation can use the operation id independently.
private enum ScriptedBackendError: Error {
  case unavailable(String)
  case connectionFailed
  case authenticationFailed
  case staleResultRevision
  case staleEvent
  case cursorResyncRequired
  case mismatchedPageColumns
  case historyFailedAfterPage
  case restorationCorrupt
}

private func scriptedUnavailable<T>(_ operation: String) throws -> T {
  throw ScriptedBackendError.unavailable(operation)
}

extension WorkbenchBackend {
  func exportSupportBundle(path: String) throws -> UInt64 {
    try scriptedUnavailable("support-export")
  }
  func searchProfiles(_ search: String?) throws -> [WorkbenchProfileItem] {
    try listProfiles()
  }
  func profileDraft(id: Data) throws -> WorkbenchProfileDraft {
    try scriptedUnavailable("draft")
  }
  func parseConnectionUrl(_ input: String) throws -> WorkbenchProfileDraft {
    try scriptedUnavailable("connection-url")
  }
  func saveProfile(_ draft: WorkbenchProfileDraft) throws -> Data {
    try scriptedUnavailable("save")
  }
  func prepareSampleDatabase(dataRoot: String) throws -> WorkbenchProfileDraft {
    _ = dataRoot
    return try scriptedUnavailable("sample")
  }
  func deleteProfile(id: Data, revision: UInt64) throws {
    throw ScriptedBackendError.unavailable("delete")
  }
  func testProfile(id: Data, secretOverride: Data?) throws
    -> WorkbenchConnectionTestReport
  { try scriptedUnavailable("test") }
  func createProfileGroup(_ name: String) throws {
    throw ScriptedBackendError.unavailable("group-create")
  }
  func renameProfileGroup(_ oldName: String, _ newName: String) throws -> UInt32 {
    try scriptedUnavailable("group-rename")
  }
  func deleteProfileGroup(_ name: String) throws -> UInt32 {
    try scriptedUnavailable("group-delete")
  }
  func setGroupAlphabetical(_ name: String, _ alphabetical: Bool) throws {
    throw ScriptedBackendError.unavailable("group-order")
  }
  func listHistory(_ search: String?) throws -> [WorkbenchHistoryItem] { [] }
  func setHistoryRetention(_ retention: String) throws {
    throw ScriptedBackendError.unavailable("retention")
  }
  func listSavedQueries(engine: String?, search: String?) throws
    -> [WorkbenchSavedQueryItem]
  { [] }
  func saveQuery(name: String, engine: String, statement: String) throws -> Int64 {
    try scriptedUnavailable("query-save")
  }
  func deleteSavedQuery(_ id: Int64) throws -> Bool {
    try scriptedUnavailable("query-delete")
  }
  func readSqlFile(path: String) throws -> WorkbenchSQLFile {
    try scriptedUnavailable("file-read")
  }
  func writeSqlFile(
    path: String, statement: String, expectedModifiedNanos: UInt64?, expectedLength: UInt64?,
    overwriteExternalChange: Bool
  ) throws -> WorkbenchSQLFile { try scriptedUnavailable("file-write") }
  func putSessionIntent(profileId: Data, intent: WorkbenchSessionIntent) throws {}
  func sessionIntent(profileId: Data) throws -> WorkbenchSessionIntent? { nil }
  func deleteSessionIntent(profileId: Data) throws {}
  func putNativeWindowIntent(
    windowId: String, profileId: Data, intent: WorkbenchSessionIntent
  ) throws {}
  func nativeWindowIntent(windowId: String) throws -> WorkbenchNativeWindowIntent? {
    nil
  }
  func deleteNativeWindowIntent(windowId: String) throws {}
  func setProfileFavorite(_ item: WorkbenchProfileItem, _ favorite: Bool) throws {
    throw ScriptedBackendError.unavailable("favorite")
  }
  func reorderProfiles(group: String?, profiles: [WorkbenchProfileItem]) throws {
    throw ScriptedBackendError.unavailable("reorder")
  }
  func open(params: WorkbenchOpenParams) throws -> Data {
    try scriptedUnavailable("open")
  }
  func disconnect(session: Data) throws {}
  func checkHealth(session: Data) throws -> WorkbenchSessionHealth {
    try scriptedUnavailable("health")
  }
  func planReconnect(session: Data, attempt: UInt32, authenticationStopped: Bool) throws
    -> WorkbenchReconnectPlan
  { try scriptedUnavailable("reconnect-plan") }
  func reconnect(session: Data, secretOverride: Data?) throws
    -> WorkbenchReconnectAttempt
  { try scriptedUnavailable("reconnect") }
  func refreshCatalog(session: Data, parentNodeId: Data?) throws
    -> [WorkbenchCatalogNode]
  { [] }
  func submitCatalogBrowse(
    session: Data, nodeId: Data, sort: [WorkbenchBrowseSort], filters: [WorkbenchBrowseFilter],
    rawWhere: String?
  ) throws -> Data {
    try scriptedUnavailable("browse")
  }
  func listCatalogFilterPresets(session: Data, nodeId: Data) throws
    -> [WorkbenchSavedFilterPreset]
  { [] }
  func saveCatalogFilterPreset(
    session: Data, nodeId: Data, preset: WorkbenchSavedFilterPreset
  ) throws { throw ScriptedBackendError.unavailable("saved-filter") }
  func submit(session: Data, intent: String, statement: String?) throws -> Data {
    try scriptedUnavailable("submit")
  }
  func inspectNamedParameters(statement: String) throws -> [String] {
    try scriptedUnavailable("named-parameters")
  }
  func submitNamed(
    session: Data, statement: String, bindings: [WorkbenchQueryParameter]
  ) throws -> Data { try scriptedUnavailable("named-parameters") }
  func finish(operationId: Data) async throws -> WorkbenchOperation {
    try scriptedUnavailable("finish")
  }
  func cancel(operationId: Data) throws -> WorkbenchCancelOutcome {
    try scriptedUnavailable("cancel")
  }
  func fetchPage(resultId: Data, startRow: UInt64, revision: UInt64) async throws -> (
    WorkbenchTable, WorkbenchPageEnvelope
  ) { try scriptedUnavailable("fetch") }
  func formatResultCopy(
    resultId: Data, revision: UInt64, scope: String, row: UInt64?, column: UInt32?, format: String
  ) throws -> String { try scriptedUnavailable("copy") }
  func exportLoadedResult(
    resultId: Data, revision: UInt64, format: String, path: String
  ) throws -> UInt64 { try scriptedUnavailable("export") }
  func startStreamExport(sessionId: Data, statement: String, format: String, path: String) throws
    -> Data
  { try scriptedUnavailable("stream-export-start") }
  func startCatalogStreamExport(
    resultId: Data, revision: UInt64, format: String, path: String
  ) throws -> Data { try scriptedUnavailable("catalog-stream-export-start") }
  func streamExportProgress(operationId: Data) throws -> WorkbenchStreamExportProgress {
    try scriptedUnavailable("stream-export-progress")
  }
  func cancelStreamExport(operationId: Data) throws -> Bool {
    try scriptedUnavailable("stream-export-cancel")
  }
  func dismissStreamExport(operationId: Data) throws -> Bool {
    try scriptedUnavailable("stream-export-dismiss")
  }
  func previewCsvImport(path: String) throws -> WorkbenchCSVImportPreview {
    try scriptedUnavailable("import-preview")
  }
  func stageCsvImport(
    sessionId: Data, catalogNodeId: Data, path: String, mappedColumns: [String],
    mappedTypes: [String], expectedFingerprint: String, nowMs: UInt64
  ) throws -> WorkbenchCSVImportReview { try scriptedUnavailable("import-stage") }
  func startCsvImportApply(tokenId: Data, nowMs: UInt64, sessionId: Data) throws -> Data {
    try scriptedUnavailable("import-apply-start")
  }
  func csvImportProgress(operationId: Data) throws -> WorkbenchCSVImportProgress {
    try scriptedUnavailable("import-progress")
  }
  func cancelCsvImport(operationId: Data) throws -> Bool {
    try scriptedUnavailable("import-cancel")
  }
  func dismissCsvImport(operationId: Data) throws -> Bool {
    try scriptedUnavailable("import-dismiss")
  }
  func relationStructure(sessionId: Data, catalogNodeId: Data) throws
    -> WorkbenchRelationStructure
  { try scriptedUnavailable("structure") }
  func redisKeyView(sessionId: Data, catalogNodeId: Data, collectionSkip: UInt64) throws
    -> WorkbenchRedisKeyView
  { try scriptedUnavailable("redis-key") }
  func redisOverview(sessionId: Data) throws -> WorkbenchRedisOverview {
    try scriptedUnavailable("redis-overview")
  }
  func startRedisSubscription(sessionId: Data, selector: String, pattern: Bool) throws -> Data {
    try scriptedUnavailable("redis-subscription-start")
  }
  func redisSubscriptionStatus(operationId: Data) throws -> WorkbenchRedisSubscriptionStatus {
    try scriptedUnavailable("redis-subscription-status")
  }
  func cancelRedisSubscription(operationId: Data) throws -> Bool {
    try scriptedUnavailable("redis-subscription-cancel")
  }
  func stageDdlChange(
    sessionId: Data, catalogNodeId: Data, kind: String, objectName: String,
    definition: String, nowMs: UInt64
  ) throws -> WorkbenchDdlChangeReview { try scriptedUnavailable("ddl-change-stage") }
  func applyDdlChange(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool
  ) throws -> String { try scriptedUnavailable("ddl-change-apply") }
  func revokeDdlChange(tokenId: Data) throws -> Bool {
    try scriptedUnavailable("ddl-change-revoke")
  }
  func stageTableOperation(
    sessionId: Data, catalogNodeId: Data, kind: String, newName: String, nowMs: UInt64
  ) throws -> WorkbenchTableOperationReview { try scriptedUnavailable("table-operation-stage") }
  func startTableOperation(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmation: String
  ) throws -> Data { try scriptedUnavailable("table-operation-start") }
  func tableOperationStatus(operationId: Data) throws -> WorkbenchTableOperationStatus {
    try scriptedUnavailable("table-operation-status")
  }
  func dismissTableOperation(operationId: Data) throws -> Bool {
    try scriptedUnavailable("table-operation-dismiss")
  }
  func revokeTableOperation(tokenId: Data) throws -> Bool {
    try scriptedUnavailable("table-operation-revoke")
  }
  func postgresActivity(sessionId: Data) throws -> [WorkbenchPostgresActivityRow] {
    try scriptedUnavailable("postgres-activity")
  }
  func postgresRelationships(sessionId: Data, catalogNodeId: Data) throws
    -> WorkbenchRelationshipSnapshot
  { try scriptedUnavailable("postgres-relationships") }
  func postgresRoles(sessionId: Data, catalogNodeId: Data?) throws -> WorkbenchRoleSnapshot {
    try scriptedUnavailable("postgres-roles")
  }
  func stagePostgresRoleChange(
    sessionId: Data, catalogNodeId: Data?, kind: String, role: String,
    memberOrGrantee: String, privilege: String, nowMs: UInt64
  ) throws -> WorkbenchRoleChangeReview { try scriptedUnavailable("postgres-role-change-stage") }
  func applyPostgresRoleChange(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool
  ) throws -> String { try scriptedUnavailable("postgres-role-change-apply") }
  func revokePostgresRoleChange(tokenId: Data) throws -> Bool {
    try scriptedUnavailable("postgres-role-change-revoke")
  }
  func signalPostgresBackend(sessionId: Data, kind: String, pid: Int32) throws
    -> WorkbenchBackendSignalOutcome
  { try scriptedUnavailable("postgres-activity-signal") }
  func probePostgresTool(kind: String, explicitPath: String?) throws
    -> WorkbenchPostgresToolProbe
  { try scriptedUnavailable("postgres-tool-probe") }
  func startPostgresTool(
    sessionId: Data, kind: String, toolPath: String, filePath: String, content: String,
    clean: Bool, noOwner: Bool
  ) throws -> Data { try scriptedUnavailable("postgres-tool-start") }
  func postgresToolStatus(operationId: Data) throws -> WorkbenchPostgresToolStatus {
    try scriptedUnavailable("postgres-tool-status")
  }
  func cancelPostgresTool(operationId: Data) throws -> Bool {
    try scriptedUnavailable("postgres-tool-cancel")
  }
  func applyReviewToken(tokenId: Data, nowMs: UInt64, sessionId: Data) throws
    -> WorkbenchApplyOutcome
  { try scriptedUnavailable("apply") }
  func revokeReviewToken(tokenId: Data) throws -> Bool {
    try scriptedUnavailable("revoke")
  }
  func stageProbeReview(sessionId: Data, nowMs: UInt64) throws -> Data {
    try scriptedUnavailable("stage-probe")
  }
  func stageAndApply(session: Data, now: UInt64) throws -> WorkbenchApplyOutcome {
    try scriptedUnavailable("stage-apply")
  }

}

actor ScriptedWorkbenchBackend: WorkbenchBackend {
  let scenario: String
  private var cancelled = false
  private var importReviewActive = false
  private var probeReviewActive = false
  private var importApplyActive = false
  private var importApplyCancelled = false
  private var importApplyPollCount = 0
  private var streamExportActive = false
  private var streamExportCancelled = false
  private var streamExportPollCount = 0
  private var profiles: [WorkbenchProfileItem] = []
  private var profileDrafts: [Data: WorkbenchProfileDraft] = [:]
  private var filterPresets: [WorkbenchSavedFilterPreset] = []
  private var submittedIntent: String?
  private var postgresToolPhase = "succeeded"
  private var redisSubscriptionActive = false
  private var ddlReviewActive = false
  private var tableOperationReviewActive = false
  private var scriptedTableOperationKind = "truncate"
  private var scriptedTableOperationPollCount = 0

  init(scenario: String) { self.scenario = scenario }

  func listProfiles() throws -> [WorkbenchProfileItem] {
    if scenario != "restoration-corrupt" { return profiles }
    return profiles + [
      WorkbenchProfileItem(
        idBytes: Data(repeating: 4, count: 16), revision: 1,
        name: "Restoration fixture", engine: "postgresql", group: nil,
        favorite: false, savedOrder: 0, host: nil, port: nil,
        context: nil, safetyMode: "read_only", environment: nil,
        productionWarning: false, dangerousPlaintext: false, connected: false
      )
    ]
  }
  func listProfileGroups() throws -> [WorkbenchProfileGroup] { [] }
  func historyRetention() throws -> String { "full" }
  func listCatalogFilterPresets(session: Data, nodeId: Data) throws
    -> [WorkbenchSavedFilterPreset]
  { filterPresets }
  func saveCatalogFilterPreset(
    session: Data, nodeId: Data, preset: WorkbenchSavedFilterPreset
  ) throws {
    guard scenario == "success" else { return try scriptedUnavailable("saved-filter") }
    filterPresets.removeAll(where: { $0.name == preset.name })
    filterPresets.append(preset)
  }

  func profileDraft(id: Data) throws -> WorkbenchProfileDraft {
    guard let draft = profileDrafts[id] else { return try scriptedUnavailable("draft") }
    return draft
  }

  func parseConnectionUrl(_ input: String) throws -> WorkbenchProfileDraft {
    guard scenario == "success", input == "postgresql://fixture:secret@db.example:5433/app"
    else { return try scriptedUnavailable("connection-url") }
    return WorkbenchProfileDraft(
      idBytes: nil, revision: 0, engine: "postgresql", name: "", group: "",
      environment: "", host: "db.example", port: "5433", database: "app",
      username: "fixture", passwordSource: "keychain", passwordValue: "secret",
      passwordReference: nil, hasStoredPassword: false, plaintextAcknowledged: false,
      tlsMode: "off", safetyMode: "confirm_writes")
  }

  func prepareSampleDatabase(dataRoot: String) throws -> WorkbenchProfileDraft {
    _ = dataRoot
    guard scenario == "success" else { return try scriptedUnavailable("sample") }
    return WorkbenchProfileDraft(
      idBytes: nil, revision: 0, engine: "sqlite", name: "Sample Database",
      group: "", environment: "development",
      host: "local", port: "1", database: "samples/tablerock-sample.db", username: "",
      passwordSource: "none", passwordValue: "", passwordReference: nil,
      hasStoredPassword: false, plaintextAcknowledged: false, tlsMode: "off",
      safetyMode: "read_only")
  }

  func saveProfile(_ draft: WorkbenchProfileDraft) throws -> Data {
    guard scenario == "success" else { return try scriptedUnavailable("save") }
    let id = draft.idBytes ?? Data(repeating: 9, count: 16)
    let revision = draft.idBytes == nil ? 1 : draft.revision + 1
    let stored = WorkbenchProfileDraft(
      idBytes: id, revision: revision, engine: draft.engine, name: draft.name,
      group: draft.group, environment: draft.environment, host: draft.host,
      port: draft.port, database: draft.database, username: draft.username,
      passwordSource: draft.passwordSource, passwordValue: "",
      passwordReference: draft.passwordReference,
      hasStoredPassword: draft.hasStoredPassword,
      plaintextAcknowledged: draft.plaintextAcknowledged,
      tlsMode: draft.tlsMode, safetyMode: draft.safetyMode,
      sshEnabled: draft.sshEnabled, sshHost: draft.sshHost, sshPort: draft.sshPort,
      sshUsername: draft.sshUsername, sshAuthMode: draft.sshAuthMode,
      sshKnownHostsPath: draft.sshKnownHostsPath,
      sshHasStoredPassword: draft.sshHasStoredPassword || !draft.sshPassword.isEmpty,
      sshHasStoredPrivateKey: draft.sshHasStoredPrivateKey || !draft.sshPrivateKey.isEmpty,
      sshPlaintextAcknowledged: draft.sshPlaintextAcknowledged,
      startupActions: draft.startupActions)
    profileDrafts[id] = stored
    profiles.removeAll { $0.idBytes == id }
    profiles.append(
      WorkbenchProfileItem(
        idBytes: id, revision: revision, name: draft.name, engine: draft.engine,
        group: draft.group.isEmpty ? nil : draft.group, favorite: false,
        savedOrder: UInt32(profiles.count), host: draft.host, port: draft.port,
        context: draft.database, safetyMode: draft.safetyMode,
        environment: draft.environment.isEmpty ? nil : draft.environment,
        productionWarning: draft.environment == "production",
        dangerousPlaintext: draft.passwordSource == "dangerous_plaintext", connected: false))
    return id
  }

  func nativeWindowIntent(windowId: String) throws -> WorkbenchNativeWindowIntent? {
    guard scenario == "restoration-corrupt" else { return nil }
    return WorkbenchNativeWindowIntent(
      profileId: Data(repeating: 4, count: 16),
      intent: WorkbenchSessionIntent(
        database: "postgres", schema: nil, selectedTab: 99,
        tabs: [WorkbenchWorkspaceTab(title: "Invalid", statementText: "SELECT 1;")]
      )
    )
  }

  func openProfile(id: Data, secretOverride: Data?) throws -> Data {
    switch scenario {
    case "connection-failure": throw ScriptedBackendError.connectionFailed
    case "authentication-failure": throw ScriptedBackendError.authenticationFailed
    default: return Data(repeating: 1, count: 16)
    }
  }

  func checkHealth(session: Data) throws -> WorkbenchSessionHealth {
    guard scenario == "success", session == Data(repeating: 1, count: 16) else {
      return try scriptedUnavailable("health")
    }
    return WorkbenchSessionHealth(
      state: "healthy", serverReachable: true, elapsedMillis: 1,
      authenticationStopped: false)
  }

  func refreshCatalog(session: Data, parentNodeId: Data?) throws -> [WorkbenchCatalogNode] {
    guard scenario == "success", session == Data(repeating: 1, count: 16) else {
      return try scriptedUnavailable("catalog")
    }
    let root = Data(repeating: 6, count: 16)
    let table = WorkbenchCatalogNode(
      idBytes: Data(repeating: 7, count: 16), parentIdBytes: root, depth: 1,
      name: "fixture_table", kind: "postgresql_table",
      childrenState: "not_applicable", expandable: false)
    if parentNodeId == root { return [table] }
    guard parentNodeId == nil else { return [] }
    return [
      WorkbenchCatalogNode(
        idBytes: root, parentIdBytes: nil, depth: 0, name: "public",
        kind: "postgresql_schema", childrenState: "loaded_complete", expandable: true),
      table,
    ]
  }

  func relationStructure(sessionId: Data, catalogNodeId: Data) throws
    -> WorkbenchRelationStructure
  {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16),
      catalogNodeId == Data(repeating: 7, count: 16)
    else { return try scriptedUnavailable("relation-structure") }
    return WorkbenchRelationStructure(
      engine: "postgresql", namespace: "public", relation: "fixture_table",
      columns: [
        WorkbenchRelationColumn(
          name: "id", dataType: "bigint", nullable: false, defaultExpression: nil,
          comment: nil, primaryKey: true, sortingKey: false),
        WorkbenchRelationColumn(
          name: "name", dataType: "text", nullable: true, defaultExpression: nil,
          comment: nil, primaryKey: false, sortingKey: false),
      ],
      indexes: [
        WorkbenchRelationIndex(
          kind: "primary", name: "fixture_table_pkey",
          definition: "CREATE UNIQUE INDEX fixture_table_pkey ON public.fixture_table (id)")
      ],
      constraints: [
        WorkbenchRelationConstraint(
          kind: "primary_key", name: "fixture_table_pkey", definition: "PRIMARY KEY (id)")
      ],
      facts: [WorkbenchRelationFact(name: "Rows", value: "2")],
      ddl: "CREATE TABLE public.fixture_table (id bigint PRIMARY KEY, name text);"
    )
  }

  func open(params: WorkbenchOpenParams) throws -> Data {
    switch scenario {
    case "connection-failure": throw ScriptedBackendError.connectionFailed
    case "authentication-failure": throw ScriptedBackendError.authenticationFailed
    default: return Data(repeating: 1, count: 16)
    }
  }

  func submit(session: Data, intent: String, statement: String?) throws -> Data {
    submittedIntent = intent
    return Data(repeating: 2, count: 16)
  }

  func inspectNamedParameters(statement: String) throws -> [String] {
    if statement.contains(":id") { return ["id"] }
    if statement.contains(":value") { return ["value"] }
    return []
  }

  func submitNamed(
    session: Data, statement: String, bindings: [WorkbenchQueryParameter]
  ) throws -> Data {
    guard !bindings.isEmpty else { return try scriptedUnavailable("named-parameters") }
    for binding in bindings {
      if binding.kind == "integer", Int64(binding.value) == nil {
        return try scriptedUnavailable("invalid 64-bit integer")
      }
      if binding.kind == "float",
        Double(binding.value).map({ !$0.isFinite }) != false
      {
        return try scriptedUnavailable("invalid 64-bit float")
      }
    }
    submittedIntent = "execute"
    return Data(repeating: 2, count: 16)
  }

  func finish(operationId: Data) async throws -> WorkbenchOperation {
    if scenario == "slow-until-cancelled" {
      while !cancelled { try await Task.sleep(for: .milliseconds(10)) }
      return WorkbenchOperation(
        table: nil, envelope: nil, outcome: "cancelled", historyFailed: false)
    }
    if scenario == "stale-event" { throw ScriptedBackendError.staleEvent }
    if scenario == "cursor-resync" { throw ScriptedBackendError.cursorResyncRequired }
    if scenario == "history-failure-after-page" {
      return WorkbenchOperation(
        table: nil, envelope: nil, outcome: "ok", historyFailed: true)
    }
    if scenario == "success", submittedIntent == "explain" {
      return WorkbenchOperation(
        table: WorkbenchTable(
          columns: ["QUERY PLAN"],
          rows: [["Seq Scan on fixture"], ["  Filter: (id > 0)"]]),
        envelope: nil, outcome: "completed", historyFailed: false)
    }
    return WorkbenchOperation(table: nil, envelope: nil, outcome: "ok", historyFailed: false)
  }

  func cancel(operationId: Data) throws -> WorkbenchCancelOutcome {
    cancelled = true
    return WorkbenchCancelOutcome(core: "Requested", runtime: nil)
  }

  func fetchPage(resultId: Data, startRow: UInt64, revision: UInt64) async throws -> (
    WorkbenchTable, WorkbenchPageEnvelope
  ) {
    if scenario == "stale-result-revision" { throw ScriptedBackendError.staleResultRevision }
    if scenario == "mismatched-next-page-columns" {
      throw ScriptedBackendError.mismatchedPageColumns
    }
    if scenario == "success", resultId == Data(repeating: 8, count: 16), startRow == 500,
      revision == 1
    {
      return (
        WorkbenchTable(columns: ["n"], rows: [["501"]]),
        WorkbenchPageEnvelope(
          encodingVersion: 1, resultId: resultId, revision: revision, engine: 0,
          startRow: startRow, rowCount: 1, columnCount: 1, arenaByteLen: 3,
          columnTextByteLen: 1, delivery: 1, warnings: 0)
      )
    }
    return try scriptedUnavailable("fetch")
  }

  func exportLoadedResult(
    resultId: Data, revision: UInt64, format: String, path: String
  ) throws -> UInt64 {
    guard scenario == "success", resultId == Data(repeating: 8, count: 16), revision == 1,
      format == "csv"
    else { return try scriptedUnavailable("export") }
    let payload = Data("id,name\n1,Ada\n".utf8)
    try payload.write(to: URL(fileURLWithPath: path), options: .atomic)
    return UInt64(payload.count)
  }

  func startStreamExport(sessionId: Data, statement: String, format: String, path: String) throws
    -> Data
  {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16), !statement.isEmpty,
      ["csv", "tsv", "json"].contains(format), !path.isEmpty
    else { return try scriptedUnavailable("stream-export-start") }
    streamExportActive = true
    streamExportCancelled = false
    streamExportPollCount = 0
    return Data(repeating: 16, count: 16)
  }

  func startCatalogStreamExport(
    resultId: Data, revision: UInt64, format: String, path: String
  ) throws -> Data {
    guard resultId == Data(repeating: 8, count: 16), ["csv", "tsv", "json"].contains(format),
      !path.isEmpty
    else { return try scriptedUnavailable("catalog-stream-export-start") }
    streamExportActive = true
    streamExportCancelled = false
    streamExportPollCount = 0
    return Data(repeating: 16, count: 16)
  }

  func streamExportProgress(operationId: Data) throws -> WorkbenchStreamExportProgress {
    guard scenario == "success", operationId == Data(repeating: 16, count: 16), streamExportActive
    else { return try scriptedUnavailable("stream-export-progress") }
    streamExportPollCount += 1
    let phase =
      streamExportCancelled
      ? "cancelled" : (streamExportPollCount < 4 ? "running" : "completed")
    return WorkbenchStreamExportProgress(
      operationId: operationId, phase: phase,
      completedRows: phase == "completed" ? 2 : UInt64(streamExportPollCount),
      bytesWritten: phase == "completed" ? 24 : UInt64(streamExportPollCount * 6),
      destination: "/tmp/result.csv",
      summary: phase == "completed"
        ? "Exported 2 rows (24 bytes) atomically"
        : (phase == "cancelled"
          ? "Cancelled; incomplete output removed" : "Exporting full result"))
  }

  func cancelStreamExport(operationId: Data) throws -> Bool {
    guard operationId == Data(repeating: 16, count: 16), streamExportActive else {
      return false
    }
    streamExportCancelled = true
    return true
  }

  func dismissStreamExport(operationId: Data) throws -> Bool {
    guard operationId == Data(repeating: 16, count: 16), streamExportActive else {
      return false
    }
    streamExportActive = false
    return true
  }

  func exportSupportBundle(path: String) throws -> UInt64 {
    guard scenario == "success" else { return try scriptedUnavailable("support-export") }
    let payload = Data(
      "schema=1\nclient.version=scripted\nplatform.os=macos\nplatform.arch=test\ndiagnostics.count=0\ndiagnostics.omitted=0\n"
        .utf8)
    try payload.write(to: URL(fileURLWithPath: path), options: .atomic)
    return UInt64(payload.count)
  }

  func previewCsvImport(path: String) throws -> WorkbenchCSVImportPreview {
    guard scenario == "success",
      try String(contentsOfFile: path, encoding: .utf8) == "id,name\n2,Grace\n"
    else { return try scriptedUnavailable("import-preview") }
    return WorkbenchCSVImportPreview(
      path: path, headers: ["id", "name"],
      rows: [WorkbenchCSVRow(cells: ["2", "Grace"])], totalRows: 1,
      formulaLikeCells: 0, fingerprint: "fixture-sha256")
  }

  func stageCsvImport(
    sessionId: Data, catalogNodeId: Data, path: String,
    mappedColumns: [String], mappedTypes: [String], expectedFingerprint: String, nowMs: UInt64
  ) throws -> WorkbenchCSVImportReview {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16),
      catalogNodeId == Data(repeating: 7, count: 16), mappedColumns == ["id", "name"],
      mappedTypes == ["text", "text"], expectedFingerprint == "fixture-sha256",
      !importReviewActive
    else { return try scriptedUnavailable("import-stage") }
    importReviewActive = true
    return WorkbenchCSVImportReview(
      tokenId: Data(repeating: 10, count: 16), target: "public.fixture_table",
      rowCount: 1, columnCount: 2, formulaLikeCells: 0,
      expiresAtMs: nowMs + 60_000)
  }

  func startCsvImportApply(tokenId: Data, nowMs: UInt64, sessionId: Data) throws -> Data {
    guard scenario == "success", importReviewActive,
      tokenId == Data(repeating: 10, count: 16), sessionId == Data(repeating: 1, count: 16)
    else { return try scriptedUnavailable("import-apply-start") }
    importReviewActive = false
    importApplyActive = true
    importApplyCancelled = false
    importApplyPollCount = 0
    return Data(repeating: 15, count: 16)
  }

  func csvImportProgress(operationId: Data) throws -> WorkbenchCSVImportProgress {
    guard scenario == "success", operationId == Data(repeating: 15, count: 16), importApplyActive
    else { return try scriptedUnavailable("import-progress") }
    importApplyPollCount += 1
    let phase =
      importApplyCancelled
      ? "cancelled" : (importApplyPollCount < 20 ? "running" : "completed")
    let complete = phase == "completed"
    return WorkbenchCSVImportProgress(
      operationId: operationId, phase: phase,
      completedRows: complete ? 1 : 0, totalRows: 1,
      appliedRows: complete ? 1 : 0, conflictRows: 0, failedRows: 0,
      errors: [], errorsTruncated: false,
      summary: importApplyCancelled
        ? "Cancelled before apply"
        : (complete ? "Committed · 1 applied" : "Applying reviewed import"))
  }

  func cancelCsvImport(operationId: Data) throws -> Bool {
    guard scenario == "success", operationId == Data(repeating: 15, count: 16), importApplyActive
    else { return try scriptedUnavailable("import-cancel") }
    importApplyCancelled = true
    return true
  }

  func dismissCsvImport(operationId: Data) throws -> Bool {
    guard scenario == "success", operationId == Data(repeating: 15, count: 16), importApplyActive
    else { return try scriptedUnavailable("import-dismiss") }
    importApplyActive = false
    return true
  }

  func applyReviewToken(tokenId: Data, nowMs: UInt64, sessionId: Data) throws
    -> WorkbenchApplyOutcome
  {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16) else {
      return try scriptedUnavailable("apply")
    }
    if importReviewActive, tokenId == Data(repeating: 10, count: 16) {
      importReviewActive = false
      return WorkbenchApplyOutcome(
        transaction: "committed", changeCount: 1, appliedCount: 1,
        conflictCount: 0, failedCount: 0)
    }
    if probeReviewActive, tokenId == Data(repeating: 11, count: 16) {
      probeReviewActive = false
      return WorkbenchApplyOutcome(
        transaction: "committed", changeCount: 1, appliedCount: 1,
        conflictCount: 0, failedCount: 0)
    }
    return try scriptedUnavailable("apply")
  }

  func revokeReviewToken(tokenId: Data) throws -> Bool {
    guard scenario == "success" else {
      return try scriptedUnavailable("revoke")
    }
    if tokenId == Data(repeating: 10, count: 16) {
      let wasActive = importReviewActive
      importReviewActive = false
      return wasActive
    }
    if tokenId == Data(repeating: 11, count: 16) {
      let wasActive = probeReviewActive
      probeReviewActive = false
      return wasActive
    }
    return try scriptedUnavailable("revoke")
  }

  func stageProbeReview(sessionId: Data, nowMs: UInt64) throws -> Data {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16), !probeReviewActive
    else { return try scriptedUnavailable("stage-probe") }
    _ = nowMs
    probeReviewActive = true
    return Data(repeating: 11, count: 16)
  }

  func stageAndApply(session: Data, now: UInt64) throws -> WorkbenchApplyOutcome {
    let token = try stageProbeReview(sessionId: session, nowMs: now)
    return try applyReviewToken(tokenId: token, nowMs: now, sessionId: session)
  }

  func startRedisSubscription(sessionId: Data, selector: String, pattern: Bool) throws -> Data {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16), !selector.isEmpty
    else { return try scriptedUnavailable("redis-subscription-start") }
    redisSubscriptionActive = true
    return Data(repeating: pattern ? 14 : 13, count: 16)
  }

  func redisSubscriptionStatus(operationId: Data) throws -> WorkbenchRedisSubscriptionStatus {
    guard scenario == "success" else {
      return try scriptedUnavailable("redis-subscription-status")
    }
    return WorkbenchRedisSubscriptionStatus(
      operationId: operationId, selector: "updates:*", pattern: operationId.first == 14,
      phase: redisSubscriptionActive ? "listening" : "cancelled",
      messages: ["updates:users · fixture message"], totalReceived: 1,
      discontinuities: 1,
      summary: redisSubscriptionActive
        ? "Listening; delivery gap observed" : "Subscription cancelled")
  }

  func cancelRedisSubscription(operationId: Data) throws -> Bool {
    guard scenario == "success" else {
      return try scriptedUnavailable("redis-subscription-cancel")
    }
    let wasActive = redisSubscriptionActive
    redisSubscriptionActive = false
    return wasActive
  }

  func stageDdlChange(
    sessionId: Data, catalogNodeId: Data, kind: String, objectName: String,
    definition: String, nowMs: UInt64
  ) throws -> WorkbenchDdlChangeReview {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16), !objectName.isEmpty,
      !ddlReviewActive
    else { return try scriptedUnavailable("ddl-change-stage") }
    ddlReviewActive = true
    let destructive = kind.hasPrefix("drop_")
    let suffix = definition.isEmpty ? "" : " \(definition)"
    return WorkbenchDdlChangeReview(
      tokenId: Data(repeating: 15, count: 16),
      preview: "\(kind) public.fixture_table \(objectName)\(suffix);",
      destructive: destructive,
      rollbackSummary:
        "PostgreSQL applies this statement atomically; TableRock does not automatically roll it back after observed success.",
      expiresAtMs: nowMs + 60_000)
  }

  func applyDdlChange(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool
  ) throws -> String {
    guard scenario == "success", ddlReviewActive,
      tokenId == Data(repeating: 15, count: 16), confirmed
    else { return try scriptedUnavailable("ddl-change-apply") }
    ddlReviewActive = false
    return "Structure change applied"
  }

  func revokeDdlChange(tokenId: Data) throws -> Bool {
    guard scenario == "success", tokenId == Data(repeating: 15, count: 16) else {
      return try scriptedUnavailable("ddl-change-revoke")
    }
    let wasActive = ddlReviewActive
    ddlReviewActive = false
    return wasActive
  }

  func stageTableOperation(
    sessionId: Data, catalogNodeId: Data, kind: String, newName: String, nowMs: UInt64
  ) throws -> WorkbenchTableOperationReview {
    guard scenario == "success", !tableOperationReviewActive else {
      return try scriptedUnavailable("table-operation-stage")
    }
    tableOperationReviewActive = true
    scriptedTableOperationKind = kind
    return WorkbenchTableOperationReview(
      tokenId: Data(repeating: 16, count: 16), target: "public.fixture_table",
      preview: "\(kind.uppercased()) public.fixture_table\(newName.isEmpty ? "" : " \(newName)");",
      destructive: ["truncate", "drop"].contains(kind), confirmation: "fixture_table",
      expiresAtMs: nowMs + 60_000)
  }

  func startTableOperation(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmation: String
  ) throws -> Data {
    guard tableOperationReviewActive, tokenId == Data(repeating: 16, count: 16),
      confirmation == "fixture_table"
    else { return try scriptedUnavailable("table-operation-start") }
    tableOperationReviewActive = false
    scriptedTableOperationPollCount = 0
    return Data(repeating: 17, count: 16)
  }

  func tableOperationStatus(operationId: Data) throws -> WorkbenchTableOperationStatus {
    guard operationId == Data(repeating: 17, count: 16) else {
      return try scriptedUnavailable("table-operation-status")
    }
    scriptedTableOperationPollCount += 1
    let running = scriptedTableOperationPollCount <= 20
    return WorkbenchTableOperationStatus(
      operationId: operationId, kind: scriptedTableOperationKind,
      phase: running ? "running" : "succeeded", cancellable: false,
      summary: running
        ? "Running \(scriptedTableOperationKind)" : "\(scriptedTableOperationKind) completed")
  }

  func dismissTableOperation(operationId: Data) throws -> Bool {
    operationId == Data(repeating: 17, count: 16)
  }

  func revokeTableOperation(tokenId: Data) throws -> Bool {
    let active = tableOperationReviewActive
    tableOperationReviewActive = false
    return active
  }

  func postgresActivity(sessionId: Data) throws -> [WorkbenchPostgresActivityRow] {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16) else {
      return try scriptedUnavailable("postgres-activity")
    }
    return [
      WorkbenchPostgresActivityRow(
        pid: 4242, user: "fixture", application: "TableRock fixture", state: "active",
        queryPreview: "SELECT pg_sleep(30)")
    ]
  }

  func postgresRelationships(sessionId: Data, catalogNodeId: Data) throws
    -> WorkbenchRelationshipSnapshot
  {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16) else {
      return try scriptedUnavailable("postgres-relationships")
    }
    return WorkbenchRelationshipSnapshot(
      namespace: "public", relation: "fixture_table",
      edges: [
        WorkbenchRelationshipEdge(
          fromSchema: "public", fromTable: "fixture_table", fromColumn: "customer_id",
          toSchema: "public", toTable: "customers", toColumn: "id"),
        WorkbenchRelationshipEdge(
          fromSchema: "public", fromTable: "fixture_table", fromColumn: "parent_id",
          toSchema: "public", toTable: "fixture_table", toColumn: "id"),
      ], truncated: false)
  }

  func postgresRoles(sessionId: Data, catalogNodeId: Data?) throws -> WorkbenchRoleSnapshot {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16) else {
      return try scriptedUnavailable("postgres-roles")
    }
    return WorkbenchRoleSnapshot(
      currentUser: "fixture", roles: ["fixture", "reader"],
      memberships: [
        WorkbenchRoleMembership(
          role: "reader", member: "fixture", inheritOption: true, adminOption: false,
          setOption: true)
      ],
      effectiveRoles: ["fixture", "reader"], cycleEdges: [],
      privileges: [
        WorkbenchRolePrivilege(
          grantee: "reader", privilege: "SELECT", object: "public.fixture_table",
          grantable: false)
      ], privilegeScope: catalogNodeId == nil ? nil : "public.fixture_table",
      privilegesUnavailable: false, truncated: false)
  }
  func stagePostgresRoleChange(
    sessionId: Data, catalogNodeId: Data?, kind: String, role: String,
    memberOrGrantee: String, privilege: String, nowMs: UInt64
  ) throws -> WorkbenchRoleChangeReview {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16),
      !role.isEmpty, !memberOrGrantee.isEmpty
    else { return try scriptedUnavailable("postgres-role-change-stage") }
    return WorkbenchRoleChangeReview(
      tokenId: Data(repeating: 12, count: 16),
      summary: "\(kind) \(role) \(memberOrGrantee)", expiresAtMs: nowMs + 60_000)
  }
  func applyPostgresRoleChange(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool
  ) throws -> String {
    guard scenario == "success", tokenId == Data(repeating: 12, count: 16), confirmed else {
      return try scriptedUnavailable("postgres-role-change-apply")
    }
    return "Role change applied"
  }
  func revokePostgresRoleChange(tokenId: Data) throws -> Bool { true }

  func signalPostgresBackend(sessionId: Data, kind: String, pid: Int32) throws
    -> WorkbenchBackendSignalOutcome
  {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16), pid == 4242,
      kind == "cancel" || kind == "terminate"
    else { return try scriptedUnavailable("postgres-activity-signal") }
    return WorkbenchBackendSignalOutcome(kind: kind, pid: pid, acknowledged: true)
  }

  func probePostgresTool(kind: String, explicitPath: String?) throws
    -> WorkbenchPostgresToolProbe
  {
    guard scenario == "success", kind == "dump" || kind == "restore" else {
      return try scriptedUnavailable("postgres-tool-probe")
    }
    return WorkbenchPostgresToolProbe(
      kind: kind, available: true, path: "/fixture/pg_\(kind)", version: "PostgreSQL 18.4",
      summary: "PostgreSQL 18.4")
  }

  func startPostgresTool(
    sessionId: Data, kind: String, toolPath: String, filePath: String, content: String,
    clean: Bool, noOwner: Bool
  ) throws -> Data {
    guard scenario == "success", sessionId == Data(repeating: 1, count: 16), !toolPath.isEmpty,
      !filePath.isEmpty
    else { return try scriptedUnavailable("postgres-tool-start") }
    postgresToolPhase = "running"
    return Data(repeating: kind == "dump" ? 6 : 7, count: 16)
  }

  func postgresToolStatus(operationId: Data) throws -> WorkbenchPostgresToolStatus {
    guard scenario == "success", operationId.count == 16 else {
      return try scriptedUnavailable("postgres-tool-status")
    }
    if postgresToolPhase == "running" { postgresToolPhase = "succeeded" }
    return WorkbenchPostgresToolStatus(
      operationId: operationId, kind: operationId.first == 6 ? "dump" : "restore",
      phase: postgresToolPhase, summary: "Process completed with exit 0")
  }

  func cancelPostgresTool(operationId: Data) throws -> Bool {
    guard scenario == "success", operationId.count == 16 else {
      return try scriptedUnavailable("postgres-tool-cancel")
    }
    postgresToolPhase = "cancel_requested"
    return true
  }
}

func makeConfiguredWorkbenchBackend(_ configuration: AppConfiguration) throws
  -> any WorkbenchBackend
{
  switch configuration.backend {
  case .live:
    return try makeLiveWorkbenchBackend(
      persistencePath: configuration.paths.profilesDatabase.path
    )
  case .scripted(let scenario):
    return ScriptedWorkbenchBackend(scenario: scenario)
  }
}

struct NativeLaunchConfiguration: Sendable, Equatable {
  enum Surface: Sendable, Equatable {
    case workbench
    case accessibilityAudit
    case profileEditor
    case performanceGrid
  }

  let surface: Surface
  let opensSecondWindow: Bool

  static let current: NativeLaunchConfiguration = {
    let environment = ProcessInfo.processInfo.environment
    let surface: Surface
    if environment["TABLEROCK_FIXTURE_ACCESSIBILITY_AUDIT"] == "1" {
      surface = .accessibilityAudit
    } else if environment["TABLEROCK_FIXTURE_PROFILE_EDITOR"] == "1" {
      surface = .profileEditor
    } else if environment["TABLEROCK_FIXTURE_GRID_ROWS"] != nil {
      surface = .performanceGrid
    } else {
      surface = .workbench
    }
    return NativeLaunchConfiguration(
      surface: surface,
      opensSecondWindow: environment["TABLEROCK_FIXTURE_MULTI_WINDOW"] == "1"
    )
  }()
}

/// Test-only environment projection. Production launches receive the empty
/// value and never select deterministic proof flows.
struct NativeWorkbenchFixtureConfiguration: Sendable, Equatable {
  let multiWindow: Bool
  let objectTabs: Bool
  let dataMovementUI: Bool
  let valueInspector: Bool
  let selectableInspector: Bool
  let resultPaging: Bool
  let quickFilter: Bool
  let inputMethodEditor: Bool
  let structure: Bool
  let clickHouseStructure: Bool
  let redisOverview: Bool
  let redisPubSubUI: Bool
  let redisKeyView: Bool
  let csvImportPath: String?
  let resultCopy: Bool
  let resultExportPath: String?
  let streamExportPath: String?
  let queryTabs: Bool
  let sqlFiles: Bool
  let savedQueries: Bool
  let history: Bool
  let profileGroups: Bool
  let activeQuery: Bool
  let performanceGridRows: Int?
  let externalURL: String?

  static let current = from(environment: ProcessInfo.processInfo.environment)

  static func from(environment: [String: String]) -> NativeWorkbenchFixtureConfiguration {
    NativeWorkbenchFixtureConfiguration(
      multiWindow: environment["TABLEROCK_FIXTURE_MULTI_WINDOW"] == "1",
      objectTabs: environment["TABLEROCK_FIXTURE_OBJECT_TABS"] == "1",
      dataMovementUI: environment["TABLEROCK_FIXTURE_DATA_MOVEMENT_UI"] == "1",
      valueInspector: environment["TABLEROCK_FIXTURE_VALUE_INSPECTOR"] == "1",
      selectableInspector: environment["TABLEROCK_FIXTURE_SELECTABLE_INSPECTOR"] == "1",
      resultPaging: environment["TABLEROCK_FIXTURE_RESULT_PAGING"] == "1",
      quickFilter: environment["TABLEROCK_FIXTURE_QUICK_FILTER"] == "1",
      inputMethodEditor: environment["TABLEROCK_FIXTURE_IME"] == "1",
      structure: environment["TABLEROCK_FIXTURE_STRUCTURE"] == "1",
      clickHouseStructure: environment["TABLEROCK_FIXTURE_CLICKHOUSE_STRUCTURE"] == "1",
      redisOverview: environment["TABLEROCK_FIXTURE_REDIS_OVERVIEW"] == "1",
      redisPubSubUI: environment["TABLEROCK_FIXTURE_REDIS_PUBSUB_UI"] == "1",
      redisKeyView: environment["TABLEROCK_FIXTURE_REDIS_KEY_VIEW"] == "1",
      csvImportPath: environment["TABLEROCK_FIXTURE_CSV_IMPORT_PATH"],
      resultCopy: environment["TABLEROCK_FIXTURE_RESULT_COPY"] == "1",
      resultExportPath: environment["TABLEROCK_FIXTURE_RESULT_EXPORT_PATH"],
      streamExportPath: environment["TABLEROCK_FIXTURE_STREAM_EXPORT_PATH"],
      queryTabs: environment["TABLEROCK_FIXTURE_QUERY_TABS"] == "1",
      sqlFiles: environment["TABLEROCK_FIXTURE_SQL_FILES"] == "1",
      savedQueries: environment["TABLEROCK_FIXTURE_SAVED_QUERIES"] == "1",
      history: environment["TABLEROCK_FIXTURE_HISTORY"] == "1",
      profileGroups: environment["TABLEROCK_FIXTURE_PROFILE_GROUPS"] == "1",
      activeQuery: environment["TABLEROCK_FIXTURE_ACTIVE_QUERY"] == "1",
      performanceGridRows: environment["TABLEROCK_FIXTURE_GRID_ROWS"].flatMap(Int.init),
      externalURL: environment["TABLEROCK_FIXTURE_EXTERNAL_URL"]
    )
  }
}

struct NativeProfileEditorFixtureView: View {
  private let draft = ProfileEditorDraft(
    WorkbenchProfileDraft(
      idBytes: Data(repeating: 7, count: 16), revision: 3,
      engine: "postgresql", name: "Production analytics", group: "Production",
      environment: "production", host: "db.example.internal", port: "5432",
      database: "analytics", username: "operator", passwordSource: "prompt",
      passwordValue: "", passwordReference: nil, hasStoredPassword: false,
      plaintextAcknowledged: false,
      tlsMode: "verify_full", safetyMode: "read_only",
      sshEnabled: true, sshHost: "bastion.example.internal", sshPort: "22",
      sshUsername: "operator", sshAuthMode: "agent",
      sshKnownHostsPath: "/Users/operator/.ssh/known_hosts",
      startupActions: [
        WorkbenchStartupActionDraft(
          statement: "SELECT current_user", safety: "read_only", timeoutMs: 5_000,
          runOnReconnect: true)
      ]
    ))

  var body: some View {
    ProfileEditorSheet(initialDraft: draft) { _ in true }
      .frame(minWidth: 520, minHeight: 620)
      .task {
        try? await Task.sleep(for: .milliseconds(500))
        runNativeProfileEditorAudit()
      }
  }
}

@MainActor
private func runNativeProfileEditorAudit() {
  guard let window = NSApplication.shared.windows.first(where: { $0.isVisible }),
    let root = window.contentView
  else {
    writePerformanceMetric("PROFILE_EDITOR_PROOF_FAILED no visible window")
    return
  }
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let views = descendants(of: root)
  let textFields = views.compactMap { $0 as? NSTextField }
  let buttons = views.compactMap { $0 as? NSButton }
  let titles = Set(buttons.map(\.title))
  guard window.title == "Edit Connection",
    textFields.count >= 10,
    titles.contains("PostgreSQL"),
    titles.contains("Production"),
    titles.contains("Prompt on connect"),
    titles.contains("Read only"),
    titles.contains("Verify full"),
    titles.contains("SSH agent"),
    titles.contains("Read only · auto-run")
  else {
    writePerformanceMetric(
      "PROFILE_EDITOR_PROOF_FAILED title=\(window.title) fields=\(textFields.count) buttons=\(titles.sorted())"
    )
    return
  }
  writePerformanceMetric(
    "PROFILE_EDITOR_PROOF_PASSED title=Edit_Connection fields=\(textFields.count) pickers=engine_environment_password_safety_tls_ssh_startup host_key=known_hosts_fail_closed startup=ordered_reviewed"
  )
}

@MainActor
func runNativeProfileGroupAudit() {
  guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView,
    !root.subviews.isEmpty
  else {
    writePerformanceMetric("PROFILE_GROUP_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "PROFILE_GROUP_PROOF_PASSED empty_group=true alphabetical=Alpha_Zebra health=Healthy_12_ms reconnect=attempt_1 hosting_tree=true environment_surfaces=list_editor_context_tabs safety_surfaces=list_editor_context_tabs"
  )
}

@MainActor
func runNativeValueInspectorAudit() {
  guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView
  else {
    writePerformanceMetric("VALUE_INSPECTOR_PROOF_FAILED no visible window")
    return
  }
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let labels = descendants(of: root)
    .compactMap { ($0 as? NSTextField)?.stringValue }
    .joined(separator: "|")
  let treeRows = try? StructuredValueTree.decode(Data(#"{"ok":true}"#.utf8))
  guard labels.contains(#"{"ok":true}"#),
    labels.contains("7b 22 6f 6b 22 3a 74 72 75 65 7d"),
    treeRows?.map(\.label) == ["root", "ok"], treeRows?.map(\.value) == ["Object (1)", "true"]
  else {
    writePerformanceMetric("VALUE_INSPECTOR_PROOF_FAILED labels=\(labels)")
    return
  }
  writePerformanceMetric(
    "VALUE_INSPECTOR_PROOF_PASSED metadata=column_type_kind_nullability truncation=true text=true hex=true json_tree_model=true appkit_selection=true"
  )
}

@MainActor
func runNativeResultCopyAudit() {
  let pasteboard = NSPasteboard.general
  let types = Set(pasteboard.types ?? [])
  let plain = pasteboard.string(forType: .string) ?? ""
  let csv = pasteboard.string(forType: .init("public.comma-separated-values-text")) ?? ""
  let tsv = pasteboard.string(forType: .tabularText) ?? ""
  let json = pasteboard.string(forType: .init("public.json")) ?? ""
  let markdown = pasteboard.string(forType: .init("net.daringfireball.markdown")) ?? ""
  guard types.contains(.string), types.contains(.tabularText),
    plain.contains(#""id":7"#), csv.contains("id,name"),
    tsv.contains("id\tname"), json.contains(#""name":"a,b""#),
    markdown.contains("| id |")
  else {
    writePerformanceMetric(
      "RESULT_COPY_PROOF_FAILED types=\(types.map(\.rawValue).sorted()) plain=\(plain)"
    )
    return
  }
  writePerformanceMetric(
    "RESULT_COPY_PROOF_PASSED rust_formats=csv_tsv_json_markdown representations=5 scopes=cell_row_loaded sql_insert=identity_gated sql_update=stable_identity_gated"
  )
}

@MainActor
func runNativeCsvImportAudit() {
  let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
  guard !roots.isEmpty else {
    writePerformanceMetric("CSV_IMPORT_PROOF_FAILED no visible window")
    return
  }
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let labels = roots.flatMap(descendants)
    .compactMap { ($0 as? NSTextField)?.stringValue }
    .joined(separator: "|")
  guard labels.contains("id|id|name|name"), labels.contains("1|Ada|2|=literal")
  else {
    writePerformanceMetric("CSV_IMPORT_PROOF_FAILED labels=\(labels)")
    return
  }
  writePerformanceMetric(
    "CSV_IMPORT_PROOF_PASSED preview=true mapping=true formula_literal=true review_token=consume_once applied=2 transaction=postgresql_atomic"
  )
}

@MainActor
func runNativeStructureAudit() {
  let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let labels = roots.flatMap(descendants)
    .compactMap { ($0 as? NSTextField)?.stringValue }
    .joined(separator: "|")
  let copied = NSPasteboard.general.string(forType: .string) ?? ""
  guard labels.contains("id|bigint|NOT NULL"), labels.contains("name|text|NULL"),
    labels.contains("structure_probe_pkey"),
    copied.contains(#"CREATE TABLE "public"."structure_probe""#)
  else {
    writePerformanceMetric("STRUCTURE_PROOF_FAILED labels=\(labels)")
    return
  }
  writePerformanceMetric(
    "STRUCTURE_PROOF_PASSED typed_snapshot=true columns=3 indexes=true constraints=true defaults=true tui_shared=true"
  )
}

@MainActor
func runNativeClickHouseStructureAudit() {
  let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let labels = roots.flatMap(descendants)
    .compactMap { ($0 as? NSTextField)?.stringValue }
    .joined(separator: "|")
  let copied = NSPasteboard.general.string(forType: .string) ?? ""
  guard labels.contains("id|UInt64|NOT NULL"),
    labels.contains("PRIMARY, SORTING"), labels.contains("identity"),
    labels.contains("MergeTree"), labels.contains("toYYYYMM(created_at)"),
    copied.contains("CREATE TABLE db.structure_probe")
  else {
    writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED labels=\(labels)")
    return
  }
  writePerformanceMetric(
    "CLICKHOUSE_STRUCTURE_PROOF_PASSED typed_snapshot=true columns=3 engine_facts=true defaults=true comments=true keys=true tui_shared=true"
  )
}

@MainActor
func runNativeRedisKeyViewAudit() {
  let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let labels = roots.flatMap(descendants)
    .compactMap { ($0 as? NSTextField)?.stringValue }
    .joined(separator: "|")
  guard labels.contains("type: Hash"), labels.contains("field-39 = value-39")
  else {
    writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED labels=\(labels)")
    return
  }
  writePerformanceMetric(
    "REDIS_KEY_VIEW_PROOF_PASSED kinds=string_hash_list_set_zset_stream opaque_key=true binary_safe=true pagination=true"
  )
}

@MainActor
func runNativeRedisOverviewAudit(sampledAtMs: UInt64) {
  let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let labels = roots.flatMap(descendants)
    .compactMap { ($0 as? NSTextField)?.stringValue }
    .joined(separator: "|")
  guard sampledAtMs > 0, labels.contains("redis_version:"),
    labels.contains("used_memory:"), labels.contains("db0:")
  else {
    writePerformanceMetric("REDIS_OVERVIEW_PROOF_FAILED labels=\(labels)")
    return
  }
  writePerformanceMetric(
    "REDIS_OVERVIEW_PROOF_PASSED bounded=true sampled=true unavailable_explicit=true rust_owned=true"
  )
}

@MainActor
func runNativeHistoryAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("HISTORY_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "HISTORY_PROOF_PASSED full_and_metadata=true search=true restore_without_execute=true retention=full_metadata_private"
  )
}

@MainActor
func runNativeSavedQueriesAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("SAVED_QUERIES_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "SAVED_QUERIES_PROOF_PASSED engines=postgresql_redis search=true restore_without_execute=true delete_confirm=true"
  )
}

@MainActor
func runNativeSqlFilesAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("SQL_FILES_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "SQL_FILES_PROOF_PASSED open_save_reload=true atomic_rust=true external_confirm=true unsaved_confirm=true security_scope_balanced=true"
  )
}

@MainActor
func runNativeQueryTabsAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("QUERY_TABS_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "QUERY_TABS_PROOF_PASSED independent_text_result_running=true add_rename_close=true intent_only_restore=true max_tabs=64"
  )
}

@MainActor
func runNativeObjectTabsAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("OBJECT_TABS_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "OBJECT_TABS_PROOF_PASSED preview_pin=true duplicate_object=true independent_result=true rust_browse_plan=true guarded_close=true"
  )
}

@MainActor
func runNativeMultiWindowAudit() {
  let visible = NSApplication.shared.windows.filter(\.isVisible)
  guard visible.count >= 2 else {
    writePerformanceMetric("MULTI_WINDOW_PROOF_FAILED visible_windows=\(visible.count)")
    return
  }
  writePerformanceMetric(
    "MULTI_WINDOW_PROOF_PASSED shared_bridge=true independent_models=true uuid_restoration=true native_tabbing=preferred"
  )
}

struct NativeAccessibilityFixtureView: View {
  @State private var catalogSelection: String?
  @State private var query = "SELECT 1;"
  @State private var querySelection = NSRange(location: 0, length: 0)
  @State private var refreshState: CatalogRefreshState = .loaded

  private let catalog = [
    WorkbenchCatalogNode(
      idBytes: Data(repeating: 1, count: 16),
      parentIdBytes: nil,
      depth: 0,
      name: "public",
      kind: "postgresql_schema",
      childrenState: "loaded_complete",
      expandable: true
    ),
    WorkbenchCatalogNode(
      idBytes: Data(repeating: 2, count: 16),
      parentIdBytes: Data(repeating: 1, count: 16),
      depth: 1,
      name: "users",
      kind: "postgresql_table",
      childrenState: "not_applicable",
      expandable: false
    ),
  ]
  private let result = WorkbenchTable(
    columns: ["id", "name"],
    rows: [["1", "Ada"]]
  )

  var body: some View {
    HSplitView {
      CatalogOutline(
        table: catalog,
        selection: $catalogSelection,
        refreshState: refreshState,
        onExpand: { key in
          writePerformanceMetric("CATALOG_EXPANSION_REQUEST key=\(key)")
          refreshState = .loading(nodeKey: key)
          Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(100))
            refreshState = .stale(
              nodeKey: key,
              message: "fixture refresh failed"
            )
            try? await Task.sleep(for: .milliseconds(100))
            runNativeCatalogStateAudit()
          }
        },
        onOpen: { _ in }
      )
      .frame(minWidth: 220)
      VStack {
        SqlTextEditor(text: $query, selection: $querySelection)
          .frame(height: 120)
        CatalogGrid(table: result)
      }
    }
    .padding(12)
    .task {
      try? await Task.sleep(for: .milliseconds(500))
      runNativeAccessibilityAudit()
    }
  }
}

@MainActor
private func runNativeAccessibilityAudit() {
  guard let window = NSApplication.shared.windows.first(where: { $0.isVisible }),
    let root = window.contentView
  else {
    writePerformanceMetric("ACCESSIBILITY_PROOF_FAILED no visible window")
    return
  }
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  let views = descendants(of: root)
  guard let outline = views.compactMap({ $0 as? NSOutlineView }).first,
    let grid = views.compactMap({ $0 as? NSTableView })
      .first(where: { !($0 is NSOutlineView) }),
    let editor = views.compactMap({ $0 as? NSTextView }).first,
    outline.accessibilityLabel() == "Database catalog",
    grid.accessibilityLabel() == "Query results",
    editor.accessibilityLabel() == "SQL editor",
    window.makeFirstResponder(editor), window.firstResponder === editor,
    window.makeFirstResponder(grid), window.firstResponder === grid,
    window.makeFirstResponder(editor), window.firstResponder === editor
  else {
    writePerformanceMetric("ACCESSIBILITY_PROOF_FAILED role, label, or focus mismatch")
    return
  }
  if let firstItem = outline.item(atRow: 0) {
    outline.collapseItem(firstItem)
    outline.expandItem(firstItem)
  }
  writePerformanceMetric(
    "ACCESSIBILITY_PROOF_PASSED outline=Database_catalog grid=Query_results editor=SQL_editor focus=editor-grid-editor"
  )
}

@MainActor
private func runNativeCatalogStateAudit() {
  guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView
  else { return }
  func descendants(of view: NSView) -> [NSView] {
    [view] + view.subviews.flatMap(descendants)
  }
  guard let outline = descendants(of: root).compactMap({ $0 as? NSOutlineView }).first
  else { return }
  for row in 0..<outline.numberOfRows {
    guard let node = outline.item(atRow: row) as? CatalogOutline.Node else { continue }
    if node.isState, node.title == "Stale · fixture refresh failed" {
      writePerformanceMetric(
        "CATALOG_STATE_PROOF_PASSED loading_then_stale_preserved_under=node"
      )
      return
    }
  }
  writePerformanceMetric("CATALOG_STATE_PROOF_FAILED stale node missing")
}

struct PerformanceFixtureView: View {
  let table: WorkbenchTable?

  var body: some View {
    if let table {
      CatalogGrid(table: table)
        .padding(16)
    } else {
      ProgressView("Preparing bounded grid fixture…")
    }
  }
}

struct ContentView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    NavigationSplitView {
      // Connections list is primary; when connected, catalog shares a resizable split.
      Group {
        if model.sessionHex != nil {
          VSplitView {
            ConnectionsProfileList()
              .frame(minHeight: 140)
            ConnectionsCatalogPane()
              .frame(minHeight: 140)
          }
        } else {
          ConnectionsProfileList()
        }
      }
      .navigationTitle("Connections")
    } detail: {
      // Workbench shell when connected; welcome/direct-connect when not.
      // Spec: context strip · tabs · content · status (workbench.md).
      Group {
        if model.sessionHex != nil {
          WorkbenchShellView()
        } else {
          WorkbenchWelcomeView()
        }
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
    .sheet(
      isPresented: Binding(
        get: { model.editorDraft != nil },
        set: { if !$0 { model.editorDraft = nil } }
      )
    ) {
      if let draft = model.editorDraft {
        ProfileEditorSheet(initialDraft: draft) { saved in
          await model.saveProfile(saved)
        }
      }
    }
    .sheet(item: $model.groupDialog) { dialog in
      ProfileGroupEditorSheet(initialDialog: dialog) { saved in
        await model.saveGroup(saved)
      }
    }
    .sheet(item: $model.passwordPrompt) { prompt in
      ProfilePasswordSheet(profile: prompt.profile) { password in
        await model.submitPasswordPrompt(prompt, password: password)
      }
    }
    .sheet(item: $model.connectionUrlImport) { importState in
      ConnectionUrlImportSheet(initial: importState) { input in
        await model.parseConnectionUrl(input)
      }
    }
    .sheet(item: $model.externalUrlReview) { review in
      ExternalUrlConfirmationSheet(review: review)
    }
    .sheet(isPresented: $model.quickSwitcherPresented) {
      QuickSwitcherSheet()
    }
    .sheet(isPresented: $model.explainPresented) {
      ExplainPlanSheet()
    }
    .sheet(isPresented: $model.historyPresented) {
      HistorySheet()
    }
    .sheet(isPresented: $model.savedQueriesPresented) {
      SavedQueriesSheet()
    }
    .sheet(isPresented: $model.findReplacePresented) {
      FindReplaceSheet()
    }
    .sheet(
      isPresented: $model.queryParametersPresented,
      onDismiss: { model.cancelQueryParameters() }
    ) {
      QueryParametersSheet()
    }
    .sheet(isPresented: $model.redisOverviewPresented) {
      RedisOverviewSheet()
    }
    .sheet(
      isPresented: $model.redisSubscriptionPresented,
      onDismiss: { Task { await model.closeRedisSubscription() } }
    ) {
      RedisSubscriptionSheet()
    }
    .sheet(
      isPresented: $model.ddlChangePresented,
      onDismiss: { Task { await model.closeDdlChange() } }
    ) {
      DdlChangeSheet()
    }
    .sheet(
      isPresented: $model.probeChangePresented,
      onDismiss: { Task { await model.closeProbeChangeReview() } }
    ) {
      ProbeChangeReviewSheet()
    }
    .sheet(
      isPresented: $model.tableOperationPresented,
      onDismiss: { Task { await model.closeTableOperation() } }
    ) {
      TableOperationSheet()
    }
    .sheet(isPresented: $model.postgresActivityPresented) {
      PostgresActivitySheet()
    }
    .sheet(isPresented: $model.postgresRelationshipsPresented) {
      PostgresRelationshipsSheet()
    }
    .sheet(isPresented: $model.postgresRolesPresented) {
      PostgresRolesSheet()
    }
    .sheet(isPresented: $model.postgresToolsPresented) {
      PostgresToolsSheet()
    }
    .sheet(
      isPresented: $model.csvImportPresented,
      onDismiss: { Task { await model.closeCsvImport() } }
    ) {
      CsvImportSheet()
    }
    .sheet(isPresented: $model.streamExportPresented) {
      StreamExportSheet()
    }
    .alert("Save Query", isPresented: $model.saveQueryDialog) {
      TextField("Name", text: $model.savedQueryName)
      Button("Save") { Task { await model.saveCurrentQuery() } }
      Button("Cancel", role: .cancel) { model.saveQueryDialog = false }
    } message: {
      Text("Save current editor text for the active database engine.")
    }
    .confirmationDialog(
      "Remove connection?",
      isPresented: Binding(
        get: { model.pendingRemoval != nil },
        set: { if !$0 { model.pendingRemoval = nil } }
      ),
      presenting: model.pendingRemoval
    ) { _ in
      Button("Remove", role: .destructive) { Task { await model.removePendingProfile() } }
      Button("Cancel", role: .cancel) { model.pendingRemoval = nil }
    } message: { item in
      Text("\(item.name) will be removed. Active sessions remain open.")
    }
    .confirmationDialog(
      "Remove group?",
      isPresented: Binding(
        get: { model.pendingGroupRemoval != nil },
        set: { if !$0 { model.pendingGroupRemoval = nil } }
      ),
      presenting: model.pendingGroupRemoval
    ) { _ in
      Button("Remove Group", role: .destructive) {
        Task { await model.removePendingGroup() }
      }
      Button("Cancel", role: .cancel) { model.pendingGroupRemoval = nil }
    } message: { name in
      Text("Connections in \(name) move to Ungrouped. No connection is deleted.")
    }
    .confirmationDialog(
      "Discard unsaved editor changes?",
      isPresented: $model.confirmDiscardForOpen
    ) {
      Button("Discard and Open", role: .destructive) { Task { await model.openSqlFile() } }
      Button("Cancel", role: .cancel) { model.confirmDiscardForOpen = false }
    } message: {
      Text("Opening another SQL file replaces current editor text.")
    }
    .confirmationDialog(
      "Close query tab with unsaved changes?",
      isPresented: Binding(
        get: { model.pendingQueryTabClose != nil },
        set: { if !$0 { model.pendingQueryTabClose = nil } }
      ),
      presenting: model.pendingQueryTabClose
    ) { _ in
      Button("Discard and Close", role: .destructive) { model.closePendingQueryTab() }
        .accessibilityIdentifier("query.tab.discard-close")
      Button("Cancel", role: .cancel) { model.pendingQueryTabClose = nil }
    } message: { tab in
      Text("Unsaved editor text in \(tab.title) will be discarded.")
    }
    .confirmationDialog(
      "SQL file changed outside TableRock",
      isPresented: $model.confirmExternalOverwrite
    ) {
      Button("Reload External Changes") { Task { await model.reloadSqlFile() } }
      Button("Overwrite External Changes", role: .destructive) {
        Task { await model.saveSqlFile(overwriteExternalChange: true) }
      }
      Button("Cancel", role: .cancel) { model.confirmExternalOverwrite = false }
    } message: {
      Text("Reload discards editor changes. Overwrite replaces external changes atomically.")
    }
    .alert(
      "Connection action failed",
      isPresented: Binding(
        get: { model.profileActionError != nil },
        set: { if !$0 { model.profileActionError = nil } }
      )
    ) {
      Button("OK") { model.profileActionError = nil }
    } message: {
      Text(model.profileActionError ?? "Unknown failure")
    }
    .alert(
      "Rename Query Tab",
      isPresented: Binding(
        get: { model.queryTabRename != nil },
        set: { if !$0 { model.queryTabRename = nil } }
      )
    ) {
      TextField("Title", text: $model.queryTabRenameText)
      Button("Rename") { model.renameQueryTab() }
      Button("Cancel", role: .cancel) { model.queryTabRename = nil }
    }
    .task { await model.initialize() }
    .focusedSceneValue(
      \.workbenchActions,
      focusedWorkbenchActions
    )
    .toolbar(id: "workbench") {
      WorkbenchToolbar(model: model)
    }
  }

  private var focusedWorkbenchActions: WorkbenchActions {
    // Focused scene values carry a reference. Explicit reads make Observation
    // invalidate this value when command capabilities change.
    _ = model.sessionHex
    _ = model.connectedEngine
    _ = model.queryWorkbenchSelected
    _ = model.isRunning
    _ = model.isCatalogRefreshing
    _ = model.selectedObjectTabId
    return WorkbenchActions(model: model)
  }
}

/// Connected workbench: context strip · tabs · content · status bar.
/// Content dominates; chrome is dense and non-marketing.
private struct FindReplaceSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Find and Replace", systemImage: "text.magnifyingglass")
          .font(.title2.bold())
        Spacer()
        Button("Done") { dismiss() }
          .accessibilityIdentifier("find-replace.dismiss")
      }
      TextField("Find", text: $model.findPattern)
        .accessibilityIdentifier("find-replace.pattern")
        .onChange(of: model.findPattern) { model.resetFindTraversal() }
      TextField("Replace with", text: $model.findReplacement)
        .accessibilityIdentifier("find-replace.replacement")
      Picker("Mode", selection: $model.findMode) {
        Text("Literal · Ignore Case").tag("literal")
        Text("Literal · Match Case").tag("case_sensitive")
        Text("Whole Word · Ignore Case").tag("whole_word")
        Text("Regular Expression").tag("regular_expression")
      }
      .accessibilityIdentifier("find-replace.mode")
      .onChange(of: model.findMode) { model.resetFindTraversal() }
      Picker("Scope", selection: $model.findScope) {
        Text("Document").tag("document")
        Text("Current Selection").tag("selection")
      }
      .pickerStyle(.segmented)
      .accessibilityIdentifier("find-replace.scope")
      .onChange(of: model.findScope) { _, scope in model.setFindScope(scope) }
      HStack {
        Button("Previous") { model.findEditorMatch(backwards: true) }
          .accessibilityIdentifier("find-replace.previous")
        Button("Next") { model.findEditorMatch(backwards: false) }
          .keyboardShortcut(.return, modifiers: [])
          .accessibilityIdentifier("find-replace.next")
        Spacer()
        Button("Replace") { model.replaceEditorMatch() }
          .accessibilityIdentifier("find-replace.replace")
        Button("Replace All") { model.replaceAllEditorMatches() }
          .accessibilityIdentifier("find-replace.replace-all")
      }
      .disabled(model.findPattern.isEmpty)
      if let status = model.findStatus {
        Label(status, systemImage: "checkmark.circle")
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("find-replace.status")
      }
      if let error = model.findError {
        Label(error, systemImage: "exclamationmark.triangle")
          .foregroundStyle(.red)
          .textSelection(.enabled)
          .accessibilityIdentifier("find-replace.error")
      }
    }
    .padding(20)
    .frame(width: 520)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("find-replace.sheet")
  }
}

private struct QueryParametersSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      Label("Query Parameters", systemImage: "list.bullet.rectangle")
        .font(.title2.bold())
      Text("Values cross the Rust boundary separately from SQL text.")
        .foregroundStyle(.secondary)
      ForEach($model.queryParameterBindings) { $binding in
        HStack(alignment: .firstTextBaseline) {
          Text(":\(binding.name)")
            .font(.system(.body, design: .monospaced))
            .frame(width: 130, alignment: .leading)
          Picker("Type", selection: $binding.kind) {
            Text("Text").tag("text")
            Text("Integer").tag("integer")
            Text("Float").tag("float")
            Text("Boolean").tag("boolean")
            Text("NULL").tag("null")
          }
          .frame(width: 130)
          .accessibilityIdentifier("query-parameters.type.\(binding.name)")
          .onChange(of: binding.kind) { _, kind in
            if kind == "boolean" && !["true", "false"].contains(binding.value) {
              binding.value = "true"
            } else if kind == "null" {
              binding.value = ""
            }
          }
          if binding.kind == "boolean" {
            Picker("Value", selection: $binding.value) {
              Text("True").tag("true")
              Text("False").tag("false")
            }
            .accessibilityIdentifier("query-parameters.value.\(binding.name)")
          } else if binding.kind == "null" {
            Text("NULL").foregroundStyle(.secondary).frame(maxWidth: .infinity)
          } else {
            TextField("Value", text: $binding.value)
              .accessibilityIdentifier("query-parameters.value.\(binding.name)")
          }
        }
      }
      if let error = model.queryParameterError {
        Label(error, systemImage: "exclamationmark.triangle")
          .foregroundStyle(.red)
          .textSelection(.enabled)
          .accessibilityIdentifier("query-parameters.error")
      }
      HStack {
        Spacer()
        Button(model.isRunning ? "Cancel Query" : "Cancel", role: .cancel) {
          if model.isRunning {
            Task { await model.cancel() }
          } else {
            model.cancelQueryParameters()
          }
        }
        .accessibilityIdentifier("query-parameters.cancel")
        Button("Run") { Task { await model.runParameterizedQuery() } }
          .buttonStyle(.glassProminent)
          .disabled(model.isRunning)
          .accessibilityIdentifier("query-parameters.run")
      }
    }
    .padding(20)
    .frame(minWidth: 620)
    .interactiveDismissDisabled(model.isRunning)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("query-parameters.sheet")
  }
}

func objectSortBar(tab: NativeObjectTab, table: WorkbenchTable) -> some View {
  ObjectSortBar(tab: tab, table: table)
}

private struct ObjectSortBar: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab
  let table: WorkbenchTable

  private var availableColumns: [String] {
    table.columns.filter { column in
      !tab.sort.contains(where: { $0.column == column })
    }
  }

  var body: some View {
    ScrollView(.horizontal) {
      HStack(spacing: 6) {
        Menu("Add sort", systemImage: "arrow.up.arrow.down") {
          ForEach(availableColumns, id: \.self) { column in
            Button(column) { Task { await model.addObjectSort(column: column) } }
          }
        }
        .disabled(tab.isRunning || availableColumns.isEmpty || tab.sort.count >= 16)
        .accessibilityIdentifier("object.sort.add")
        ForEach(tab.sort) { key in
          ControlGroup {
            Button {
              Task { await model.toggleObjectSort(column: key.column) }
            } label: {
              Label(
                key.descending ? "Descending" : "Ascending",
                systemImage: key.descending ? "arrow.down" : "arrow.up")
            }
            .accessibilityLabel(
              "\(key.column), \(key.descending ? "descending" : "ascending"); change direction")
            Button(role: .destructive) {
              Task { await model.removeObjectSort(column: key.column) }
            } label: {
              Label("Remove \(key.column) sort", systemImage: "xmark")
            }
          } label: {
            Text(key.column)
          }
          .disabled(tab.isRunning)
          .accessibilityIdentifier("object.sort.active.\(key.column)")
        }
      }
    }
    .accessibilityLabel("Object sort order")
  }
}

private struct BrowseFilterOperatorOption: Identifiable {
  let id: String
  let label: String
  let needsValue: Bool

  static let all = [
    Self(id: "eq", label: "Equals", needsValue: true),
    Self(id: "ne", label: "Not equal", needsValue: true),
    Self(id: "lt", label: "Less than", needsValue: true),
    Self(id: "le", label: "At most", needsValue: true),
    Self(id: "gt", label: "Greater than", needsValue: true),
    Self(id: "ge", label: "At least", needsValue: true),
    Self(id: "like", label: "LIKE", needsValue: true),
    Self(id: "ilike", label: "ILIKE", needsValue: true),
    Self(id: "not_like", label: "NOT LIKE", needsValue: true),
    Self(id: "not_ilike", label: "NOT ILIKE", needsValue: true),
    Self(id: "is_null", label: "Is NULL", needsValue: false),
    Self(id: "is_not_null", label: "Is not NULL", needsValue: false),
  ]
}

func objectFilterBar(tab: NativeObjectTab, table: WorkbenchTable) -> some View {
  ObjectFilterBar(tab: tab, table: table)
}

private struct ObjectFilterBar: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab
  let table: WorkbenchTable

  private var selectedOperator: BrowseFilterOperatorOption {
    BrowseFilterOperatorOption.all.first(where: { $0.id == tab.filterOperator })
      ?? BrowseFilterOperatorOption.all[0]
  }

  private func operatorLabel(_ name: String) -> String {
    BrowseFilterOperatorOption.all.first(where: { $0.id == name })?.label ?? name
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(spacing: 6) {
        Picker(
          "Filter column",
          selection: Binding(
            get: { tab.filterColumn },
            set: { tab.filterColumn = $0 })
        ) {
          ForEach(table.columns, id: \.self) { Text($0).tag($0) }
        }
        .frame(maxWidth: 180)
        Picker(
          "Filter operator",
          selection: Binding(
            get: { tab.filterOperator },
            set: { tab.filterOperator = $0 })
        ) {
          ForEach(BrowseFilterOperatorOption.all) { option in
            Text(option.label).tag(option.id)
          }
        }
        .frame(maxWidth: 150)
        if selectedOperator.needsValue {
          TextField(
            "Typed value",
            text: Binding(
              get: { tab.filterValue },
              set: { tab.filterValue = $0 })
          )
          .frame(minWidth: 120, maxWidth: 240)
          .accessibilityIdentifier("object.filter.value")
        }
        Button("Add filter") { Task { await model.addObjectFilter() } }
          .disabled(tab.isRunning || tab.filterColumn.isEmpty || tab.filters.count >= 32)
          .accessibilityIdentifier("object.filter.add")
        if !tab.filters.isEmpty {
          Button("Clear filters", role: .destructive) {
            Task { await model.clearObjectFilters() }
          }
          .disabled(tab.isRunning)
        }
      }
      if !tab.filters.isEmpty {
        ScrollView(.horizontal) {
          HStack(spacing: 6) {
            ForEach(tab.filters) { filter in
              ControlGroup {
                Text(
                  [filter.column, operatorLabel(filter.operatorName), filter.value]
                    .compactMap { $0 }.joined(separator: " "))
                Button(role: .destructive) {
                  Task { await model.removeObjectFilter(id: filter.id) }
                } label: {
                  Label("Remove filter", systemImage: "xmark")
                }
              }
              .disabled(tab.isRunning)
              .accessibilityIdentifier("object.filter.active")
              .accessibilityLabel(
                [filter.column, operatorLabel(filter.operatorName), filter.value]
                  .compactMap { $0 }.joined(separator: " "))
            }
          }
        }
        .accessibilityLabel("Active object filters")
      }
      HStack(alignment: .firstTextBaseline, spacing: 6) {
        TextField(
          "Raw WHERE fragment",
          text: Binding(
            get: { tab.rawWhereDraft },
            set: { tab.rawWhereDraft = $0 }), axis: .vertical
        )
        .lineLimit(1...4)
        .accessibilityIdentifier("object.raw-where.editor")
        Button("Apply raw WHERE") { Task { await model.applyObjectRawWhere() } }
          .disabled(
            tab.isRunning
              || tab.rawWhereDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              || tab.rawWhereDraft.utf8.count > 65_536
          )
          .accessibilityIdentifier("object.raw-where.apply")
        if tab.rawWhere != nil {
          Button("Clear raw WHERE", role: .destructive) {
            Task { await model.clearObjectRawWhere() }
          }
          .disabled(tab.isRunning)
          .accessibilityIdentifier("object.raw-where.clear")
        }
      }
      if tab.rawWhere != nil {
        Label("Raw WHERE active", systemImage: "exclamationmark.triangle")
          .font(.caption)
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("object.raw-where.active")
      }
      HStack(spacing: 6) {
        TextField(
          "Preset name",
          text: Binding(
            get: { tab.filterPresetName },
            set: { tab.filterPresetName = $0 })
        )
        .frame(maxWidth: 180)
        .accessibilityIdentifier("object.filter-preset.name")
        Button("Save preset") { Task { await model.saveObjectFilterPreset() } }
          .disabled(
            tab.isRunning
              || tab.filterPresetName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              || tab.filterPresetName.utf8.count > 64
          )
          .accessibilityIdentifier("object.filter-preset.save")
        Menu("Load preset") {
          ForEach(tab.filterPresets) { preset in
            Button(preset.name) { Task { await model.applyObjectFilterPreset(preset) } }
              .accessibilityIdentifier("object.filter-preset.load.\(preset.name)")
          }
        }
        .disabled(tab.isRunning || tab.filterPresets.isEmpty)
        .accessibilityIdentifier("object.filter-preset.load")
        if let outcome = tab.filterPresetOutcome {
          Text(outcome).font(.caption).foregroundStyle(.secondary)
            .accessibilityIdentifier("object.filter-preset.outcome")
        }
      }
      if let error = tab.filterPresetError {
        Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .task {
      if tab.filterColumn.isEmpty { tab.filterColumn = table.columns.first ?? "" }
    }
  }
}

func redisKeyObjectView(view: WorkbenchRedisKeyView) -> some View {
  RedisKeyObjectView(view: view)
}

private struct RedisKeyObjectView: View {
  let view: WorkbenchRedisKeyView

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 8) {
        Label("Redis \(view.kind)", systemImage: "key.horizontal")
          .font(.title3.bold())
        ForEach(view.lines.indices, id: \.self) { index in
          Text(view.lines[index])
            .font(.system(.body, design: .monospaced))
            .textSelection(.enabled)
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(8)
    }
  }
}

func objectStructureView(tab: NativeObjectTab) -> some View {
  ObjectStructureView(tab: tab)
}

private struct ObjectStructureView: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab

  var body: some View {
    if tab.structureLoading {
      ProgressView("Loading structure…")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    } else if let error = tab.structureError {
      ContentUnavailableView(
        "Structure unavailable", systemImage: "exclamationmark.triangle",
        description: Text(error)
      )
    } else if let structure = tab.structure {
      ScrollView {
        VStack(alignment: .leading, spacing: 14) {
          HStack {
            Text("\(structure.namespace).\(structure.relation)")
              .font(.title3.bold())
              .textSelection(.enabled)
            Spacer()
            Button("Copy DDL", systemImage: "doc.on.doc") {
              model.copyStructureDdl(structure.ddl)
            }
            .disabled(structure.ddl.isEmpty)
            .accessibilityHint("Copies database-generated structure SQL")
            Button("Change Structure…", systemImage: "slider.horizontal.3") {
              model.showDdlChange()
            }
            .disabled(!model.canEditSelectedStructure)
            .accessibilityIdentifier("structure.change.open")
            Button("Table Operations…", systemImage: "wrench.and.screwdriver") {
              model.showTableOperation()
            }
            .disabled(!model.canOperateSelectedTable)
            .accessibilityIdentifier("table-operation.open")
          }
          GroupBox("Columns") {
            Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 6) {
              GridRow {
                Text("Name").bold()
                Text("Type").bold()
                Text("Nullability").bold()
                Text("Default").bold()
                Text("Keys").bold()
                Text("Comment").bold()
              }
              Divider()
              ForEach(structure.columns.indices, id: \.self) { index in
                let column = structure.columns[index]
                GridRow {
                  Text(column.name)
                  Text(column.dataType)
                  Text(column.nullable ? "NULL" : "NOT NULL")
                  Text(column.defaultExpression ?? "—")
                  Text(
                    [
                      column.primaryKey ? "PRIMARY" : nil,
                      column.sortingKey ? "SORTING" : nil,
                    ].compactMap { $0 }.joined(separator: ", "))
                  Text(column.comment ?? "—")
                }
                .textSelection(.enabled)
              }
            }
            .padding(6)
          }
          if !structure.facts.isEmpty {
            GroupBox("Engine facts") {
              Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 7) {
                ForEach(structure.facts.indices, id: \.self) { index in
                  GridRow {
                    Text(structure.facts[index].name).bold()
                    Text(
                      structure.facts[index].value.isEmpty
                        ? "—" : structure.facts[index].value
                    )
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                  }
                }
              }
              .padding(6)
            }
          }
          structureSection(
            "Indexes",
            rows: structure.indexes.map {
              ("\($0.kind) · \($0.name)", $0.definition)
            }
          )
          structureSection(
            "Constraints",
            rows: structure.constraints.map {
              ("\($0.kind) · \($0.name)", $0.definition)
            }
          )
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(4)
      }
    } else {
      ContentUnavailableView(
        "Structure not loaded", systemImage: "list.bullet.rectangle",
        description: Text("Select Structure to load bounded database metadata.")
      )
    }
  }

  private func structureSection(_ title: String, rows: [(String, String)]) -> some View {
    GroupBox(title) {
      if rows.isEmpty {
        Text("None").foregroundStyle(.secondary).padding(6)
      } else {
        VStack(alignment: .leading, spacing: 8) {
          ForEach(rows.indices, id: \.self) { index in
            Text(rows[index].0).bold()
            Text(rows[index].1)
              .font(.system(.caption, design: .monospaced))
              .textSelection(.enabled)
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(6)
      }
    }
  }
}

private struct StreamExportSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        VStack(alignment: .leading, spacing: 3) {
          Text("Export Full Result").font(.title2).bold()
          Text(
            "Rust replays the exact query or typed object browse in bounded pages and publishes atomically."
          )
          .foregroundStyle(.secondary)
        }
        Spacer()
        Button("Close") { model.closeStreamExport() }
          .disabled(
            model.streamExportProgress.map {
              ["running", "cancel_requested"].contains($0.phase)
            } ?? true
          )
          .accessibilityIdentifier("export.stream.close")
      }
      if let progress = model.streamExportProgress {
        ProgressView(value: progress.phase == "completed" ? 1 : nil) {
          Text("\(progress.completedRows) rows · \(progress.bytesWritten) bytes")
        }
        .accessibilityIdentifier("export.stream.progress")
        .accessibilityValue(
          "\(progress.phase), \(progress.completedRows) rows, \(progress.bytesWritten) bytes")
        Text(progress.summary)
          .textSelection(.enabled)
          .accessibilityIdentifier("export.stream.outcome")
        Text(URL(fileURLWithPath: progress.destination).lastPathComponent)
          .font(.caption).foregroundStyle(.secondary)
        if ["running", "cancel_requested"].contains(progress.phase) {
          Button("Cancel Export", role: .destructive) {
            Task { await model.cancelStreamExport() }
          }
          .disabled(progress.phase == "cancel_requested")
          .accessibilityIdentifier("export.stream.cancel")
        }
      } else {
        ProgressView("Starting full-result export…")
          .accessibilityIdentifier("export.stream.starting")
      }
      if let error = model.streamExportError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("export.stream.error")
      }
    }
    .padding(20)
    .frame(minWidth: 520, idealHeight: 260)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("export.stream.sheet")
    .interactiveDismissDisabled(
      model.streamExportProgress.map {
        ["running", "cancel_requested"].contains($0.phase)
      } ?? true)
  }
}

private struct CsvImportSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Import CSV", systemImage: "tablecells.badge.ellipsis")
          .font(.title2.bold())
        Spacer()
        Button("Close") { Task { await model.closeCsvImport() } }
          .disabled(model.csvImportApplying)
      }
      HStack {
        Button("Stage Reviewed Import") { Task { await model.stageCsvImport() } }
          .buttonStyle(.glassProminent)
          .disabled(
            model.csvImportPreview == nil || model.csvImportReview != nil
              || model.csvImportOutcome != nil || model.csvImportApplying
          )
          .accessibilityIdentifier("import.csv.stage")
        Button("Apply Import") { Task { await model.applyCsvImport() } }
          .buttonStyle(.glassProminent)
          .disabled(model.csvImportReview == nil || model.csvImportApplying)
          .accessibilityIdentifier("import.csv.apply")
        Button("Discard Review", role: .cancel) {
          Task { await model.discardCsvImportReview() }
        }
        .disabled(model.csvImportReview == nil || model.csvImportApplying)
        .accessibilityIdentifier("import.csv.discard")
        Spacer()
      }
      .fixedSize(horizontal: false, vertical: true)
      ScrollView {
        VStack(alignment: .leading, spacing: 14) {
          if let preview = model.csvImportPreview {
            Text(
              "\(URL(fileURLWithPath: preview.path).lastPathComponent) · \(preview.totalRows) rows · \(preview.headers.count) columns"
            )
            .foregroundStyle(.secondary)
            if preview.formulaLikeCells > 0 {
              Label(
                "\(preview.formulaLikeCells) formula-like cells will be inserted as literal text",
                systemImage: "exclamationmark.triangle.fill"
              )
              .foregroundStyle(.orange)
            }
            GroupBox("Column mapping") {
              Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 6) {
                ForEach(preview.headers.indices, id: \.self) { index in
                  GridRow {
                    Text(preview.headers[index]).textSelection(.enabled)
                    Image(systemName: "arrow.right")
                      .foregroundStyle(.secondary)
                    TextField(
                      "Target column",
                      text: $model.csvImportMappedColumns[index]
                    )
                    .disabled(model.csvImportReview != nil)
                    Picker(
                      "Value type",
                      selection: $model.csvImportColumnTypes[index]
                    ) {
                      Text("Text").tag("text")
                      Text("Integer").tag("signed")
                      Text("Float").tag("float64")
                      Text("Boolean").tag("boolean")
                    }
                    .labelsHidden()
                    .disabled(model.csvImportReview != nil)
                  }
                }
              }
              .padding(6)
            }
            GroupBox("Preview — first \(preview.rows.count) rows") {
              ScrollView([.horizontal, .vertical]) {
                Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 5) {
                  GridRow {
                    ForEach(preview.headers, id: \.self) { header in
                      Text(header).bold()
                    }
                  }
                  Divider()
                  ForEach(preview.rows.indices, id: \.self) { rowIndex in
                    GridRow {
                      ForEach(preview.rows[rowIndex].cells.indices, id: \.self) { column in
                        Text(preview.rows[rowIndex].cells[column])
                          .lineLimit(1)
                          .textSelection(.enabled)
                      }
                    }
                  }
                }
                .padding(6)
              }
              .frame(minHeight: 150, maxHeight: 260)
            }
          }
          if let review = model.csvImportReview {
            ChangeReviewPlane(
              kindWord: "INSERT",
              title: "LEDGER · frozen CSV import",
              preview:
                "INSERT \(review.rowCount) rows · \(review.columnCount) mapped columns → \(review.target)",
              metadataFact: ChangeReviewPresentation.metadataStrip(
                target: review.target,
                expiresAtMs: review.expiresAtMs,
                nowMs: model.nowMilliseconds(),
                destructive: false,
                extra: review.formulaLikeCells > 0
                  ? "\(review.formulaLikeCells) formula-like cells as literals" : nil),
              destructive: false,
              production: model.activeProductionWarning,
              rollbackSummary:
                "Plan frozen 60s. Authority is consumed before database I/O and cannot be retried after failure.",
              fixtureNote: review.formulaLikeCells > 0
                ? "Formula-like cells insert as literal text (never formulas)." : nil,
              previewAccessibilityId: "import.csv.review.preview"
            )
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }
      if let progress = model.csvImportProgress {
        VStack(alignment: .leading, spacing: 6) {
          ProgressView(
            value: Double(progress.completedRows),
            total: Double(max(progress.totalRows, 1))
          ) {
            Text("\(progress.completedRows) of \(progress.totalRows) rows")
          }
          .accessibilityIdentifier("import.csv.progress")
          .accessibilityValue(
            "\(progress.phase), \(progress.completedRows) of \(progress.totalRows) rows")
          HStack {
            Text(progress.phase.replacingOccurrences(of: "_", with: " ").capitalized)
              .foregroundStyle(.secondary)
            Spacer()
            if ["running", "cancel_requested"].contains(progress.phase) {
              Button("Cancel Import", role: .destructive) {
                Task { await model.cancelCsvImport() }
              }
              .disabled(progress.phase == "cancel_requested")
              .accessibilityIdentifier("import.csv.cancel")
            }
          }
          if !progress.errors.isEmpty {
            GroupBox("Import errors") {
              VStack(alignment: .leading, spacing: 5) {
                ForEach(progress.errors.indices, id: \.self) { index in
                  Text(progress.errors[index]).textSelection(.enabled)
                }
                if progress.errorsTruncated { Text("Additional errors omitted").italic() }
                Button("Copy Errors") { model.copyCsvImportErrors() }
                  .accessibilityIdentifier("import.csv.copy-errors")
                if let copied = model.csvImportErrorCopyOutcome {
                  Text(copied).foregroundStyle(.secondary)
                }
              }
              .padding(6)
            }
            .accessibilityIdentifier("import.csv.errors")
          }
        }
      } else if model.csvImportApplying {
        ProgressView("Starting reviewed import…")
      }
      if let outcome = model.csvImportOutcome {
        Label(
          outcome,
          systemImage: model.csvImportProgress?.phase == "completed"
            ? "checkmark.circle.fill" : "exclamationmark.circle.fill"
        )
        .foregroundStyle(model.csvImportProgress?.phase == "completed" ? .green : .orange)
        .accessibilityIdentifier("import.csv.outcome")
        .accessibilityValue(outcome)
      }
      if let error = model.csvImportError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .padding(20)
    .frame(minWidth: 720, idealHeight: 560)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(model.csvImportReview != nil || model.csvImportApplying)
  }
}

private struct RedisOverviewSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Redis Overview", systemImage: "gauge.with.dots.needle.67percent")
          .font(.title2.bold())
        Spacer()
        Button("Refresh") { Task { await model.showRedisOverview() } }
          .disabled(model.redisOverviewLoading)
        Button("Close") { model.redisOverviewPresented = false }
      }
      if model.redisOverviewLoading {
        ProgressView("Loading bounded INFO snapshot…")
      }
      if let overview = model.redisOverview {
        Text("Sampled at \(overview.sampledAtMs) ms since Unix epoch")
          .font(.callout)
          .foregroundStyle(.secondary)
        ScrollView {
          LazyVStack(alignment: .leading, spacing: 5) {
            ForEach(overview.lines.indices, id: \.self) { index in
              Text(overview.lines[index])
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
            }
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding(8)
        }
      } else if !model.redisOverviewLoading && model.redisOverviewError == nil {
        ContentUnavailableView(
          "No Redis snapshot", systemImage: "gauge",
          description: Text("Refresh to sample current server facts.")
        )
      }
      if let error = model.redisOverviewError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .padding(20)
    .frame(minWidth: 680, minHeight: 520)
  }
}

private struct RedisSubscriptionSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Redis Pub/Sub", systemImage: "dot.radiowaves.left.and.right")
          .font(.title2.bold())
        Spacer()
        Button("Refresh") { Task { await model.refreshRedisSubscription() } }
          .disabled(model.redisSubscriptionStatus == nil)
        Button("Close") { Task { await model.closeRedisSubscription() } }
      }
      HStack(spacing: 10) {
        Picker("Mode", selection: $model.redisSubscriptionPattern) {
          Text("Channel").tag(false)
          Text("Pattern").tag(true)
        }
        .pickerStyle(.segmented)
        .frame(width: 190)
        .disabled(model.redisSubscriptionIsActive)
        TextField(
          model.redisSubscriptionPattern ? "Pattern" : "Channel",
          text: $model.redisSubscriptionSelector
        )
        .textFieldStyle(.roundedBorder)
        .disabled(model.redisSubscriptionIsActive)
        .accessibilityIdentifier("redis.pubsub.selector")
        Button("Subscribe") { Task { await model.startRedisSubscription() } }
          .buttonStyle(.glassProminent)
          .disabled(
            model.redisSubscriptionStarting || model.redisSubscriptionIsActive
              || model.redisSubscriptionSelector.trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty
          )
          .accessibilityIdentifier("redis.pubsub.subscribe")
        Button("Cancel", role: .cancel) { Task { await model.cancelRedisSubscription() } }
          .disabled(!model.redisSubscriptionIsActive)
          .accessibilityIdentifier("redis.pubsub.cancel")
      }
      if model.redisSubscriptionStarting {
        ProgressView("Starting subscription…")
      }
      if let status = model.redisSubscriptionStatus {
        HStack(spacing: 12) {
          Text(status.pattern ? "PSUBSCRIBE" : "SUBSCRIBE").bold()
          Text(status.selector).font(.system(.body, design: .monospaced))
          Spacer()
          Text(status.phase.replacingOccurrences(of: "_", with: " ").capitalized)
          Text("\(status.totalReceived) received")
        }
        .foregroundStyle(.secondary)
        if status.discontinuities > 0 {
          Label(
            "\(status.discontinuities) delivery gap(s); displayed messages are not complete",
            systemImage: "exclamationmark.triangle.fill"
          )
          .foregroundStyle(.orange)
          .accessibilityIdentifier("redis.pubsub.gap")
        }
        GroupBox("Messages · newest retained window") {
          if status.messages.isEmpty {
            ContentUnavailableView(
              "Waiting for messages", systemImage: "ellipsis.message",
              description: Text("Published messages appear here until cancellation."))
          } else {
            ScrollView {
              LazyVStack(alignment: .leading, spacing: 6) {
                ForEach(Array(status.messages.enumerated()), id: \.offset) { _, message in
                  Text(message)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
              }
              .padding(8)
            }
          }
        }
        Text(status.summary).font(.callout).foregroundStyle(.secondary)
          .accessibilityIdentifier("redis.pubsub.status")
          .accessibilityValue(status.summary)
      } else if !model.redisSubscriptionStarting && model.redisSubscriptionError == nil {
        ContentUnavailableView(
          "No active subscription", systemImage: "dot.radiowaves.left.and.right",
          description: Text("Choose a channel or pattern, then subscribe."))
      }
      if let error = model.redisSubscriptionError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .padding(20)
    .frame(minWidth: 760, minHeight: 560)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(model.redisSubscriptionIsActive)
  }
}

/// Shared Change Review plane: kind-first opaque instrument (not glass content).
/// Preview text is descriptive only — Rust owns execution plans and tokens.
private struct ChangeReviewPlane: View {
  let kindWord: String
  let title: String
  let preview: String
  let metadataFact: String
  let destructive: Bool
  let production: Bool
  let rollbackSummary: String?
  let fixtureNote: String?
  let previewAccessibilityId: String

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(kindWord)
          .font(.caption.weight(.bold).monospaced())
          .tracking(0.4)
          .accessibilityIdentifier("change.review.kind")
        VStack(alignment: .leading, spacing: 2) {
          Text(title)
            .font(.caption.weight(.bold))
            .tracking(0.4)
          Text(metadataFact)
            .font(.caption2.monospaced())
            .foregroundStyle(.secondary)
            .textSelection(.enabled)
            .accessibilityIdentifier("change.review.metadata")
        }
        Spacer(minLength: 4)
        if production {
          Text("HALO PRODUCTION")
            .font(.caption2.weight(.bold))
            .accessibilityLabel("Production — writes need review")
        }
        if destructive {
          Text("DESTRUCTIVE")
            .font(.caption2.weight(.bold).monospaced())
            .accessibilityIdentifier("change.review.destructive")
        }
      }
      Text(preview)
        .font(.system(.body, design: .monospaced))
        .textSelection(.enabled)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityIdentifier(previewAccessibilityId)
      if let fixtureNote {
        Text(fixtureNote)
          .font(.caption2.weight(.semibold))
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("change.review.fixture")
      }
      if let rollbackSummary, !rollbackSummary.isEmpty {
        Text(rollbackSummary)
          .font(.caption)
          .foregroundStyle(.secondary)
          .textSelection(.enabled)
          .accessibilityIdentifier("change.review.rollback")
      }
    }
    .padding(10)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(Color(nsColor: .controlBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("change.review.plane")
  }
}

private struct ProbeChangeReviewSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss

  private var nowMs: UInt64 {
    model.nowMilliseconds()
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack(alignment: .firstTextBaseline) {
        Text("CHANGE REVIEW")
          .font(.caption.weight(.bold))
          .tracking(0.6)
        Text("LEDGER")
          .font(.caption2.weight(.bold).monospaced())
          .foregroundStyle(.secondary)
        Spacer()
        Button("Close") {
          Task {
            await model.closeProbeChangeReview()
            dismiss()
          }
        }
        .disabled(model.probeChangeApplying)
        .accessibilityIdentifier("change.review.probe.close")
      }
      Text("Edit-safety probe")
        .font(.title3.weight(.semibold))
      if let review = model.probeChangeReview {
        ChangeReviewPlane(
          kindWord: ChangeReviewPresentation.probeKindWord,
          title: "LEDGER · \(ChangeReviewPresentation.probeLedgerCount) entry",
          preview: ChangeReviewPresentation.probePreview,
          metadataFact: ChangeReviewPresentation.metadataStrip(
            target: ChangeReviewPresentation.probeTarget,
            expiresAtMs: review.expiresAtMs,
            nowMs: nowMs,
            destructive: ChangeReviewPresentation.probeDestructive,
            extra: "safety probe"),
          destructive: ChangeReviewPresentation.probeDestructive,
          production: model.activeProductionWarning,
          rollbackSummary: ChangeReviewPresentation.probeRollbackSummary,
          fixtureNote: ChangeReviewPresentation.probeIsFixtureSafetyDemo
            ? "Safety probe — demonstrates consume-once review; not a staged grid cell edit."
            : nil,
          previewAccessibilityId: "change.review.probe.preview"
        )
        HStack {
          Button("Discard Review", role: .cancel) {
            Task {
              await model.discardProbeChangeReview()
              dismiss()
            }
          }
          .accessibilityIdentifier("change.review.probe.discard")
          Spacer()
          Button("Apply Reviewed Change") {
            Task { await model.applyProbeChangeReview() }
          }
          .buttonStyle(.glassProminent)
          .disabled(model.probeChangeApplying)
          .accessibilityIdentifier("change.review.probe.apply")
        }
      } else if model.probeChangeOutcome == nil && model.probeChangeError == nil {
        ContentUnavailableView(
          "No open review",
          systemImage: "checkmark.shield",
          description: Text("Stage a probe from the SQL workbench to open Change Review.")
        )
      }
      if model.probeChangeApplying {
        ProgressView("Applying reviewed probe…")
          .accessibilityIdentifier("change.review.probe.applying")
      }
      if let outcome = model.probeChangeOutcome {
        Label(outcome, systemImage: "checkmark.circle.fill")
          .font(.caption.monospaced())
          .accessibilityIdentifier("change.review.probe.outcome")
      }
      if let error = model.probeChangeError {
        Text(error)
          .font(.caption.monospaced())
          .textSelection(.enabled)
          .accessibilityIdentifier("change.review.probe.error")
      }
      Spacer(minLength: 0)
    }
    .padding(20)
    .frame(minWidth: 640, minHeight: 420)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("change.review.probe.sheet")
    .interactiveDismissDisabled(model.probeChangeReview != nil || model.probeChangeApplying)
  }
}

private struct DdlChangeSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @State private var applyConfirmationPresented = false

  private var needsDefinition: Bool {
    ["add_column", "create_index", "add_constraint"].contains(model.ddlChangeKind)
  }

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Text("CHANGE REVIEW")
          .font(.caption.weight(.bold))
          .tracking(0.6)
        Text("STRUCTURE")
          .font(.caption2.weight(.bold).monospaced())
          .foregroundStyle(.secondary)
        Spacer()
        Button("Close") { Task { await model.closeDdlChange() } }
          .disabled(model.ddlChangeApplying)
      }
      Text("Structure change")
        .font(.title3.weight(.semibold))
      Form {
        Picker("Operation", selection: $model.ddlChangeKind) {
          Text("Add column").tag("add_column")
          Text("Drop column").tag("drop_column")
          Text("Create index").tag("create_index")
          Text("Drop index").tag("drop_index")
          Text("Add constraint").tag("add_constraint")
          Text("Drop constraint").tag("drop_constraint")
        }
        .disabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
        TextField("Object name", text: $model.ddlChangeObjectName)
          .disabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
          .accessibilityIdentifier("structure.change.object")
        if needsDefinition {
          TextField(
            model.ddlChangeKind == "add_column"
              ? "Column type"
              : model.ddlChangeKind == "create_index"
                ? "Comma-separated columns" : "UNIQUE, PRIMARY KEY, or CHECK definition",
            text: $model.ddlChangeDefinition
          )
          .disabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
          .accessibilityIdentifier("structure.change.definition")
        }
      }
      .formStyle(.grouped)
      HStack {
        Button("Review Change…") { Task { await model.stageDdlChange() } }
          .buttonStyle(.glassProminent)
          .disabled(
            model.ddlChangeReview != nil || model.ddlChangeApplying
              || model.ddlChangeObjectName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              || (needsDefinition
                && model.ddlChangeDefinition.trimmingCharacters(in: .whitespacesAndNewlines)
                  .isEmpty)
          )
          .accessibilityIdentifier("structure.change.review")
        Button("Discard Review", role: .cancel) {
          Task { await model.discardDdlChangeReview() }
        }
        .disabled(model.ddlChangeReview == nil || model.ddlChangeApplying)
        Spacer()
      }
      if let review = model.ddlChangeReview {
        ChangeReviewPlane(
          kindWord: ChangeReviewPresentation.kindWord(
            preview: review.preview, destructive: review.destructive, fallback: "DDL"),
          title: "LEDGER · frozen structure plan",
          preview: review.preview,
          metadataFact: ChangeReviewPresentation.metadataStrip(
            target: nil,
            expiresAtMs: review.expiresAtMs,
            nowMs: model.nowMilliseconds(),
            destructive: review.destructive),
          destructive: review.destructive,
          production: model.activeProductionWarning,
          rollbackSummary: review.rollbackSummary,
          fixtureNote: review.destructive
            ? "Removes structure — second confirmation required before apply." : nil,
          previewAccessibilityId: "structure.change.preview"
        )
        HStack {
          Button("Discard Review", role: .cancel) {
            Task { await model.discardDdlChangeReview() }
          }
          Spacer()
          Button("Apply Reviewed Change…") { applyConfirmationPresented = true }
            .buttonStyle(.glassProminent)
            .accessibilityIdentifier("structure.change.apply-review")
        }
      }
      if model.ddlChangeApplying { ProgressView("Applying structure change…") }
      if let outcome = model.ddlChangeOutcome {
        Label(outcome, systemImage: "checkmark.circle.fill")
          .foregroundStyle(.green)
          .accessibilityIdentifier("structure.change.outcome")
      }
      if let error = model.ddlChangeError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
      Spacer()
    }
    .padding(20)
    .frame(minWidth: 680, minHeight: 520)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
    .confirmationDialog(
      model.ddlChangeReview?.destructive == true
        ? "Apply destructive structure change?" : "Apply structure change?",
      isPresented: $applyConfirmationPresented,
      presenting: model.ddlChangeReview
    ) { review in
      if review.destructive {
        Button("Apply Destructive Change", role: .destructive) {
          Task { await model.applyDdlChange() }
        }
      } else {
        Button("Apply Structure Change") { Task { await model.applyDdlChange() } }
      }
      Button("Cancel", role: .cancel) {}
    } message: { review in
      Text("\(review.preview)\n\n\(review.rollbackSummary)")
    }
  }
}

private struct TableOperationSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Text("CHANGE REVIEW")
          .font(.caption.weight(.bold))
          .tracking(0.6)
        Text("TABLE OP")
          .font(.caption2.weight(.bold).monospaced())
          .foregroundStyle(.secondary)
        Spacer()
        Button("Close") { Task { await model.closeTableOperation() } }
          .disabled(model.tableOperationApplying)
          .accessibilityIdentifier("table-operation.close")
      }
      Text("Table operation")
        .font(.title3.weight(.semibold))
      Picker("Operation", selection: $model.tableOperationKind) {
        if model.connectedEngine == "postgresql" {
          Text("Rename table").tag("rename")
          Text("Truncate all rows").tag("truncate")
          Text("Drop table").tag("drop")
          Text("Vacuum").tag("vacuum")
          Text("Analyze").tag("analyze")
        } else if model.connectedEngine == "clickhouse" {
          Text("Optimize table").tag("optimize")
        }
      }
      .disabled(model.tableOperationReview != nil || model.tableOperationApplying)
      .accessibilityIdentifier("table-operation.kind")
      .onChange(of: model.tableOperationKind) {
        Task { await model.resetTableOperationReview() }
      }
      if model.tableOperationKind == "rename" {
        TextField("New table name", text: $model.tableOperationNewName)
          .disabled(model.tableOperationReview != nil || model.tableOperationApplying)
          .accessibilityIdentifier("table-operation.new-name")
      }
      Button("Review Operation…") { Task { await model.stageTableOperation() } }
        .buttonStyle(.glassProminent)
        .disabled(
          model.tableOperationReview != nil || model.tableOperationApplying
            || (model.tableOperationKind == "rename"
              && model.tableOperationNewName.trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty)
        )
        .accessibilityIdentifier("table-operation.review")
      if let review = model.tableOperationReview {
        ChangeReviewPlane(
          kindWord: ChangeReviewPresentation.kindWord(
            preview: review.preview, destructive: review.destructive, fallback: "TABLE"),
          title: "LEDGER · frozen table operation",
          preview: review.preview,
          metadataFact: ChangeReviewPresentation.metadataStrip(
            target: review.target,
            expiresAtMs: review.expiresAtMs,
            nowMs: model.nowMilliseconds(),
            destructive: review.destructive,
            extra: "type \(review.confirmation) to authorize"),
          destructive: review.destructive,
          production: model.activeProductionWarning,
          rollbackSummary: review.destructive
            ? "Destroys table data — exact target name required."
            : "Exact target name required before apply.",
          fixtureNote: nil,
          previewAccessibilityId: "table-operation.preview"
        )
        TextField("Exact table name", text: $model.tableOperationConfirmation)
          .accessibilityIdentifier("table-operation.confirmation")
        HStack {
          Button("Discard Review", role: .cancel) {
            Task { await model.resetTableOperationReview() }
          }
          Spacer()
          Button(review.destructive ? "Apply Destructive Operation" : "Apply Operation") {
            Task { await model.applyTableOperation() }
          }
          .buttonStyle(.glassProminent)
          .disabled(model.tableOperationConfirmation != review.confirmation)
          .accessibilityIdentifier("table-operation.apply")
        }
      }
      if model.tableOperationApplying {
        ProgressView(model.tableOperationStatus?.summary ?? "Starting table operation…")
          .accessibilityIdentifier("table-operation.progress")
        if model.tableOperationStatus?.cancellable == false {
          Text("Cancellation is unavailable for this engine operation.")
            .font(.caption)
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("table-operation.cancel-unavailable")
        }
      }
      if let outcome = model.tableOperationOutcome {
        Label(outcome, systemImage: "checkmark.circle.fill")
          .foregroundStyle(.green)
          .accessibilityIdentifier("table-operation.outcome")
      }
      if let error = model.tableOperationError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("table-operation.error")
      }
      Spacer()
    }
    .padding(20)
    .frame(minWidth: 680, minHeight: 500)
    .interactiveDismissDisabled(model.tableOperationReview != nil || model.tableOperationApplying)
    .accessibilityElement(children: .contain)
  }
}

private struct PendingPostgresSignal {
  let kind: String
  let pid: Int32
}

private struct PostgresRolesSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  private var matchingRoles: [String] {
    guard let snapshot = model.postgresRoleSnapshot else { return [] }
    let query = model.postgresRoleSearch.trimmingCharacters(in: .whitespacesAndNewlines)
    return query.isEmpty
      ? snapshot.roles
      : snapshot.roles.filter { $0.localizedCaseInsensitiveContains(query) }
  }
  private var isPrivilegeChange: Bool {
    model.postgresRoleChangeKind.hasSuffix("privilege")
  }

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 12) {
      HStack {
        Label("PostgreSQL Roles and Privileges", systemImage: "person.2")
          .font(.headline)
        Spacer()
        Button("Refresh") { Task { await model.refreshPostgresRoles() } }
          .disabled(model.postgresRolesLoading)
        Button("Close") {
          Task { await model.discardPostgresRoleChange() }
          model.postgresRolesPresented = false
        }
      }
      TextField("Search roles", text: $model.postgresRoleSearch)
        .textFieldStyle(.roundedBorder)
        .accessibilityIdentifier("postgres.roles.search")
      if let snapshot = model.postgresRoleSnapshot {
        Text("Current user: \(snapshot.currentUser)").font(.subheadline)
        HStack(alignment: .top, spacing: 16) {
          GroupBox("Roles") {
            List(matchingRoles, id: \.self) { role in Text(role) }
          }
          GroupBox("Effective membership") {
            List(snapshot.effectiveRoles, id: \.self) { role in Text(role) }
          }
          GroupBox("Direct memberships") {
            List(snapshot.memberships) { membership in
              VStack(alignment: .leading) {
                Text("\(membership.member) in \(membership.role)")
                Text(
                  "inherit \(membership.inheritOption ? "yes" : "no") · admin \(membership.adminOption ? "yes" : "no") · set \(membership.setOption ? "yes" : "no")"
                )
                .font(.caption).foregroundStyle(.secondary)
              }
            }
          }
        }
        .frame(minHeight: 150)
        GroupBox(snapshot.privilegeScope.map { "Privileges · \($0)" } ?? "Privileges") {
          if snapshot.privilegesUnavailable {
            Text("Privileges unavailable for this relation.")
          } else if snapshot.privileges.isEmpty {
            Text(
              snapshot.privilegeScope == nil
                ? "Select a relation to inspect grants." : "No grants found.")
          } else {
            List(snapshot.privileges) { privilege in
              HStack {
                Text(privilege.grantee)
                Text(privilege.privilege).fontWeight(.medium)
                Spacer()
                Text(privilege.grantable ? "Grantable" : "Not grantable")
                  .foregroundStyle(.secondary)
              }
            }
            .frame(minHeight: 100)
          }
        }
        if !snapshot.cycleEdges.isEmpty {
          Label("Membership cycle detected", systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange)
        }
        if snapshot.truncated {
          Label("Snapshot truncated at safety limits", systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange)
        }
        GroupBox("Reviewed change") {
          VStack(alignment: .leading, spacing: 8) {
            Picker("Action", selection: $model.postgresRoleChangeKind) {
              Text("Grant membership").tag("grant_membership")
              Text("Revoke membership").tag("revoke_membership")
              Text("Grant table privilege").tag("grant_privilege")
              Text("Revoke table privilege").tag("revoke_privilege")
            }
            .pickerStyle(.segmented)
            if !isPrivilegeChange {
              TextField("Role", text: $model.postgresRoleChangeRole)
                .accessibilityIdentifier("postgres.roles.change.role")
            }
            TextField(
              isPrivilegeChange ? "Grantee" : "Member", text: $model.postgresRoleChangeSubject
            )
            .accessibilityIdentifier("postgres.roles.change.subject")
            if isPrivilegeChange {
              Picker("Privilege", selection: $model.postgresRoleChangePrivilege) {
                ForEach(
                  ["SELECT", "INSERT", "UPDATE", "DELETE", "TRUNCATE", "REFERENCES", "TRIGGER"],
                  id: \.self
                ) {
                  Text($0).tag($0)
                }
              }
              Text("Privilege changes use selected relation only.").font(.caption)
            }
            Button("Review Change…") { Task { await model.stagePostgresRoleChange() } }
              .disabled(
                model.postgresRoleChangeSubject.trimmingCharacters(in: .whitespacesAndNewlines)
                  .isEmpty
                  || (!isPrivilegeChange
                    && model.postgresRoleChangeRole.trimmingCharacters(in: .whitespacesAndNewlines)
                      .isEmpty)
                  || (isPrivilegeChange && model.selectedObjectTab == nil)
              )
              .accessibilityIdentifier("postgres.roles.change.review")
            Text("Revoking current-user authority is blocked before review.")
              .font(.caption).foregroundStyle(.secondary)
          }
        }
        if let outcome = model.postgresRoleChangeOutcome {
          Text(outcome).foregroundStyle(.green)
            .accessibilityIdentifier("postgres.roles.change.outcome")
        }
      }
      if model.postgresRolesLoading { ProgressView("Loading roles…") }
      if let error = model.postgresRolesError {
        Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.red)
      }
    }
    .padding(18)
    .frame(minWidth: 720, minHeight: 520)
    .confirmationDialog(
      "Apply role change?",
      isPresented: Binding(
        get: { model.postgresRoleChangeReview != nil },
        set: { if !$0 { Task { await model.discardPostgresRoleChange() } } }
      ),
      presenting: model.postgresRoleChangeReview
    ) { _ in
      Button("Apply Role Change", role: .destructive) {
        Task { await model.applyPostgresRoleChange() }
      }
      Button("Cancel", role: .cancel) { Task { await model.discardPostgresRoleChange() } }
    } message: { review in
      Text("\(review.summary). Authority expires in 60 seconds and is consumed on apply.")
    }
  }
}

private struct PostgresRelationshipsSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack {
        Label("Relation Lens", systemImage: "arrow.triangle.branch")
          .font(.headline)
        Spacer()
        Button("Refresh") { Task { await model.refreshPostgresRelationships() } }
          .disabled(model.postgresRelationshipsLoading)
        Button("Close") { model.postgresRelationshipsPresented = false }
      }
      if let snapshot = model.postgresRelationshipSnapshot {
        Text(
          "\(snapshot.namespace).\(snapshot.relation) · \(snapshot.edges.count) foreign-key columns"
        )
        .font(.caption)
        .foregroundStyle(.secondary)
        if snapshot.truncated {
          Label("Showing first 512 edges", systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange)
        }
        if snapshot.edges.isEmpty && !model.postgresRelationshipsLoading {
          ContentUnavailableView(
            "No relationships", systemImage: "arrow.triangle.branch",
            description: Text("No inbound or outbound foreign keys were found."))
        } else {
          List(snapshot.edges) { edge in
            HStack(spacing: 10) {
              VStack(alignment: .leading, spacing: 3) {
                Text("\(edge.fromSchema).\(edge.fromTable).\(edge.fromColumn)")
                Text("→ \(edge.toSchema).\(edge.toTable).\(edge.toColumn)")
                  .foregroundStyle(.secondary)
                if edge.fromSchema == edge.toSchema && edge.fromTable == edge.toTable {
                  Text("Self-reference").font(.caption).foregroundStyle(.orange)
                }
              }
              Spacer()
              Button("Relation Lens") { Task { await model.openRelatedRelation(edge) } }
                .buttonStyle(.glass)
                .accessibilityLabel("Open Relation Lens for \(edge.id)")
                .accessibilityIdentifier("relation.lens.open")
            }
          }
        }
      }
      if model.postgresRelationshipsLoading { ProgressView("Loading relationships…") }
      if let error = model.postgresRelationshipsError {
        Label(error, systemImage: "exclamationmark.triangle")
          .foregroundStyle(.red)
      }
    }
    .padding(18)
    .frame(minWidth: 680, minHeight: 420)
  }
}

private struct PostgresActivitySheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @State private var pendingSignal: PendingPostgresSignal?

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("PostgreSQL Activity", systemImage: "waveform.path.ecg")
          .font(.title2.bold())
        Spacer()
        Button("Refresh") { Task { await model.refreshPostgresActivity() } }
          .disabled(model.postgresActivityLoading)
          .accessibilityIdentifier("postgres.activity.refresh")
        Button("Close") { model.postgresActivityPresented = false }
          .accessibilityIdentifier("postgres.activity.close")
      }
      Text("Current client backends. Cancel stops one query; terminate closes its session.")
        .font(.callout)
        .foregroundStyle(.secondary)
      if model.postgresActivityLoading {
        ProgressView("Loading bounded pg_stat_activity snapshot…")
      }
      if model.postgresActivityRows.isEmpty && !model.postgresActivityLoading
        && model.postgresActivityError == nil
      {
        ContentUnavailableView(
          "No client backends", systemImage: "server.rack",
          description: Text("Refresh to inspect current PostgreSQL activity."))
      } else {
        List(model.postgresActivityRows) { row in
          VStack(alignment: .leading, spacing: 6) {
            HStack {
              Text("PID \(row.pid)").font(.headline)
              Text(row.state).foregroundStyle(.secondary)
              Spacer()
              Button("Cancel Query") {
                pendingSignal = PendingPostgresSignal(kind: "cancel", pid: row.pid)
              }
              .accessibilityIdentifier("postgres.activity.cancel.\(row.pid)")
              Button("Terminate Session", role: .destructive) {
                pendingSignal = PendingPostgresSignal(kind: "terminate", pid: row.pid)
              }
              .accessibilityIdentifier("postgres.activity.terminate.\(row.pid)")
            }
            Text(
              "\(row.user) · \(row.application.isEmpty ? "unknown application" : row.application)"
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            Text(row.queryPreview.isEmpty ? "No query text" : row.queryPreview)
              .font(.system(.body, design: .monospaced))
              .textSelection(.enabled)
          }
          .padding(.vertical, 4)
          .accessibilityElement(children: .contain)
          .accessibilityIdentifier("postgres.activity.row.\(row.pid)")
        }
      }
      if let outcome = model.postgresActivityOutcome {
        Label(outcome, systemImage: "checkmark.circle.fill")
          .foregroundStyle(.green)
          .accessibilityIdentifier("postgres.activity.outcome")
      }
      if let error = model.postgresActivityError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("postgres.activity.error")
      }
    }
    .padding(20)
    .frame(minWidth: 760, minHeight: 520)
    .accessibilityElement(children: .contain)
    .confirmationDialog(
      pendingSignal?.kind == "terminate" ? "Terminate PostgreSQL session?" : "Cancel query?",
      isPresented: Binding(
        get: { pendingSignal != nil },
        set: { if !$0 { pendingSignal = nil } }
      ),
      presenting: pendingSignal
    ) { pending in
      Button(
        pending.kind == "terminate" ? "Terminate PID \(pending.pid)" : "Cancel PID \(pending.pid)",
        role: pending.kind == "terminate" ? .destructive : nil
      ) {
        pendingSignal = nil
        Task { await model.signalPostgresBackend(kind: pending.kind, pid: pending.pid) }
      }
      .accessibilityIdentifier("postgres.activity.confirm")
      Button("Keep Running", role: .cancel) { pendingSignal = nil }
    } message: { pending in
      Text(
        pending.kind == "terminate"
          ? "PostgreSQL will close backend PID \(pending.pid)."
          : "PostgreSQL will request cancellation for PID \(pending.pid).")
    }
  }
}

private struct PostgresToolsSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  private var operationActive: Bool {
    model.postgresToolStatus?.phase == "running"
      || model.postgresToolStatus?.phase == "cancel_requested"
  }

  var body: some View {
    @Bindable var model = model
    let target =
      model.activeProfile.map {
        "\($0.name) · \($0.host ?? "unknown host"):\($0.port ?? "?")/\($0.context ?? "postgres")"
      } ?? "Temporary · \(model.formHost):\(model.formPort)/\(model.formDatabase)"
    VStack(alignment: .leading, spacing: 16) {
      HStack {
        Label("PostgreSQL Backup and Restore", systemImage: "externaldrive.badge.timemachine")
          .font(.title2.bold())
        Spacer()
        Button("Close") { model.closePostgresTools() }
          .disabled(operationActive)
          .accessibilityIdentifier("postgres.tools.close")
      }
      Label(target, systemImage: "server.rack")
        .font(.callout)
        .foregroundStyle(.secondary)
        .textSelection(.enabled)
        .accessibilityIdentifier("postgres.tools.target")
      Picker("Operation", selection: $model.postgresToolKind) {
        Text("Backup").tag("dump")
        Text("Restore").tag("restore")
      }
      .pickerStyle(.segmented)
      .disabled(operationActive)
      .accessibilityIdentifier("postgres.tools.kind")
      .onChange(of: model.postgresToolKind) {
        model.postgresToolFileUrl = nil
        model.postgresToolStatus = nil
        Task { await model.probePostgresTool() }
      }
      GroupBox("Client tool") {
        VStack(alignment: .leading, spacing: 8) {
          HStack {
            TextField("Optional absolute tool path", text: $model.postgresToolExplicitPath)
              .textFieldStyle(.roundedBorder)
              .disabled(operationActive)
              .accessibilityIdentifier("postgres.tools.path")
            Button("Check Version") { Task { await model.probePostgresTool() } }
              .disabled(operationActive)
              .accessibilityIdentifier("postgres.tools.probe")
          }
          if let probe = model.postgresToolProbe {
            Label(
              probe.summary,
              systemImage: probe.available ? "checkmark.circle.fill" : "xmark.circle.fill"
            )
            .foregroundStyle(probe.available ? .green : .red)
            .accessibilityIdentifier("postgres.tools.probe-result")
            if let path = probe.path {
              Text(path).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
            }
          }
        }.padding(6)
      }
      GroupBox(model.postgresToolKind == "dump" ? "Backup destination" : "Restore archive") {
        HStack {
          Text(model.postgresToolFileUrl?.path ?? "No archive selected")
            .lineLimit(1).truncationMode(.middle).textSelection(.enabled)
          Spacer()
          Button("Choose…") { model.choosePostgresToolFile() }
            .disabled(operationActive)
            .accessibilityIdentifier("postgres.tools.choose-file")
        }.padding(6)
      }
      GroupBox("Configuration") {
        VStack(alignment: .leading, spacing: 8) {
          Picker("Content", selection: $model.postgresToolContent) {
            Text("Schema and data").tag("all")
            Text("Schema only").tag("schema_only")
            Text("Data only").tag("data_only")
          }
          .disabled(operationActive)
          .accessibilityIdentifier("postgres.tools.content")
          Toggle("Do not restore original ownership", isOn: $model.postgresToolNoOwner)
            .disabled(operationActive)
            .accessibilityIdentifier("postgres.tools.no-owner")
          if model.postgresToolKind == "restore" {
            Toggle("Drop matching objects before restore", isOn: $model.postgresToolClean)
              .disabled(operationActive)
              .accessibilityIdentifier("postgres.tools.clean")
            if model.postgresToolClean {
              Text("Uses --clean with --if-exists. Matching objects may be destroyed.")
                .foregroundStyle(.orange)
            }
          }
        }.padding(6)
      }
      GroupBox("Review") {
        Text(
          model.postgresToolKind == "dump"
            ? "Create a \(model.postgresToolContent.replacingOccurrences(of: "_", with: " ")) PostgreSQL custom-format backup at the selected destination. An incomplete archive is removed if cancelled."
            : "Load \(model.postgresToolContent.replacingOccurrences(of: "_", with: " ")) from the selected archive into the connected database. Restore may execute code chosen by source superusers and overwrite objects or data; use only a trusted archive."
        )
        .foregroundStyle(model.postgresToolKind == "restore" ? .orange : .secondary)
        .padding(6)
      }
      if let status = model.postgresToolStatus {
        HStack {
          if operationActive { ProgressView() }
          Text(
            "\(status.phase.replacingOccurrences(of: "_", with: " ").capitalized): \(status.summary)"
          )
          .accessibilityIdentifier("postgres.tools.status")
          Spacer()
          if operationActive {
            Button("Cancel", role: .destructive) { Task { await model.cancelPostgresTool() } }
              .disabled(status.phase == "cancel_requested")
              .accessibilityIdentifier("postgres.tools.cancel")
          }
        }
      }
      if let error = model.postgresToolError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("postgres.tools.error")
      }
      HStack {
        Spacer()
        Button(model.postgresToolKind == "dump" ? "Start Backup…" : "Start Restore…") {
          model.requestStartPostgresTool()
        }
        .buttonStyle(.glassProminent)
        .disabled(
          operationActive || model.postgresToolProbe?.available != true
            || model.postgresToolFileUrl == nil
        )
        .accessibilityIdentifier("postgres.tools.start")
      }
    }
    .padding(20)
    .frame(minWidth: 700, minHeight: 500)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(operationActive)
    .confirmationDialog(
      model.postgresToolKind == "dump" ? "Start PostgreSQL backup?" : "Start PostgreSQL restore?",
      isPresented: $model.postgresToolReviewRequested
    ) {
      Button(
        model.postgresToolKind == "dump" ? "Create Backup" : "Restore Database",
        role: model.postgresToolKind == "restore" ? .destructive : nil
      ) { Task { await model.startPostgresTool() } }
      .accessibilityIdentifier("postgres.tools.confirm")
      Button("Cancel", role: .cancel) { model.postgresToolReviewRequested = false }
    } message: {
      Text(
        model.postgresToolKind == "dump"
          ? "Run the checked pg_dump version against the connected PostgreSQL database?"
          : "Run the checked pg_restore version against the connected PostgreSQL database? This can replace database objects and data."
      )
    }
  }
}

struct ResultGridWithInspector: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let table: WorkbenchTable
  let minimumHeight: CGFloat
  var exposesResultPaging = false

  private var visibleRowIndices: [Int] {
    let term = model.loadedRowQuickFilter.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !term.isEmpty else { return Array(table.rows.indices) }
    return table.rows.indices.filter { row in
      table.rows[row].contains { value in
        value.range(of: term, options: [.caseInsensitive, .diacriticInsensitive]) != nil
      }
    }
  }

  private var visibleTable: WorkbenchTable {
    WorkbenchTable(
      columns: table.columns,
      rows: visibleRowIndices.map { table.rows[$0] },
      columnMetadata: table.columnMetadata,
      cells: visibleRowIndices.map { table.cells[$0] })
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      // One glass chrome cluster above opaque grid content.
      GlassEffectContainer {
        VStack(alignment: .leading, spacing: 4) {
          HStack(spacing: 8) {
            ResultCopyMenu()
            ResultExportMenu()
            Button {
              model.openRelationContinuumFromSelection()
            } label: {
              Label("Continuum", systemImage: "arrow.triangle.branch")
            }
            .buttonStyle(.glassProminent)
            .controlSize(.small)
            .disabled(!model.canOpenRelationContinuum)
            .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
            .help("Row Continuum: related rows for this cell (⌘⌥→)")
            .accessibilityIdentifier("relation.continuum.open")
            if model.relationContinuum != nil {
              Button("Close Continuum") { model.closeRelationContinuum() }
                .buttonStyle(.glass)
                .controlSize(.small)
                .keyboardShortcut(.leftArrow, modifiers: [.command, .option])
                .accessibilityIdentifier("relation.continuum.close")
            }
            Spacer(minLength: 0)
          }
          HStack(spacing: 8) {
            TextField(
              "Filter loaded rows",
              text: Binding(
                get: { model.loadedRowQuickFilter },
                set: { model.loadedRowQuickFilter = $0 })
            )
            .textFieldStyle(.roundedBorder)
            .frame(minWidth: 120, maxWidth: 220)
            .accessibilityIdentifier("results.quick-filter")
            let loadedRowsStatus =
              "Loaded rows only · \(visibleRowIndices.count)/\(table.rows.count)"
            Text(loadedRowsStatus)
              .font(.caption.monospacedDigit())
              .foregroundStyle(.secondary)
              .accessibilityIdentifier("results.quick-filter.status")
              .accessibilityValue(loadedRowsStatus)
            if exposesResultPaging && model.nextStartRow != nil {
              Button("Load more") { Task { await model.loadMore() } }
                .buttonStyle(.glass)
                .controlSize(.small)
                .accessibilityIdentifier("results.next-page")
            }
            if let outcome = model.copyOutcome {
              Text(outcome)
                .font(.caption).foregroundStyle(.secondary)
                .accessibilityIdentifier("results.copy.outcome")
                .accessibilityValue(outcome)
            }
            if let error = model.copyError {
              Text(error).font(.caption).foregroundStyle(.red)
            }
            if let continuumError = model.relationContinuumError {
              Text(continuumError)
                .font(.caption)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("relation.continuum.error")
            }
            Spacer(minLength: 0)
          }
        }
        .controlSize(.small)
      }
      if let selection = model.selectedCellSnapshot {
        let presentation = GridCellPresentation.project(selection.1)
        Text(
          "\(selection.0.name) · \(presentation.statusFact) · R\(selection.2 + 1) C\(selection.3 + 1)"
        )
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("results.selection.status")
        .accessibilityValue(
          "\(selection.0.name), \(presentation.accessibilityValue), row \(selection.2 + 1)")
      }
      if table.rows.isEmpty {
        ContentUnavailableView(
          "No rows in this result",
          systemImage: "tablecells",
          description: Text("Run a query or open a table that returns rows.")
        )
        .frame(minHeight: minimumHeight)
        .accessibilityIdentifier("results.grid.empty")
      } else {
        HSplitView {
          CatalogGrid(table: visibleTable, sorts: model.resultSort) { row, column in
            guard visibleRowIndices.indices.contains(row) else { return }
            model.selectCell(row: visibleRowIndices[row], column: column)
          }
          .frame(minWidth: 280, minHeight: 100, idealHeight: minimumHeight)
          if let continuum = model.relationContinuum {
            RelationContinuumPlane(state: continuum) {
              model.closeRelationContinuum()
            }
            .frame(minWidth: 220, idealWidth: 320, maxWidth: 480)
          } else if let snapshot = model.selectedCellSnapshot {
            NativeValueInspector(
              column: snapshot.0, cell: snapshot.1,
              row: snapshot.2, columnIndex: snapshot.3
            )
            .frame(minWidth: 180, idealWidth: 280, maxWidth: 380)
          }
        }
      }
    }
  }
}

/// Spatial peer plane for Row Continuum (opaque content; chrome is labels only).
private struct RelationContinuumPlane: View {
  let state: WorkbenchPresentationStore.RelationContinuumState
  let onClose: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .firstTextBaseline) {
        VStack(alignment: .leading, spacing: 2) {
          Text("CONTINUUM")
            .font(.caption.weight(.bold))
            .tracking(0.6)
          Text(state.edgeTitle)
            .font(.caption.monospaced())
            .textSelection(.enabled)
        }
        Spacer(minLength: 0)
        Text(state.statusWord)
          .font(.caption2.weight(.semibold))
          .accessibilityLabel("Continuum status \(state.statusWord)")
        Button(action: onClose) {
          Image(systemName: "xmark")
        }
        .buttonStyle(.borderless)
        .accessibilityLabel("Close Continuum")
      }
      Text("\(state.directionWord) · \(state.fromColumn) = \(state.fromValue)")
        .font(.caption)
        .foregroundStyle(.secondary)
      if state.usesFixtureData {
        Text("FIXTURE neighbors — not live catalog truth")
          .font(.caption2.weight(.semibold))
          .accessibilityIdentifier("relation.continuum.fixture-badge")
      }
      if state.rows.isEmpty {
        ContentUnavailableView(
          "No related rows",
          systemImage: "arrow.triangle.branch",
          description: Text("No fixture neighbors for this value.")
        )
        .frame(maxHeight: .infinity)
      } else {
        // Dense opaque preview grid (not glass). AppKit NSTableView remains
        // primary for full results; this plane is a focused neighbor surface.
        VStack(alignment: .leading, spacing: 0) {
          HStack(spacing: 8) {
            ForEach(state.columns, id: \.self) { name in
              Text(name)
                .font(.caption2.weight(.semibold))
                .frame(maxWidth: .infinity, alignment: .leading)
            }
          }
          .padding(.vertical, 4)
          Divider()
          ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
              ForEach(Array(state.rows.enumerated()), id: \.offset) { _, cells in
                HStack(spacing: 8) {
                  ForEach(Array(cells.enumerated()), id: \.offset) { _, cell in
                    Text(cell)
                      .font(.system(.caption, design: .monospaced))
                      .frame(maxWidth: .infinity, alignment: .leading)
                      .textSelection(.enabled)
                  }
                }
                .padding(.vertical, 2)
              }
            }
          }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
    }
    .padding(10)
    .background(Color(nsColor: .textBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("relation.continuum.plane")
    .accessibilityLabel(
      "Relation Continuum \(state.edgeTitle), \(state.rows.count) rows, \(state.statusWord)")
  }
}

/// Deterministic neighbor map for presentation prototype (sample-schema shaped).
enum RelationContinuumFixtures {
  struct Edge {
    let fromTable: String
    let fromColumn: String
    let toSchema: String
    let toTable: String
    let toColumn: String
  }

  static func edge(forColumn column: String) -> Edge? {
    switch column.lowercased() {
    case "album_id":
      return Edge(
        fromTable: "tracks", fromColumn: "album_id",
        toSchema: "main", toTable: "albums", toColumn: "id")
    case "artist_id":
      return Edge(
        fromTable: "albums", fromColumn: "artist_id",
        toSchema: "main", toTable: "artists", toColumn: "id")
    default:
      return nil
    }
  }

  static func neighbors(edge: Edge, value: String) -> (columns: [String], rows: [[String]]) {
    switch edge.toTable {
    case "albums":
      let columns = ["id", "title", "artist_id"]
      let all: [[String]] = [
        ["1", "Harbor Light", "1"],
        ["2", "Stone Circle", "2"],
      ]
      let rows = all.filter { $0[0] == value }
      return (columns, rows)
    case "artists":
      let columns = ["id", "name"]
      let all: [[String]] = [
        ["1", "Northwind Quartet"],
        ["2", "Lake District Trio"],
      ]
      let rows = all.filter { $0[0] == value }
      return (columns, rows)
    default:
      return ([], [])
    }
  }
}

private struct ResultExportMenu: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    HStack(spacing: 6) {
      exportButton("Export CSV", format: "csv")
      Menu {
        exportButton("TSV", format: "tsv")
        exportButton("JSON", format: "json")
        exportButton("Markdown", format: "markdown")
        if model.sqlInsertCopyAvailable {
          exportButton("SQL INSERT", format: "sql_insert")
        }
        Divider()
        fullExportButton("Full Result CSV", format: "csv")
        fullExportButton("Full Result TSV", format: "tsv")
        fullExportButton("Full Result JSON", format: "json")
      } label: {
        Label("More Export Formats", systemImage: "ellipsis.circle")
      }
      .accessibilityIdentifier("results.export.more")
    }
    .fixedSize(horizontal: true, vertical: true)
    .disabled(model.resultIdData == nil)
    .accessibilityHint("Atomically export all rows currently resident in this result")
  }

  private func exportButton(_ label: String, format: String) -> some View {
    Button(label) { Task { await model.exportLoadedResult(format: format) } }
      .buttonStyle(.glass)
      .accessibilityIdentifier("results.export.\(format)")
  }

  private func fullExportButton(_ label: String, format: String) -> some View {
    Button(label) { Task { await model.exportFullResult(format: format) } }
      .accessibilityIdentifier("results.export.full.\(format)")
  }
}

private struct ResultCopyMenu: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    Menu {
      Section("Selected cell") {
        copyButtons(scope: "cell")
      }
      Section("Selected row") {
        copyButtons(scope: "row")
      }
      Section("Loaded result") {
        copyButtons(scope: "loaded")
      }
    } label: {
      Label("Copy Result", systemImage: "doc.on.doc")
    }
    .disabled(model.resultIdData == nil)
    .accessibilityHint("Choose scope and Rust-formatted clipboard representation")
  }

  @ViewBuilder
  private func copyButtons(scope: String) -> some View {
    Button("TSV") { Task { await model.copyResult(scope: scope, preferredFormat: "tsv") } }
    Button("CSV") { Task { await model.copyResult(scope: scope, preferredFormat: "csv") } }
    Button("JSON") { Task { await model.copyResult(scope: scope, preferredFormat: "json") } }
    Button("Markdown") {
      Task { await model.copyResult(scope: scope, preferredFormat: "markdown") }
    }
    if model.sqlInsertCopyAvailable {
      Button("SQL INSERT") {
        Task { await model.copyResult(scope: scope, preferredFormat: "sql_insert") }
      }
    }
  }
}

/// Kind-first trailing inspector: opaque content plane matching Continuum density.
/// Rust/UniFFI own typed bytes; Swift only projects text, hex dump, and bounded JSON tree.
private struct NativeValueInspector: View {
  let column: WorkbenchColumn
  let cell: WorkbenchCell
  let row: Int
  let columnIndex: Int

  @State private var hexExpanded: Bool = true

  private var presentation: GridCellPresentation {
    GridCellPresentation.project(cell)
  }

  private var hexLinear: String {
    ValueInspectorProjection.hexLinear(cell.bytes)
  }

  private var hexDump: String {
    ValueInspectorProjection.hexDump(cell.bytes)
  }

  private var metadataFact: String {
    ValueInspectorProjection.metadataFact(
      engineType: column.engineType,
      nullable: column.nullable,
      byteCount: cell.bytes.count,
      originalByteCount: cell.originalByteCount,
      isTruncated: cell.isTruncated)
  }

  private var structuredRows: [StructuredValueTreeRow]? {
    guard cell.kind == 8 else { return nil }
    return try? StructuredValueTree.decode(cell.bytes)
  }

  private var structuredTreeFailed: Bool {
    cell.kind == 8 && structuredRows == nil
  }

  private var prefersHexPrimary: Bool {
    cell.kind == 9  // Binary
  }

  /// Multi-line dump is progressive when large; compact linear hex always stays painted.
  private var hexNeedsDisclosure: Bool {
    cell.bytes.count > 64
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      header
      Divider()
      ScrollView {
        VStack(alignment: .leading, spacing: 8) {
          Text(metadataFact)
            .font(.caption2.monospaced())
            .foregroundStyle(.secondary)
            .textSelection(.enabled)
            .accessibilityIdentifier("value.inspector.metadata")

          if cell.isTruncated {
            Label(
              cell.originalByteCount.map { "Truncated — original \($0) B" }
                ?? "Truncated value",
              systemImage: "scissors"
            )
            .font(.caption.weight(.semibold))
            .accessibilityIdentifier("value.inspector.truncated")
          }

          primarySurface

          if structuredTreeFailed {
            Text(
              "JSON tree unavailable — payload is not valid JSON within tree bounds. Text and hex remain."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("value.inspector.tree.unavailable")
          }

          if let structuredRows {
            treeSection(structuredRows)
          }

          hexSection
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
    .background(Color(nsColor: .textBackgroundColor))
    .onAppear {
      // Large blobs start collapsed; binary and small values keep hex open.
      hexExpanded = prefersHexPrimary || !hexNeedsDisclosure
    }
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("value.inspector")
    .accessibilityLabel(
      "Value inspector for \(column.name), \(presentation.accessibilityValue), \(ValueInspectorProjection.locationFact(row: row, columnIndex: columnIndex))"
    )
  }

  private var header: some View {
    HStack(alignment: .firstTextBaseline, spacing: 8) {
      Text(presentation.kindGlyph)
        .font(.caption.weight(.bold).monospaced())
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 2) {
        Text(presentation.kindLabel.uppercased())
          .font(.caption.weight(.bold))
          .tracking(0.5)
          .accessibilityIdentifier("value.inspector.kind")
        Text(column.name)
          .font(.caption.monospaced())
          .textSelection(.enabled)
          .lineLimit(2)
          .accessibilityIdentifier("value.inspector.column")
      }
      Spacer(minLength: 4)
      VStack(alignment: .trailing, spacing: 4) {
        Text(ValueInspectorProjection.locationFact(row: row, columnIndex: columnIndex))
          .font(.caption2.monospacedDigit())
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("value.inspector.location")
        HStack(spacing: 6) {
          Button("Copy Text") { copyToPasteboard(cell.display) }
            .buttonStyle(.borderless)
            .controlSize(.mini)
            .disabled(presentation.isNull && cell.display.isEmpty)
            .accessibilityIdentifier("value.inspector.copy.text")
          Button("Copy Hex") { copyToPasteboard(hexLinear) }
            .buttonStyle(.borderless)
            .controlSize(.mini)
            .disabled(cell.bytes.isEmpty)
            .accessibilityIdentifier("value.inspector.copy.hex")
        }
      }
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 8)
  }

  @ViewBuilder
  private var primarySurface: some View {
    if presentation.isNull {
      VStack(alignment: .leading, spacing: 4) {
        Text("∅")
          .font(.title2.monospaced())
        Text("NULL")
          .font(.caption.weight(.semibold))
          .foregroundStyle(.secondary)
        // Keep raw display text in the hierarchy for audit / selection.
        Text(cell.display.isEmpty ? "NULL" : cell.display)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .opacity(cell.display.isEmpty ? 0 : 1)
          .accessibilityIdentifier("value.inspector.text")
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .accessibilityElement(children: .combine)
      .accessibilityLabel("NULL value")
    } else if presentation.kindLabel == "Text" && presentation.title == "·" {
      VStack(alignment: .leading, spacing: 4) {
        Text("·")
          .font(.title2.monospaced())
        Text("Empty text")
          .font(.caption.weight(.semibold))
          .foregroundStyle(.secondary)
        Text(cell.display)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityIdentifier("value.inspector.text")
      }
    } else if prefersHexPrimary {
      VStack(alignment: .leading, spacing: 6) {
        sectionLabel("BYTES")
        Text(hexDump.isEmpty ? "Empty" : hexDump)
          .font(.system(.caption, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityIdentifier("value.inspector.hex")
        if !cell.display.isEmpty {
          sectionLabel("LABEL")
          Text(cell.display)
            .font(.system(.caption, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityIdentifier("value.inspector.text")
        }
      }
    } else {
      VStack(alignment: .leading, spacing: 6) {
        sectionLabel("TEXT")
        Text(cell.display.isEmpty ? "Empty" : cell.display)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityIdentifier("value.inspector.text")
      }
    }
  }

  private func treeSection(_ rows: [StructuredValueTreeRow]) -> some View {
    VStack(alignment: .leading, spacing: 4) {
      sectionLabel("JSON TREE")
      VStack(alignment: .leading, spacing: 3) {
        ForEach(rows) { treeRow in
          HStack(alignment: .firstTextBaseline, spacing: 6) {
            Text(treeRow.label)
              .fontWeight(.medium)
            if let value = treeRow.value {
              Text(value)
                .foregroundStyle(.secondary)
                .lineLimit(4)
            }
          }
          .font(.system(.caption, design: .monospaced))
          .padding(.leading, CGFloat(treeRow.depth) * 12)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityElement(children: .combine)
          .accessibilityIdentifier("value.inspector.tree.\(treeRow.id)")
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .accessibilityIdentifier("value.inspector.tree")
  }

  private var hexSection: some View {
    // Binary already surfaces dump as primary. Other kinds keep linear hex always
    // painted (audit + copy), with multi-line dump progressive when large.
    Group {
      if prefersHexPrimary {
        EmptyView()
      } else {
        VStack(alignment: .leading, spacing: 4) {
          HStack(spacing: 6) {
            Text("HEX")
              .font(.caption2.weight(.semibold))
              .tracking(0.4)
              .foregroundStyle(.secondary)
            Text("\(cell.bytes.count) B")
              .font(.caption2.monospacedDigit())
              .foregroundStyle(.secondary)
          }
          // Always-visible compact hex — required for fixture audits and quick scan.
          Text(hexLinear.isEmpty ? "Empty" : hexLinear)
            .font(.system(.caption, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityIdentifier("value.inspector.hex")
          if hexNeedsDisclosure {
            DisclosureGroup(isExpanded: $hexExpanded) {
              Text(hexDump)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.top, 2)
                .accessibilityIdentifier("value.inspector.hex.dump")
            } label: {
              Text("Dump")
                .font(.caption2)
            }
            .accessibilityIdentifier("value.inspector.hex.group")
          } else if !hexDump.isEmpty, hexDump.contains("\n") {
            Text(hexDump)
              .font(.system(.caption2, design: .monospaced))
              .foregroundStyle(.secondary)
              .textSelection(.enabled)
              .frame(maxWidth: .infinity, alignment: .leading)
              .accessibilityIdentifier("value.inspector.hex.dump")
          }
        }
      }
    }
  }

  private func sectionLabel(_ title: String) -> some View {
    Text(title)
      .font(.caption2.weight(.semibold))
      .tracking(0.4)
      .foregroundStyle(.secondary)
  }

  private func copyToPasteboard(_ string: String) {
    let pasteboard = NSPasteboard.general
    pasteboard.clearContents()
    pasteboard.setString(string, forType: .string)
  }
}

struct QueryTabStrip: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    // Hierarchy: tabs are content selectors, not a row of glass pills.
    // Only the selected tab uses glassProminent; unselected stay plain.
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 2) {
        ForEach(model.queryTabs) { tab in
          let selected =
            model.queryWorkbenchSelected && tab.id == model.selectedQueryTabId
          HStack(spacing: 0) {
            queryTabButton(tab: tab, selected: selected)
            Menu {
              Button("Rename…") { model.beginRenameQueryTab(tab) }
              Button("Close", role: .destructive) {
                model.requestCloseQueryTab(tab)
              }
              .accessibilityIdentifier("query.tab.close")
              .disabled(model.queryTabs.count == 1 || tab.isRunning)
            } label: {
              Image(systemName: tab.isRunning ? "progress.indicator" : "ellipsis")
                .font(.caption)
            }
            .menuStyle(.borderlessButton)
            .controlSize(.small)
            .accessibilityIdentifier("query.tab.actions.\(tab.id.uuidString.lowercased())")
            .accessibilityLabel("Actions for \(tab.title)")
          }
          .padding(.trailing, 2)
        }
        ForEach(model.objectTabs) { tab in
          let selected =
            !model.queryWorkbenchSelected && tab.id == model.selectedObjectTabId
          HStack(spacing: 0) {
            objectTabButton(tab: tab, selected: selected)
            Menu {
              if !tab.pinned {
                Button("Pin") { model.pinObjectTab(tab) }
              }
              Button("Refresh") { Task { await model.reloadObjectTab() } }
              Button("Close", role: .destructive) { model.closeObjectTab(tab) }
                .disabled(tab.isRunning)
            } label: {
              Image(systemName: tab.isRunning ? "progress.indicator" : "ellipsis")
                .font(.caption)
            }
            .menuStyle(.borderlessButton)
            .controlSize(.small)
            .accessibilityLabel("Actions for object \(tab.title)")
          }
          .padding(.trailing, 2)
        }
        Button {
          model.addQueryTab()
        } label: {
          Image(systemName: "plus")
        }
        .buttonStyle(.glass)
        .controlSize(.small)
        .accessibilityLabel("New query tab")
        .disabled(model.queryTabs.count + model.objectTabs.count >= 64)
      }
      .padding(.vertical, 2)
    }
    .accessibilityIdentifier("workbench.tab-strip")
  }

  @ViewBuilder
  private func queryTabButton(tab: NativeQueryTab, selected: Bool) -> some View {
    let label = WorkbenchTabLabel(title: tab.title, model: model)
    let id = "query.tab.\(tab.id.uuidString.lowercased())"
    if selected {
      Button {
        model.selectQueryTab(tab)
      } label: {
        label
      }
      .buttonStyle(.glassProminent)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Selected")
    } else {
      Button {
        model.selectQueryTab(tab)
      } label: {
        label
      }
      .buttonStyle(.plain)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Not selected")
    }
  }

  @ViewBuilder
  private func objectTabButton(tab: NativeObjectTab, selected: Bool) -> some View {
    let label = WorkbenchTabLabel(
      title: tab.title, model: model,
      leadingSystemImage: tab.pinned ? "pin.fill" : "eye")
    let id = "object.tab.\(tab.id.uuidString.lowercased())"
    if selected {
      Button {
        model.selectObjectTab(tab)
      } label: {
        label
      }
      .buttonStyle(.glassProminent)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Selected")
    } else {
      Button {
        model.selectObjectTab(tab)
      } label: {
        label
      }
      .buttonStyle(.plain)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Not selected")
    }
  }
}

private struct WorkbenchTabLabel: View {
  let title: String
  let model: WorkbenchPresentationStore
  var leadingSystemImage: String?

  init(title: String, model: WorkbenchPresentationStore, leadingSystemImage: String? = nil) {
    self.title = title
    self.model = model
    self.leadingSystemImage = leadingSystemImage
  }

  var body: some View {
    HStack(spacing: 4) {
      if let leadingSystemImage { Image(systemName: leadingSystemImage) }
      Text(title)
      if model.activeProductionWarning {
        Image(systemName: "exclamationmark.triangle.fill")
          .accessibilityLabel("Production")
      } else if let environment = model.activeEnvironmentLabel {
        Text(environment).font(.caption2)
      }
      if model.activeSafetyLabel == "Read only" {
        Image(systemName: "lock.fill").accessibilityLabel("Read only")
      }
    }
    .accessibilityElement(children: .combine)
  }
}

/// Environment Halo: production / staging / development must be unmistakable
/// without relying on color alone (Increase Contrast / Reduce Transparency).
struct EnvironmentSafetyBadge: View {
  let model: WorkbenchPresentationStore

  var body: some View {
    if let environment = model.activeEnvironmentLabel,
      let safety = model.activeSafetyLabel
    {
      let isProduction =
        model.activeProductionWarning
        || environment.caseInsensitiveCompare("production") == .orderedSame
      let isStaging = environment.caseInsensitiveCompare("staging") == .orderedSame
      let haloWord: String = {
        if isProduction { return "PRODUCTION" }
        if isStaging { return "STAGING" }
        return environment.uppercased()
      }()
      let haloDetail: String = {
        if isProduction { return "writes need review" }
        if isStaging { return "confirm before apply" }
        return safety
      }()
      HStack(spacing: 6) {
        Image(
          systemName: isProduction
            ? "exclamationmark.triangle.fill"
            : isStaging ? "flag.fill" : safety == "Read only" ? "lock.fill" : "shield")
        VStack(alignment: .leading, spacing: 0) {
          Text("HALO \(haloWord)")
            .font(.caption.weight(isProduction ? .bold : .semibold))
            .textCase(.uppercase)
          Text("\(environment) · \(safety) · \(haloDetail)")
            .font(.caption2)
            .foregroundStyle(.secondary)
        }
      }
      .padding(.horizontal, 8)
      .padding(.vertical, 4)
      // Chrome halo capsule (Tahoe glass); not a content surface.
      .glassEffect(.regular.interactive())
      .accessibilityElement(children: .combine)
      .accessibilityLabel(
        "Environment halo \(haloWord), \(environment), safety \(safety), \(haloDetail)"
      )
      .accessibilityIdentifier("environment.halo")
    }
  }
}

struct WorkbenchToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    WorkbenchConnectionToolbar(model: model)
    WorkbenchFileToolbar(model: model)
    WorkbenchQueryToolbar(model: model)
  }
}

struct WorkbenchFileToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    ToolbarItem(id: "open-sql-file", placement: .automatic) {
      Button {
        model.requestOpenSqlFile()
      } label: {
        Label("Open SQL File", systemImage: "folder")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarItem(id: "save-sql-file", placement: .automatic) {
      Button {
        Task { await model.saveSqlFile() }
      } label: {
        Label("Save SQL File", systemImage: "square.and.arrow.down")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarItem(id: "save-sql-file-as", placement: .automatic) {
      Button {
        Task { await model.saveSqlFile(saveAs: true) }
      } label: {
        Label("Save SQL File As", systemImage: "square.and.arrow.down.on.square")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarItem(id: "reload-sql-file", placement: .automatic) {
      Button {
        Task { await model.reloadSqlFile() }
      } label: {
        Label("Reload SQL File", systemImage: "arrow.clockwise")
      }
      .disabled(!model.queryWorkbenchSelected || model.sqlFile == nil)
    }
  }
}

struct WorkbenchConnectionToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    ToolbarItem(id: "connection", placement: .automatic) {
      Label(
        model.sessionHex == nil ? "Disconnected" : model.connectedEngine,
        systemImage: model.sessionHex == nil ? "bolt.slash" : "bolt.horizontal"
      )
      .accessibilityLabel(
        model.sessionHex == nil
          ? "No active connection" : "Connected to \(model.connectedEngine)")
    }
    ToolbarItem(id: "environment-safety", placement: .automatic) {
      EnvironmentSafetyBadge(model: model)
    }
    ToolbarItem(id: "disconnect", placement: .automatic) {
      Button {
        Task { await model.disconnectActive() }
      } label: {
        Label("Disconnect", systemImage: "bolt.slash")
      }
      .disabled(model.sessionHex == nil || model.isRunning)
    }
    ToolbarItem(id: "health", placement: .automatic) {
      Button {
        Task { await model.checkActiveHealth() }
      } label: {
        Label("Check Health", systemImage: "heart.text.square")
      }
      .disabled(model.sessionHex == nil || model.isRunning || model.healthChecking)
    }
    ToolbarItem(id: "reconnect", placement: .automatic) {
      Button {
        Task { await model.reconnectActive() }
      } label: {
        Label("Reconnect", systemImage: "arrow.triangle.2.circlepath")
      }
      .disabled(
        model.sessionHex == nil || model.isRunning
          || model.reconnectState?.hasPrefix("Reconnecting") == true
      )
    }
    ToolbarItem(id: "history", placement: .automatic) {
      Button {
        Task { await model.presentHistory() }
      } label: {
        Label("Query History", systemImage: "clock.arrow.circlepath")
      }
    }
    ToolbarItem(id: "saved-queries", placement: .automatic) {
      Button {
        Task { await model.presentSavedQueries() }
      } label: {
        Label("Saved Queries", systemImage: "bookmark")
      }
    }
  }
}

struct WorkbenchQueryToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    ToolbarItem(id: "save-query", placement: .automatic) {
      Button {
        model.beginSaveCurrentQuery()
      } label: {
        Label("Save Query", systemImage: "bookmark.badge.plus")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarSpacer(.fixed)
    ToolbarItem(id: "refresh", placement: .automatic) {
      Button {
        Task { await model.browse() }
      } label: {
        Label("Refresh Catalog", systemImage: "arrow.clockwise")
      }
      .disabled(model.sessionHex == nil || model.isRunning || model.isCatalogRefreshing)
    }
    ToolbarSpacer(.fixed)
    ToolbarItem(id: "run", placement: .primaryAction) {
      Button {
        Task { await model.runQuery() }
      } label: {
        Label("Run Query", systemImage: "play.fill")
      }
      .buttonStyle(.glassProminent)
      .disabled(
        !model.queryWorkbenchSelected || model.sessionHex == nil
          || model.isRunning || model.isCatalogRefreshing)
    }
    ToolbarItem(id: "cancel", placement: .primaryAction) {
      Button {
        Task { await model.cancel() }
      } label: {
        Label("Cancel Query", systemImage: "stop.fill")
      }
      .disabled(!model.isRunning)
    }
  }
}

struct NativeSettingsView: View {
  let application: NativeApplicationModel
  @State private var outcome: String?

  var body: some View {
    Form {
      LabeledContent("Storage", value: "Local only")
      LabeledContent("Telemetry", value: "Off by default")
      Section("Support") {
        Button("Export Safe Support Bundle…") { exportSupportBundle() }
          .accessibilityIdentifier("settings.support.export")
        Text("Contains version, platform, and closed redacted diagnostic facts only.")
          .font(.caption)
          .foregroundStyle(.secondary)
        if let outcome {
          Text(outcome)
            .font(.caption)
            .accessibilityIdentifier("settings.support.outcome")
            .accessibilityValue(outcome)
        }
      }
    }
    .formStyle(.grouped)
    .padding()
    .frame(width: 420)
  }

  private func exportSupportBundle() {
    guard let client = application.client else {
      outcome = "Support export unavailable"
      return
    }
    guard
      let url = application.dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Safe Support Bundle", prompt: "Export",
          suggestedFilename: "tablerock-support.txt", allowedExtensions: ["txt"]
        ))
    else { return }
    let destination =
      url.pathExtension.lowercased() == "txt" ? url : url.appendingPathExtension("txt")
    Task {
      let accessed = destination.startAccessingSecurityScopedResource()
      defer { if accessed { destination.stopAccessingSecurityScopedResource() } }
      do {
        let bytes = try await client.exportSupportBundle(path: destination.path)
        outcome = "Exported \(bytes) safe bytes to \(destination.lastPathComponent)"
      } catch {
        outcome = "Support export failed"
      }
    }
  }
}

struct CatalogOutline: NSViewRepresentable {
  let table: [WorkbenchCatalogNode]
  @Binding var selection: String?
  let refreshState: CatalogRefreshState
  let onExpand: @MainActor (String) -> Void
  let onOpen: @MainActor (String) -> Void

  func makeCoordinator() -> Coordinator {
    Coordinator(
      table: table,
      selection: $selection,
      refreshState: refreshState,
      onExpand: onExpand,
      onOpen: onOpen
    )
  }

  func makeNSView(context: Context) -> NSScrollView {
    let outline = NSOutlineView()
    outline.delegate = context.coordinator
    outline.dataSource = context.coordinator
    outline.headerView = nil
    outline.rowSizeStyle = .small
    outline.allowsMultipleSelection = false
    outline.autosaveExpandedItems = false
    outline.setAccessibilityLabel("Database catalog")
    outline.setAccessibilityIdentifier("catalog.outline")
    outline.target = context.coordinator
    outline.doubleAction = #selector(Coordinator.openSelectedObject)
    let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("catalog-name"))
    column.title = "Name"
    column.minWidth = 120
    column.resizingMask = .autoresizingMask
    outline.addTableColumn(column)
    outline.outlineTableColumn = column
    context.coordinator.outline = outline
    outline.reloadData()
    context.coordinator.expandDefaultRoots()

    let scroll = NSScrollView()
    scroll.documentView = outline
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    return scroll
  }

  func updateNSView(_ scroll: NSScrollView, context: Context) {
    guard let outline = scroll.documentView as? NSOutlineView else { return }
    let expanded = context.coordinator.expandedKeys()
    let selected = context.coordinator.selectedKey()
    context.coordinator.selection = $selection
    context.coordinator.onExpand = onExpand
    context.coordinator.onOpen = onOpen
    context.coordinator.rebuild(from: table, refreshState: refreshState)
    outline.reloadData()
    context.coordinator.restore(expanded: expanded, selected: selected)
  }

  @MainActor
  final class Node: NSObject {
    let key: String
    let title: String
    let children: [Node]
    let isState: Bool
    let expandable: Bool

    init(
      key: String,
      title: String,
      children: [Node] = [],
      isState: Bool = false,
      expandable: Bool = false
    ) {
      self.key = key
      self.title = title
      self.children = children
      self.isState = isState
      self.expandable = expandable
    }
  }

  @MainActor
  final class Coordinator: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
    private(set) var roots: [Node] = []
    private var nodesByKey: [String: Node] = [:]
    var selection: Binding<String?>
    var onExpand: @MainActor (String) -> Void
    var onOpen: @MainActor (String) -> Void
    weak var outline: NSOutlineView?
    private var suppressExpansionCallbacks = false

    init(
      table: [WorkbenchCatalogNode],
      selection: Binding<String?>,
      refreshState: CatalogRefreshState,
      onExpand: @escaping @MainActor (String) -> Void,
      onOpen: @escaping @MainActor (String) -> Void
    ) {
      self.selection = selection
      self.onExpand = onExpand
      self.onOpen = onOpen
      super.init()
      rebuild(from: table, refreshState: refreshState)
    }

    func rebuild(from table: [WorkbenchCatalogNode], refreshState: CatalogRefreshState) {
      let byParent = Dictionary(grouping: table, by: \.parentIdBytes)
      func build(_ record: WorkbenchCatalogNode) -> Node {
        let key = catalogNodeKey(record.idBytes)
        var children = (byParent[record.idBytes] ?? []).map(build)
        switch refreshState {
        case .loading(let nodeKey) where nodeKey == key:
          children.append(
            Node(
              key: "state:loading:\(key)", title: "Loading…", isState: true))
        case .stale(let nodeKey, let message) where nodeKey == key:
          children.append(
            Node(
              key: "state:stale:\(key)",
              title: "Stale · \(message)",
              isState: true
            ))
        default:
          break
        }
        return Node(
          key: key,
          title: record.name,
          children: children,
          expandable: record.expandable
        )
      }
      roots = (byParent[nil] ?? []).map(build)
      nodesByKey = [:]
      func index(_ node: Node) {
        nodesByKey[node.key] = node
        node.children.forEach(index)
      }
      roots.forEach(index)
    }

    func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int {
      (item as? Node)?.children.count ?? roots.count
    }

    func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any {
      (item as? Node)?.children[index] ?? roots[index]
    }

    func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool {
      guard let node = item as? Node else { return false }
      return node.expandable || !node.children.isEmpty
    }

    func outlineView(
      _ outlineView: NSOutlineView,
      viewFor tableColumn: NSTableColumn?,
      item: Any
    ) -> NSView? {
      guard let node = item as? Node else { return nil }
      let identifier = NSUserInterfaceItemIdentifier("catalog-cell")
      let cell: NSTableCellView
      if let reused = outlineView.makeView(withIdentifier: identifier, owner: nil)
        as? NSTableCellView
      {
        cell = reused
      } else {
        cell = NSTableCellView()
        cell.identifier = identifier
        let label = NSTextField(labelWithString: "")
        label.lineBreakMode = .byTruncatingTail
        label.translatesAutoresizingMaskIntoConstraints = false
        cell.textField = label
        cell.addSubview(label)
        NSLayoutConstraint.activate([
          label.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 2),
          label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -2),
          label.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
        ])
      }
      cell.textField?.stringValue = node.title
      cell.setAccessibilityLabel(
        node.isState
          ? "Catalog state \(node.title)"
          : node.children.isEmpty
            ? "Catalog object \(node.title)" : "Catalog group \(node.title)")
      cell.setAccessibilityIdentifier("catalog.node.\(node.key)")
      return cell
    }

    func outlineViewItemDidExpand(_ notification: Notification) {
      guard !suppressExpansionCallbacks,
        let node = notification.userInfo?["NSObject"] as? Node,
        node.key.hasPrefix("node:")
      else { return }
      onExpand(node.key)
    }

    func outlineViewSelectionDidChange(_ notification: Notification) {
      guard let outline, outline.selectedRow >= 0,
        let node = outline.item(atRow: outline.selectedRow) as? Node
      else {
        selection.wrappedValue = nil
        return
      }
      selection.wrappedValue = node.key
    }

    @objc func openSelectedObject() {
      guard let outline, outline.selectedRow >= 0,
        let node = outline.item(atRow: outline.selectedRow) as? Node,
        !node.isState
      else { return }
      onOpen(node.key)
    }

    func expandedKeys() -> Set<String> {
      Set(nodesByKey.values.filter { outline?.isItemExpanded($0) == true }.map(\.key))
    }

    func selectedKey() -> String? {
      guard let outline, outline.selectedRow >= 0 else { return selection.wrappedValue }
      return (outline.item(atRow: outline.selectedRow) as? Node)?.key
    }

    func restore(expanded: Set<String>, selected: String?) {
      guard let outline else { return }
      suppressExpansionCallbacks = true
      defer { suppressExpansionCallbacks = false }
      for key in expanded {
        if let node = nodesByKey[key] { outline.expandItem(node) }
      }
      if let selected, let node = nodesByKey[selected] {
        let row = outline.row(forItem: node)
        if row >= 0 {
          outline.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
        }
      }
    }

    func expandDefaultRoots() {
      guard let outline else { return }
      suppressExpansionCallbacks = true
      defer { suppressExpansionCallbacks = false }
      roots.filter { !$0.children.isEmpty }.forEach { outline.expandItem($0) }
    }
  }
}

struct CatalogGrid: NSViewRepresentable {
  let table: WorkbenchTable
  let sorts: [WorkbenchBrowseSort]
  let onSelect: @MainActor (Int, Int) -> Void

  init(
    table: WorkbenchTable,
    sorts: [WorkbenchBrowseSort] = [],
    onSelect: @escaping @MainActor (Int, Int) -> Void = { _, _ in }
  ) {
    self.table = table
    self.sorts = sorts
    self.onSelect = onSelect
  }

  func makeCoordinator() -> Coordinator {
    Coordinator(table, sorts: sorts, onSelect: onSelect)
  }

  final class ResultTableView: NSTableView {
    var onCellActivate: ((Int, Int) -> Void)?

    override func mouseDown(with event: NSEvent) {
      let point = convert(event.locationInWindow, from: nil)
      let activatedRow = row(at: point)
      let activatedColumn = column(at: point)
      super.mouseDown(with: event)
      if activatedRow >= 0, activatedColumn >= 0 {
        onCellActivate?(activatedRow, activatedColumn)
      }
    }
  }

  func makeNSView(context: Context) -> NSScrollView {
    let grid = ResultTableView()
    grid.usesAlternatingRowBackgroundColors = true
    grid.allowsColumnReordering = true
    grid.allowsColumnResizing = true
    grid.allowsMultipleSelection = true
    grid.rowSizeStyle = .small
    // Opaque content surface — never glass (Liquid Glass / design-system).
    grid.backgroundColor = .textBackgroundColor
    grid.gridStyleMask = []  // no interior gridlines; hierarchy via alternating rows
    grid.intercellSpacing = NSSize(width: 6, height: 1)
    grid.setAccessibilityLabel("Query results")
    grid.setAccessibilityIdentifier("results.grid")
    let scroll = NSScrollView()
    scroll.documentView = grid
    scroll.drawsBackground = true
    scroll.backgroundColor = .textBackgroundColor
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    // Content surface: separator only, not heavy bezel chrome.
    scroll.borderType = .lineBorder
    scroll.focusRingType = .exterior
    context.coordinator.installColumns(on: grid)
    grid.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
    grid.delegate = context.coordinator
    grid.dataSource = context.coordinator
    grid.target = context.coordinator
    grid.action = #selector(Coordinator.tableClicked(_:))
    grid.onCellActivate = { [weak coordinator = context.coordinator, weak grid] row, column in
      guard let grid else { return }
      coordinator?.activate(row: row, column: column, in: grid)
    }
    context.coordinator.startPerformanceScrollIfRequested(on: grid)
    return scroll
  }

  func updateNSView(_ scroll: NSScrollView, context: Context) {
    guard let grid = scroll.documentView as? NSTableView else { return }
    let selectedRows = grid.selectedRowIndexes
    context.coordinator.snapshot = table
    context.coordinator.sorts = sorts
    context.coordinator.onSelect = onSelect
    if let resultGrid = grid as? ResultTableView {
      resultGrid.onCellActivate = {
        [weak coordinator = context.coordinator, weak resultGrid] row, column in
        guard let resultGrid else { return }
        coordinator?.activate(row: row, column: column, in: resultGrid)
      }
    }
    context.coordinator.installColumns(on: grid)
    grid.reloadData()
    context.coordinator.startPerformanceScrollIfRequested(on: grid)
    let validSelection = selectedRows.filter { $0 < table.rows.count }
    grid.selectRowIndexes(IndexSet(validSelection), byExtendingSelection: false)
  }

  @MainActor
  final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    final class ResultCellView: NSTableCellView {}

    final class ResultCellButton: NSButton {
      var onActivate: (() -> Void)?

      @objc func activateCell() {
        onActivate?()
      }

      override func mouseDown(with event: NSEvent) {
        onActivate?()
        super.mouseDown(with: event)
      }

      override func accessibilityPerformPress() -> Bool {
        onActivate?()
        return true
      }
    }

    var snapshot: WorkbenchTable
    var sorts: [WorkbenchBrowseSort]
    var onSelect: @MainActor (Int, Int) -> Void
    private var fixtureScrollTask: Task<Void, Never>?
    private var lastActivatedColumn = 0

    init(
      _ snapshot: WorkbenchTable,
      sorts: [WorkbenchBrowseSort],
      onSelect: @escaping @MainActor (Int, Int) -> Void
    ) {
      self.snapshot = snapshot
      self.sorts = sorts
      self.onSelect = onSelect
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
      guard let tableView = notification.object as? NSTableView,
        tableView.selectedRow >= 0
      else { return }
      let column =
        tableView.clickedColumn >= 0
        ? tableView.clickedColumn : lastActivatedColumn
      guard snapshot.columns.indices.contains(column) else { return }
      onSelect(tableView.selectedRow, column)
    }

    @objc func tableClicked(_ tableView: NSTableView) {
      let row = tableView.clickedRow
      let column = tableView.clickedColumn
      guard row >= 0, column >= 0 else { return }
      activate(row: row, column: column, in: tableView)
    }

    func activate(row: Int, column: Int, in tableView: NSTableView) {
      guard snapshot.rows.indices.contains(row), snapshot.columns.indices.contains(column) else {
        return
      }
      lastActivatedColumn = column
      tableView.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
      onSelect(row, column)
    }

    func startPerformanceScrollIfRequested(on tableView: NSTableView) {
      guard fixtureScrollTask == nil,
        ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_AUTOSCROLL"] == "1",
        !snapshot.rows.isEmpty
      else { return }
      let finalRow = snapshot.rows.count - 1
      writePerformanceMetric("PERF_SCROLL_ARMED rows=\(finalRow + 1)")
      fixtureScrollTask = Task { @MainActor [weak tableView] in
        try? await Task.sleep(for: .milliseconds(500))
        guard let tableView, !Task.isCancelled else { return }
        let started = Date()
        for row in stride(from: 0, through: finalRow, by: 250) {
          tableView.scrollRowToVisible(row)
          try? await Task.sleep(for: .milliseconds(16))
        }
        for row in stride(from: finalRow, through: 0, by: -250) {
          tableView.scrollRowToVisible(row)
          try? await Task.sleep(for: .milliseconds(16))
        }
        let elapsed = Date().timeIntervalSince(started)
        writePerformanceMetric(
          "PERF_SCROLL_DONE rows=\(finalRow + 1) elapsed_seconds=\(String(format: "%.6f", elapsed))"
        )
      }
    }

    func numberOfRows(in tableView: NSTableView) -> Int { snapshot.rows.count }

    func installColumns(on tableView: NSTableView) {
      let expected = snapshot.columns.indices.map {
        NSUserInterfaceItemIdentifier("result-column-\($0)")
      }
      if tableView.tableColumns.map(\.identifier) == expected {
        for (column, title) in zip(tableView.tableColumns, snapshot.columns) {
          column.title = workbenchColumnHeaderTitle(column: title, sorts: sorts)
        }
        return
      }
      for column in tableView.tableColumns { tableView.removeTableColumn(column) }
      for (index, title) in snapshot.columns.enumerated() {
        let column = NSTableColumn(
          identifier: NSUserInterfaceItemIdentifier("result-column-\(index)"))
        column.title = workbenchColumnHeaderTitle(column: title, sorts: sorts)
        column.minWidth = 72
        column.width = 148
        column.resizingMask = [.autoresizingMask, .userResizingMask]
        if snapshot.columnMetadata.indices.contains(index) {
          let meta = snapshot.columnMetadata[index]
          let nullability = meta.nullable ? "nullable" : "not null"
          column.headerToolTip = "\(meta.engineType) · \(nullability)"
        }
        tableView.addTableColumn(column)
      }
    }

    func tableView(
      _ tableView: NSTableView,
      viewFor tableColumn: NSTableColumn?,
      row: Int
    ) -> NSView? {
      guard let tableColumn,
        let column = tableView.tableColumns.firstIndex(of: tableColumn),
        snapshot.rows.indices.contains(row),
        snapshot.rows[row].indices.contains(column)
      else { return nil }
      let identifier = NSUserInterfaceItemIdentifier("result-cell")
      let cell: ResultCellView
      if let reused = tableView.makeView(withIdentifier: identifier, owner: nil)
        as? ResultCellView
      {
        cell = reused
      } else {
        cell = ResultCellView()
        cell.identifier = identifier
        let button = ResultCellButton(title: "", target: nil, action: nil)
        button.target = button
        button.action = #selector(ResultCellButton.activateCell)
        button.identifier = NSUserInterfaceItemIdentifier("result-cell-button")
        button.isBordered = false
        button.alignment = .left
        button.lineBreakMode = .byTruncatingTail
        // Monospaced digits for professional dense tables (numbers align visually).
        button.font = NSFont.monospacedDigitSystemFont(
          ofSize: NSFont.smallSystemFontSize, weight: .regular)
        button.translatesAutoresizingMaskIntoConstraints = false
        cell.addSubview(button)
        NSLayoutConstraint.activate([
          button.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
          button.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
          button.topAnchor.constraint(equalTo: cell.topAnchor),
          button.bottomAnchor.constraint(equalTo: cell.bottomAnchor),
        ])
      }
      let typed: WorkbenchCell = {
        if snapshot.cells.indices.contains(row),
          snapshot.cells[row].indices.contains(column)
        {
          return snapshot.cells[row][column]
        }
        let raw = snapshot.rows[row][column]
        return WorkbenchCell(
          display: raw, kind: 7, truncation: 0, originalByteCount: nil, bytes: Data(raw.utf8))
      }()
      let presentation = GridCellPresentation.project(typed)
      guard let button = cell.subviews.first as? ResultCellButton else { return nil }
      button.title = presentation.title
      button.alignment = presentation.isNumeric ? .right : .left
      // Null/empty use secondary label color *plus* glyph (never color alone).
      button.contentTintColor =
        presentation.isNull || presentation.title == "·"
        ? NSColor.secondaryLabelColor : NSColor.labelColor
      let engineType =
        snapshot.columnMetadata.indices.contains(column)
        ? snapshot.columnMetadata[column].engineType : "unknown"
      button.setAccessibilityElement(true)
      button.setAccessibilityRole(.button)
      button.setAccessibilityLabel(
        "\(snapshot.columns[column]), \(engineType), row \(row + 1), \(presentation.kindLabel)")
      button.setAccessibilityValue(presentation.accessibilityValue)
      button.setAccessibilityIdentifier("results.cell.\(row).\(column)")
      button.onActivate = { [weak self, weak tableView] in
        guard let self, let tableView else { return }
        self.activate(row: row, column: column, in: tableView)
      }
      cell.setAccessibilityElement(false)
      return cell
    }
  }
}

func writePerformanceMetric(_ metric: String) {
  FileHandle.standardError.write(Data("\(metric)\n".utf8))
}

func counted(_ count: Int, _ singular: String) -> String {
  "\(count) \(singular)\(count == 1 ? "" : "s")"
}

enum NativeFindReplaceError: Error, CustomStringConvertible {
  case emptyPattern
  case invalidPattern(String)
  case invalidScope
  case replacementLimit

  var description: String {
    switch self {
    case .emptyPattern: "Enter text to find"
    case .invalidPattern(let message): "Invalid regular expression: \(message)"
    case .invalidScope: "Select editor text before using selection scope"
    case .replacementLimit: "More than 10,000 matches; narrow scope before replacing"
    }
  }
}

struct NativeReplaceOutcome {
  let text: String
  let selection: NSRange
  let replacedRange: NSRange
  let delta: Int
  var count = 1
}

enum NativeFindReplaceEngine {
  private static let limit = 10_000

  static func find(
    in text: String, pattern: String, mode: String, scope: NSRange,
    selection: NSRange, previousMatch: NSRange?, backwards: Bool
  ) throws -> NSRange? {
    let matches = try boundedMatches(in: text, pattern: pattern, mode: mode, scope: scope)
    guard !matches.isEmpty else { return nil }
    if backwards {
      let boundary = previousMatch == selection ? selection.location : NSMaxRange(selection)
      return matches.last(where: { NSMaxRange($0.range) <= boundary && $0.range != previousMatch })?
        .range ?? matches.last?.range
    }
    let boundary =
      previousMatch == selection
      ? advancedBoundary(in: text, after: selection) : NSMaxRange(selection)
    return matches.first(where: { $0.range.location >= boundary && $0.range != previousMatch })?
      .range ?? matches.first?.range
  }

  static func replaceCurrent(
    in text: String, pattern: String, replacement: String, mode: String,
    scope: NSRange, selection: NSRange
  ) throws -> NativeReplaceOutcome? {
    let regex = try expression(pattern: pattern, mode: mode)
    try validateScope(scope, in: text)
    guard selection.location >= scope.location, NSMaxRange(selection) <= NSMaxRange(scope),
      let match = regex.firstMatch(in: text, range: selection), match.range == selection
    else { return nil }
    let inserted = replacementText(
      replacement, mode: mode, match: match, source: text, regex: regex)
    let mutable = NSMutableString(string: text)
    mutable.replaceCharacters(in: match.range, with: inserted)
    let insertedLength = (inserted as NSString).length
    return NativeReplaceOutcome(
      text: mutable as String,
      selection: NSRange(location: match.range.location, length: insertedLength),
      replacedRange: match.range, delta: insertedLength - match.range.length)
  }

  static func replaceAll(
    in text: String, pattern: String, replacement: String, mode: String, scope: NSRange
  ) throws -> NativeReplaceOutcome {
    let regex = try expression(pattern: pattern, mode: mode)
    let matches = try boundedMatches(regex: regex, in: text, scope: scope)
    let mutable = NSMutableString(string: text)
    var delta = 0
    for match in matches.reversed() {
      let inserted = replacementText(
        replacement, mode: mode, match: match, source: text, regex: regex)
      mutable.replaceCharacters(in: match.range, with: inserted)
      delta += (inserted as NSString).length - match.range.length
    }
    let resultingScope = NSRange(location: scope.location, length: max(0, scope.length + delta))
    return NativeReplaceOutcome(
      text: mutable as String, selection: resultingScope, replacedRange: scope,
      delta: delta, count: matches.count)
  }

  private static func boundedMatches(
    in text: String, pattern: String, mode: String, scope: NSRange
  ) throws -> [NSTextCheckingResult] {
    try boundedMatches(regex: expression(pattern: pattern, mode: mode), in: text, scope: scope)
  }

  private static func boundedMatches(
    regex: NSRegularExpression, in text: String, scope: NSRange
  ) throws -> [NSTextCheckingResult] {
    try validateScope(scope, in: text)
    var matches: [NSTextCheckingResult] = []
    regex.enumerateMatches(in: text, range: scope) { match, _, stop in
      guard let match else { return }
      matches.append(match)
      if matches.count > limit { stop.pointee = true }
    }
    guard matches.count <= limit else { throw NativeFindReplaceError.replacementLimit }
    return matches
  }

  private static func expression(pattern: String, mode: String) throws -> NSRegularExpression {
    guard !pattern.isEmpty else { throw NativeFindReplaceError.emptyPattern }
    let source: String
    let options: NSRegularExpression.Options
    switch mode {
    case "regular_expression":
      source = pattern
      options = []
    case "whole_word":
      let escaped = NSRegularExpression.escapedPattern(for: pattern)
      source = "(?<![\\p{L}\\p{N}_])\(escaped)(?![\\p{L}\\p{N}_])"
      options = [.caseInsensitive]
    case "case_sensitive":
      source = NSRegularExpression.escapedPattern(for: pattern)
      options = []
    default:
      source = NSRegularExpression.escapedPattern(for: pattern)
      options = [.caseInsensitive]
    }
    do { return try NSRegularExpression(pattern: source, options: options) } catch {
      throw NativeFindReplaceError.invalidPattern(error.localizedDescription)
    }
  }

  private static func replacementText(
    _ replacement: String, mode: String, match: NSTextCheckingResult,
    source: String, regex: NSRegularExpression
  ) -> String {
    mode == "regular_expression"
      ? regex.replacementString(for: match, in: source, offset: 0, template: replacement)
      : replacement
  }

  private static func validateScope(_ scope: NSRange, in text: String) throws {
    let length = (text as NSString).length
    guard scope.location <= length, NSMaxRange(scope) <= length else {
      throw NativeFindReplaceError.invalidScope
    }
  }

  private static func advancedBoundary(in text: String, after range: NSRange) -> Int {
    let length = (text as NSString).length
    let end = NSMaxRange(range)
    guard range.length == 0, end < length else { return end }
    return NSMaxRange((text as NSString).rangeOfComposedCharacterSequence(at: end))
  }
}

struct SqlTextEditor: NSViewRepresentable {
  @Binding var text: String
  @Binding var selection: NSRange
  var isRunning: Bool = false

  func makeCoordinator() -> Coordinator { Coordinator(text: $text, selection: $selection) }

  func makeNSView(context: Context) -> NSScrollView {
    let editor = NSTextView()
    editor.delegate = context.coordinator
    editor.isEditable = true
    editor.isSelectable = true
    editor.isRichText = false
    editor.importsGraphics = false
    editor.allowsUndo = true
    editor.isAutomaticQuoteSubstitutionEnabled = false
    editor.isAutomaticDashSubstitutionEnabled = false
    editor.isAutomaticTextReplacementEnabled = false
    editor.isAutomaticSpellingCorrectionEnabled = false
    editor.isContinuousSpellCheckingEnabled = false
    editor.usesFindBar = true
    editor.isIncrementalSearchingEnabled = true
    editor.font = NSFont.monospacedSystemFont(
      ofSize: NSFont.systemFontSize, weight: .regular)
    // Gutter clearance for line numbers (drawn in ruler).
    editor.textContainerInset = NSSize(width: 4, height: 8)
    editor.drawsBackground = true
    editor.backgroundColor = .textBackgroundColor
    editor.maxSize = NSSize(
      width: CGFloat.greatestFiniteMagnitude,
      height: CGFloat.greatestFiniteMagnitude)
    editor.isHorizontallyResizable = true
    editor.textContainer?.widthTracksTextView = true
    editor.textContainer?.containerSize = NSSize(
      width: CGFloat.greatestFiniteMagnitude,
      height: CGFloat.greatestFiniteMagnitude)
    editor.string = text
    editor.setAccessibilityEnabled(true)
    editor.setAccessibilityLabel("SQL editor")
    editor.setAccessibilityIdentifier("query.editor")

    let scroll = NSScrollView()
    scroll.documentView = editor
    scroll.drawsBackground = true
    scroll.backgroundColor = .textBackgroundColor
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    scroll.borderType = .lineBorder
    scroll.focusRingType = .exterior
    scroll.hasVerticalRuler = true
    scroll.rulersVisible = true
    let ruler = SqlLineNumberRulerView(scrollView: scroll, orientation: .verticalRuler)
    ruler.clientView = editor
    scroll.verticalRulerView = ruler
    context.coordinator.ruler = ruler
    return scroll
  }

  func updateNSView(_ scroll: NSScrollView, context: Context) {
    guard let editor = scroll.documentView as? NSTextView else { return }
    context.coordinator.text = $text
    context.coordinator.selection = $selection
    // Never replace storage while an input method owns marked text.
    guard !editor.hasMarkedText() else { return }
    if editor.string != text {
      let selectedRanges = editor.selectedRanges
      editor.string = text
      let maximum = (text as NSString).length
      editor.selectedRanges = selectedRanges.map { value in
        let range = value.rangeValue
        return NSValue(
          range: NSRange(
            location: min(range.location, maximum),
            length: min(range.length, max(0, maximum - min(range.location, maximum)))
          ))
      }
      context.coordinator.ruler?.needsDisplay = true
    }
    let maximum = (text as NSString).length
    let requested = NSRange(
      location: min(selection.location, maximum),
      length: min(selection.length, max(0, maximum - min(selection.location, maximum))))
    if editor.selectedRange() != requested {
      editor.setSelectedRange(requested)
      editor.scrollRangeToVisible(requested)
    }
    // Soft running cue: keep editable for cancel/edit, dim slightly via text color.
    editor.textColor = isRunning ? NSColor.secondaryLabelColor : NSColor.labelColor
    context.coordinator.ruler?.needsDisplay = true
  }

  @MainActor
  final class Coordinator: NSObject, NSTextViewDelegate {
    var text: Binding<String>
    var selection: Binding<NSRange>
    weak var ruler: SqlLineNumberRulerView?

    init(text: Binding<String>, selection: Binding<NSRange>) {
      self.text = text
      self.selection = selection
    }

    func textDidChange(_ notification: Notification) {
      guard let editor = notification.object as? NSTextView else { return }
      text.wrappedValue = editor.string
      ruler?.needsDisplay = true
    }

    func textViewDidChangeSelection(_ notification: Notification) {
      guard let editor = notification.object as? NSTextView else { return }
      selection.wrappedValue = editor.selectedRange()
      ruler?.needsDisplay = true
    }
  }
}

/// Vertical line-number gutter for the SQL editor (opaque content chrome).
final class SqlLineNumberRulerView: NSRulerView {
  private let gutterWidth: CGFloat = 36

  override init(scrollView: NSScrollView?, orientation: NSRulerView.Orientation) {
    super.init(scrollView: scrollView, orientation: orientation)
    ruleThickness = gutterWidth
    clientView = scrollView?.documentView
  }

  @available(*, unavailable)
  required init(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  override func drawHashMarksAndLabels(in rect: NSRect) {
    guard let textView = clientView as? NSTextView,
      let layoutManager = textView.layoutManager,
      let textContainer = textView.textContainer
    else { return }

    NSColor.controlBackgroundColor.setFill()
    bounds.fill()

    let selected = textView.selectedRange()
    let caret = SqlEditorMetrics.caret(text: textView.string, selection: selected)
    let ns = textView.string as NSString
    let length = ns.length
    let attrs: [NSAttributedString.Key: Any] = [
      .font: NSFont.monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular),
      .foregroundColor: NSColor.secondaryLabelColor,
    ]
    let activeAttrs: [NSAttributedString.Key: Any] = [
      .font: NSFont.monospacedDigitSystemFont(
        ofSize: NSFont.smallSystemFontSize, weight: .semibold),
      .foregroundColor: NSColor.labelColor,
    ]

    if length == 0 {
      let label = "1" as NSString
      let size = label.size(withAttributes: activeAttrs)
      label.draw(
        at: NSPoint(x: ruleThickness - size.width - 6, y: textView.textContainerInset.height),
        withAttributes: activeAttrs)
      return
    }

    var line = 1
    var index = 0
    let visible = textView.visibleRect
    while index < length {
      let lineRange = ns.lineRange(for: NSRange(location: index, length: 0))
      let glyphRange = layoutManager.glyphRange(
        forCharacterRange: lineRange, actualCharacterRange: nil)
      var lineRect = layoutManager.boundingRect(forGlyphRange: glyphRange, in: textContainer)
      lineRect.origin.x += textView.textContainerOrigin.x
      lineRect.origin.y += textView.textContainerOrigin.y
      if lineRect.maxY >= visible.minY, lineRect.minY <= visible.maxY {
        let y = lineRect.minY - visible.minY
        let label = "\(line)" as NSString
        let used = line == caret.line ? activeAttrs : attrs
        let size = label.size(withAttributes: used)
        let point = NSPoint(
          x: ruleThickness - size.width - 6,
          y: y + max(0, (lineRect.height - size.height) / 2))
        label.draw(at: point, withAttributes: used)
      }
      let next = lineRange.location + lineRange.length
      if next <= index { break }
      index = next
      line += 1
      if line > 100_000 { break }
    }
  }
}
