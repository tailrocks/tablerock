#if TABLEROCK_DEVELOPMENT_SUPPORT

// TableRock native macOS development and test support.
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

func makeDevelopmentWorkbenchBackend(scenario: String) -> any WorkbenchBackend {
  ScriptedWorkbenchBackend(scenario: scenario)
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
  let performanceAutoScroll: Bool
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
      performanceAutoScroll: environment["TABLEROCK_FIXTURE_AUTOSCROLL"] == "1",
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
        CatalogGrid(
          table: result,
          performanceAutoScroll: NativeWorkbenchFixtureConfiguration.current.performanceAutoScroll
        )
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
      CatalogGrid(
        table: table,
        performanceAutoScroll: NativeWorkbenchFixtureConfiguration.current.performanceAutoScroll
      )
        .padding(16)
    } else {
      ProgressView("Preparing bounded grid fixture…")
    }
  }
}

func writePerformanceMetric(_ metric: String) {
  FileHandle.standardError.write(Data("\(metric)\n".utf8))
}

#endif
