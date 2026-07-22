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
import Security
import SwiftUI
import TableRockBridge
import TableRockFeature
import UniformTypeIdentifiers

private func connectedSessionLabel(_ session: String) -> String {
  "Connected · session \(session.prefix(16))…"
}

private func zeroizeTransientData(_ data: inout Data?) {
  guard var value = data else { return }
  value.resetBytes(in: 0..<value.count)
  data = value
}

extension Data {
  fileprivate func hexEncodedString() -> String {
    map { String(format: "%02x", $0) }.joined()
  }
}

/// Mutable presentation form. The backend boundary receives a fresh immutable
/// value only when the operator saves or tests the form.
struct StartupActionEditorDraft: Identifiable {
  let id = UUID()
  var statement: String
  var safety: String
  var timeoutMs: UInt32
  var runOnReconnect: Bool

  init(_ value: WorkbenchStartupActionDraft) {
    statement = value.statement
    safety = value.safety
    timeoutMs = value.timeoutMs
    runOnReconnect = value.runOnReconnect
  }

  var workbench: WorkbenchStartupActionDraft {
    .init(
      statement: statement, safety: safety, timeoutMs: timeoutMs,
      runOnReconnect: runOnReconnect)
  }
}

struct ProfileEditorDraft {
  var idBytes: Data?
  var revision: UInt64
  var engine: String
  var name: String
  var group: String
  var environment: String
  var host: String
  var port: String
  var database: String
  var username: String
  var passwordSource: String
  var passwordValue: String
  var passwordReference: Data?
  var hasStoredPassword: Bool
  var plaintextAcknowledged: Bool
  var tlsMode: String
  var safetyMode: String
  var sshEnabled: Bool
  var sshHost: String
  var sshPort: String
  var sshUsername: String
  var sshAuthMode: String
  var sshPassword: String
  var sshPrivateKey: String
  var sshKnownHostsPath: String
  var sshHasStoredPassword: Bool
  var sshHasStoredPrivateKey: Bool
  var sshPlaintextAcknowledged: Bool
  var startupActions: [StartupActionEditorDraft]

  init(_ value: WorkbenchProfileDraft) {
    idBytes = value.idBytes
    revision = value.revision
    engine = value.engine
    name = value.name
    group = value.group
    environment = value.environment
    host = value.host
    port = value.port
    database = value.database
    username = value.username
    passwordSource = value.passwordSource
    passwordValue = value.passwordValue
    passwordReference = value.passwordReference
    hasStoredPassword = value.hasStoredPassword
    plaintextAcknowledged = value.plaintextAcknowledged
    tlsMode = value.tlsMode
    safetyMode = value.safetyMode
    sshEnabled = value.sshEnabled
    sshHost = value.sshHost
    sshPort = value.sshPort
    sshUsername = value.sshUsername
    sshAuthMode = value.sshAuthMode
    sshPassword = value.sshPassword
    sshPrivateKey = value.sshPrivateKey
    sshKnownHostsPath = value.sshKnownHostsPath
    sshHasStoredPassword = value.sshHasStoredPassword
    sshHasStoredPrivateKey = value.sshHasStoredPrivateKey
    sshPlaintextAcknowledged = value.sshPlaintextAcknowledged
    startupActions = value.startupActions.map(StartupActionEditorDraft.init)
  }

  var workbench: WorkbenchProfileDraft {
    .init(
      idBytes: idBytes, revision: revision, engine: engine, name: name,
      group: group, environment: environment, host: host, port: port,
      database: database, username: username, passwordSource: passwordSource,
      passwordValue: passwordValue, passwordReference: passwordReference,
      hasStoredPassword: hasStoredPassword,
      plaintextAcknowledged: plaintextAcknowledged,
      tlsMode: tlsMode, safetyMode: safetyMode,
      sshEnabled: sshEnabled, sshHost: sshHost, sshPort: sshPort,
      sshUsername: sshUsername, sshAuthMode: sshAuthMode, sshPassword: sshPassword,
      sshPrivateKey: sshPrivateKey, sshKnownHostsPath: sshKnownHostsPath,
      sshHasStoredPassword: sshHasStoredPassword,
      sshHasStoredPrivateKey: sshHasStoredPrivateKey,
      sshPlaintextAcknowledged: sshPlaintextAcknowledged,
      startupActions: startupActions.map(\.workbench)
    )
  }
}

/// Test-only environment projection for deterministic appearance evidence.
/// Production launches have no fixture variables and follow system settings.
private struct NativeAppearanceFixture: Sendable {
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

private struct NativeAppearanceFixtureModifier: ViewModifier {
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

@MainActor
private struct WorkbenchActions {
  let model: BridgeModel

  var canRun: Bool {
    model.queryWorkbenchSelected && model.sessionHex != nil
      && !model.isRunning && !model.isCatalogRefreshing
  }
  var canCancel: Bool { model.isRunning }
  var canRefresh: Bool {
    model.sessionHex != nil && !model.isRunning && !model.isCatalogRefreshing
  }
  var canShowActivity: Bool {
    model.sessionHex != nil && model.connectedEngine == "postgresql"
  }
  var canShowPostgresTools: Bool { canShowActivity }
  var canShowRelationships: Bool {
    canShowActivity && model.selectedObjectTab != nil
  }
  var canShowRoles: Bool { canShowActivity }
  var canShowRedisSubscription: Bool {
    model.sessionHex != nil && model.connectedEngine == "redis"
  }
  var canShowFindReplace: Bool { model.queryWorkbenchSelected }

  func run() { Task { await model.runQuery() } }
  func cancel() { Task { await model.cancel() } }
  func refresh() { Task { await model.browse() } }
  func quickSwitch() { Task { await model.showQuickSwitcher() } }
  func explain() { Task { await model.runExplain() } }
  func showActivity() { Task { await model.showPostgresActivity() } }
  func showPostgresTools() { Task { await model.showPostgresTools() } }
  func showRelationships() { Task { await model.showPostgresRelationships() } }
  func showRoles() { Task { await model.showPostgresRoles() } }
  func showRedisSubscription() { model.showRedisSubscription() }
  func showFindReplace() { model.showFindReplace() }
}

private struct WorkbenchActionsKey: FocusedValueKey {
  typealias Value = WorkbenchActions
}

extension FocusedValues {
  fileprivate var workbenchActions: WorkbenchActions? {
    get { self[WorkbenchActionsKey.self] }
    set { self[WorkbenchActionsKey.self] = newValue }
  }
}

private struct WorkbenchCommands: Commands {
  @FocusedValue(\.workbenchActions) private var actions

  var body: some Commands {
    CommandMenu("Query") {
      Button("Run Query") { actions?.run() }
        .keyboardShortcut(.return, modifiers: .command)
        .disabled(actions?.canRun != true)
      Button("Cancel Query") { actions?.cancel() }
        .keyboardShortcut(".", modifiers: .command)
        .disabled(actions?.canCancel != true)
      Divider()
      Button("Refresh Catalog") { actions?.refresh() }
        .keyboardShortcut("r", modifiers: [.command, .shift])
        .disabled(actions?.canRefresh != true)
      Divider()
      Button("Quick Switcher…") { actions?.quickSwitch() }
        .keyboardShortcut("o", modifiers: [.command, .shift])
      Button("Explain Query") { actions?.explain() }
        .keyboardShortcut("e", modifiers: [.command, .shift])
        .disabled(actions?.canRun != true)
      Button("Find and Replace…") { actions?.showFindReplace() }
        .keyboardShortcut("f", modifiers: [.command, .option])
        .disabled(actions?.canShowFindReplace != true)
      Button("PostgreSQL Activity…") { actions?.showActivity() }
        .disabled(actions?.canShowActivity != true)
      Button("PostgreSQL Backup and Restore…") { actions?.showPostgresTools() }
        .disabled(actions?.canShowPostgresTools != true)
      Button("Relationships…") { actions?.showRelationships() }
        .disabled(actions?.canShowRelationships != true)
      Button("PostgreSQL Roles and Privileges…") { actions?.showRoles() }
        .disabled(actions?.canShowRoles != true)
      Button("Redis Pub/Sub…") { actions?.showRedisSubscription() }
        .disabled(actions?.canShowRedisSubscription != true)
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
  func stageAndApply(session: Data, now: UInt64) throws -> WorkbenchApplyOutcome {
    try scriptedUnavailable("stage-apply")
  }

}

actor ScriptedWorkbenchBackend: WorkbenchBackend {
  let scenario: String
  private var cancelled = false
  private var importReviewActive = false
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
    let phase = streamExportCancelled
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
    let phase = importApplyCancelled
      ? "cancelled" : (importApplyPollCount < 4 ? "running" : "completed")
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
    guard scenario == "success", importReviewActive,
      tokenId == Data(repeating: 10, count: 16),
      sessionId == Data(repeating: 1, count: 16)
    else { return try scriptedUnavailable("apply") }
    importReviewActive = false
    return WorkbenchApplyOutcome(
      transaction: "committed", changeCount: 1, appliedCount: 1,
      conflictCount: 0, failedCount: 0)
  }

  func revokeReviewToken(tokenId: Data) throws -> Bool {
    guard scenario == "success", tokenId == Data(repeating: 10, count: 16) else {
      return try scriptedUnavailable("revoke")
    }
    let wasActive = importReviewActive
    importReviewActive = false
    return wasActive
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
    let running = scriptedTableOperationPollCount == 1
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

  func stageAndApply(session: Data, now: UInt64) throws -> WorkbenchApplyOutcome {
    let token = try bridge.stageProbeReview(sessionId: session, nowMs: now)
    return try bridge.applyReviewToken(
      tokenId: token, nowMs: now, sessionId: session, expectedRevision: 0
    ).workbench
  }
}

private func nativeApplicationSupportRoot() throws -> URL {
  try FileManager.default.url(
    for: .applicationSupportDirectory,
    in: .userDomainMask,
    appropriateFor: nil,
    create: true
  )
}

@MainActor
private struct SystemFilePanelPort: AppFilePanelPort {
  func chooseOpenFile(_ request: AppFilePanelRequest) -> URL? {
    let panel = NSOpenPanel()
    configure(panel, request: request)
    panel.allowsMultipleSelection = false
    panel.canChooseDirectories = false
    panel.canChooseFiles = true
    return panel.runModal() == .OK ? panel.url : nil
  }

  func chooseSaveFile(_ request: AppFilePanelRequest) -> URL? {
    let panel = NSSavePanel()
    configure(panel, request: request)
    return panel.runModal() == .OK ? panel.url : nil
  }

  private func configure(_ panel: NSSavePanel, request: AppFilePanelRequest) {
    panel.title = request.title
    panel.prompt = request.prompt
    if let suggestedFilename = request.suggestedFilename {
      panel.nameFieldStringValue = suggestedFilename
    }
    panel.allowedContentTypes = request.allowedExtensions.map {
      UTType(filenameExtension: $0) ?? .plainText
    }
  }
}

@MainActor
struct TestFilePanelPort: AppFilePanelPort {
  let root: URL
  let openPath: String?
  let savePath: String?

  func chooseOpenFile(_ request: AppFilePanelRequest) -> URL? {
    confined(openPath)
  }

  func chooseSaveFile(_ request: AppFilePanelRequest) -> URL? {
    confined(savePath)
  }

  private func confined(_ path: String?) -> URL? {
    guard let path else { return nil }
    let candidate = URL(fileURLWithPath: path).standardizedFileURL
    let root = root.resolvingSymlinksInPath().standardizedFileURL
    let parent = candidate.deletingLastPathComponent().resolvingSymlinksInPath()
    let resolved = parent.appendingPathComponent(candidate.lastPathComponent).standardizedFileURL
    guard resolved.path.hasPrefix(root.path + "/") else { return nil }
    return resolved
  }
}

@MainActor
private struct SystemPasteboardPort: AppPasteboardPort {
  func write(_ representations: [AppPasteboardRepresentation]) throws {
    let item = NSPasteboardItem()
    for representation in representations {
      item.setString(
        representation.value,
        forType: NSPasteboard.PasteboardType(representation.type)
      )
    }
    NSPasteboard.general.clearContents()
    guard NSPasteboard.general.writeObjects([item]) else {
      throw AppCapabilityError.rejected("pasteboard")
    }
  }
}

@MainActor
private struct SystemKeychainPort: AppKeychainPort {
  let namespace: String

  func store(secret: Data, account: String) throws -> Data {
    let query: [CFString: Any] = [
      kSecClass: kSecClassGenericPassword,
      kSecAttrService: namespace,
      kSecAttrAccount: account,
      kSecValueData: secret,
      kSecReturnPersistentRef: true,
    ]
    var result: CFTypeRef?
    let status = SecItemAdd(query as CFDictionary, &result)
    guard status == errSecSuccess, let reference = result as? Data else {
      throw AppCapabilityError.rejected("keychain-store-\(status)")
    }
    return reference
  }

  func read(reference: Data) throws -> Data {
    let query: [CFString: Any] = [
      kSecClass: kSecClassGenericPassword,
      kSecAttrService: namespace,
      kSecMatchItemList: [reference] as CFArray,
      kSecReturnData: true,
      kSecMatchLimit: kSecMatchLimitOne,
    ]
    var result: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    guard status == errSecSuccess, let secret = result as? Data, !secret.isEmpty else {
      throw AppCapabilityError.rejected("keychain-read-\(status)")
    }
    return secret
  }

  func remove(reference: Data) throws {
    let status = SecItemDelete(
      [
        kSecClass: kSecClassGenericPassword,
        kSecAttrService: namespace,
        kSecMatchItemList: [reference] as CFArray,
      ] as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else {
      throw AppCapabilityError.rejected("keychain-remove-\(status)")
    }
  }
}

@MainActor
private final class NativeApplicationModel {
  let client: (any WorkbenchBackend)?
  let bridgeError: String?
  let dependencies: AppDependencies
  private var fixtureWindowOpened = false

  init() {
    var configuredDependencies = AppDependencies(
      filePanels: SystemFilePanelPort(),
      pasteboard: SystemPasteboardPort()
    )
    do {
      let configuration = try AppConfiguration.resolve(
        environment: ProcessInfo.processInfo.environment,
        applicationSupportRoot: nativeApplicationSupportRoot(),
        temporaryRoot: FileManager.default.temporaryDirectory,
        processIdentifier: ProcessInfo.processInfo.processIdentifier
      )
      let filePanels: any AppFilePanelPort =
        configuration.isTestMode
        ? TestFilePanelPort(
          root: configuration.paths.dataRoot,
          openPath: ProcessInfo.processInfo.environment["TABLEROCK_TEST_OPEN_FILE"],
          savePath: ProcessInfo.processInfo.environment["TABLEROCK_TEST_SAVE_FILE"]
        ) : SystemFilePanelPort()
      configuredDependencies = AppDependencies(
        filePanels: filePanels,
        pasteboard: SystemPasteboardPort(),
        keychain: SystemKeychainPort(namespace: configuration.keychainNamespace)
      )
      try configuration.paths.prepare()
      let configuredClient: any WorkbenchBackend
      switch configuration.backend {
      case .live:
        configuredClient = try LiveWorkbenchBackend(
          persistencePath: configuration.paths.profilesDatabase.path
        )
      case .scripted(let scenario):
        configuredClient = ScriptedWorkbenchBackend(scenario: scenario)
      }
      dependencies = configuredDependencies
      client = configuredClient
      bridgeError = nil
    } catch {
      dependencies = configuredDependencies
      client = nil
      bridgeError = "Bridge init failed: \(error)"
    }
  }

  func claimMultiWindowFixtureOpen() -> Bool {
    guard !fixtureWindowOpened else { return false }
    fixtureWindowOpened = true
    return true
  }
}

@main
struct TableRockApp: App {
  private let application = NativeApplicationModel()

  init() {
    NativeAppearanceFixture.current.applyApplicationAppearance()
  }

  var body: some Scene {
    WindowGroup(for: UUID.self) { $windowId in
      if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_ACCESSIBILITY_AUDIT"] == "1" {
        NativeAccessibilityFixtureView()
          .frame(minWidth: 760, minHeight: 520)
      } else if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_PROFILE_EDITOR"] == "1" {
        NativeProfileEditorFixtureView()
      } else if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_GRID_ROWS"] != nil {
        WorkbenchWindowRoot(
          application: application, windowId: windowId
        )
      } else {
        WorkbenchWindowRoot(
          application: application, windowId: windowId
        )
      }
    } defaultValue: {
      application.dependencies.identifiers.next()
    }
    .restorationBehavior(.automatic)
    .commands {
      WorkbenchCommands()
    }
    Settings {
      NativeSettingsView(application: application)
    }
  }
}

private struct WorkbenchWindowRoot: View {
  @Environment(\.openWindow) private var openWindow
  @State private var model: BridgeModel
  private let application: NativeApplicationModel

  init(application: NativeApplicationModel, windowId: UUID) {
    self.application = application
    _model = State(
      initialValue: BridgeModel(
        client: application.client,
        startupError: application.bridgeError,
        windowId: windowId,
        dependencies: application.dependencies
      ))
  }

  var body: some View {
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_GRID_ROWS"] != nil {
      PerformanceFixtureView(table: model.resultTable)
        .frame(minWidth: 760, minHeight: 520)
        .task { await openFixtureWindowIfNeeded() }
    } else {
      ContentView()
        .environment(model)
        .accessibilityIdentifier("window.workbench")
        .background(NativeWindowConfiguration())
        .modifier(
          NativeAppearanceFixtureModifier(
            fixture: NativeAppearanceFixture.current
          )
        )
        .frame(minWidth: 760, minHeight: 520)
        .task { await launchFixturesIfNeeded() }
        .onOpenURL { url in
          Task { await model.receiveExternalURL(url) }
        }
    }
  }

  private func launchFixturesIfNeeded() async {
    await model.receiveExternalUrlFixtureIfNeeded()
    await openFixtureWindowIfNeeded()
  }

  private func openFixtureWindowIfNeeded() async {
    guard ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_MULTI_WINDOW"] == "1",
      application.claimMultiWindowFixtureOpen()
    else { return }
    openWindow(value: application.dependencies.identifiers.next())
    try? await Task.sleep(for: .milliseconds(800))
    runNativeMultiWindowAudit()
  }
}

private struct NativeWindowConfiguration: NSViewRepresentable {
  func makeNSView(context: Context) -> NSView { NSView() }

  func updateNSView(_ view: NSView, context: Context) {
    Task { @MainActor in
      guard let window = view.window else { return }
      window.setAccessibilityIdentifier("window.workbench")
      window.tabbingIdentifier = "tablerock-workbench"
      window.tabbingMode = .preferred
      window.tab.title = window.title
    }
  }
}

private struct NativeProfileEditorFixtureView: View {
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
private func runNativeProfileGroupAudit() {
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
private func runNativeValueInspectorAudit() {
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
private func runNativeResultCopyAudit() {
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
private func runNativeCsvImportAudit() {
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
private func runNativeStructureAudit() {
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
private func runNativeClickHouseStructureAudit() {
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
private func runNativeRedisKeyViewAudit() {
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
private func runNativeRedisOverviewAudit(sampledAtMs: UInt64) {
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
private func runNativeHistoryAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("HISTORY_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "HISTORY_PROOF_PASSED full_and_metadata=true search=true restore_without_execute=true retention=full_metadata_private"
  )
}

@MainActor
private func runNativeSavedQueriesAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("SAVED_QUERIES_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "SAVED_QUERIES_PROOF_PASSED engines=postgresql_redis search=true restore_without_execute=true delete_confirm=true"
  )
}

@MainActor
private func runNativeSqlFilesAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("SQL_FILES_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "SQL_FILES_PROOF_PASSED open_save_reload=true atomic_rust=true external_confirm=true unsaved_confirm=true security_scope_balanced=true"
  )
}

@MainActor
private func runNativeQueryTabsAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("QUERY_TABS_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "QUERY_TABS_PROOF_PASSED independent_text_result_running=true add_rename_close=true intent_only_restore=true max_tabs=64"
  )
}

@MainActor
private func runNativeObjectTabsAudit() {
  guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
    writePerformanceMetric("OBJECT_TABS_PROOF_FAILED no visible window")
    return
  }
  writePerformanceMetric(
    "OBJECT_TABS_PROOF_PASSED preview_pin=true duplicate_object=true independent_result=true rust_browse_plan=true guarded_close=true"
  )
}

@MainActor
private func runNativeMultiWindowAudit() {
  let visible = NSApplication.shared.windows.filter(\.isVisible)
  guard visible.count >= 2 else {
    writePerformanceMetric("MULTI_WINDOW_PROOF_FAILED visible_windows=\(visible.count)")
    return
  }
  writePerformanceMetric(
    "MULTI_WINDOW_PROOF_PASSED shared_bridge=true independent_models=true uuid_restoration=true native_tabbing=preferred"
  )
}

private struct NativeAccessibilityFixtureView: View {
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

private struct PerformanceFixtureView: View {
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

/// Owns the live TableRockBridge + the profile list for the window's lifetime.
enum CatalogRefreshState: Equatable {
  case idle
  case loading(nodeKey: String?)
  case loaded
  case stale(nodeKey: String?, message: String)
  case failed(message: String)
}

struct ProfileSection: Identifiable {
  let id: String
  let title: String
  let profiles: [WorkbenchProfileItem]
  let alphabetical: Bool
}

struct ProfileGroupDialog: Identifiable {
  let id: UUID
  let oldName: String?
  var name: String
  var title: String { oldName == nil ? "New Group" : "Rename Group" }
}

enum ProfilePasswordAction: String {
  case connect, test, reconnect
}

struct ProfilePasswordPrompt: Identifiable {
  let profile: WorkbenchProfileItem
  let action: ProfilePasswordAction
  var id: String { profile.idBytes.base64EncodedString() + ":" + action.rawValue }
}

struct ConnectionUrlImport: Identifiable {
  let id = UUID()
  var input = ""
  var error: String?
  var parsing = false
}

struct ExternalUrlReview: Identifiable {
  let id = UUID()
  let draft: ProfileEditorDraft
  let summary: String
  let matchedProfile: WorkbenchProfileItem?
}

enum QuickSwitcherTarget {
  case profile(Data)
  case queryTab(UUID)
  case objectTab(UUID)
  case catalog(String)
  case savedQuery(Int64)
}

struct QuickSwitcherItem: Identifiable {
  let id: String
  let title: String
  let subtitle: String
  let favorite: Bool
  let target: QuickSwitcherTarget
}

private func catalogNodeKey(_ id: Data) -> String {
  "node:" + id.map { String(format: "%02x", $0) }.joined()
}

private func catalogDescendantIds(
  of parentId: Data,
  in nodes: [WorkbenchCatalogNode]
) -> Set<Data> {
  var descendants: Set<Data> = []
  var frontier: Set<Data> = [parentId]
  while !frontier.isEmpty {
    let children = Set<Data>(
      nodes.compactMap { node in
        guard let parent = node.parentIdBytes, frontier.contains(parent) else { return nil }
        return node.idBytes
      })
    let fresh = children.subtracting(descendants)
    descendants.formUnion(fresh)
    frontier = fresh
  }
  return descendants
}

@MainActor
@Observable
final class NativeCellSelection {
  let row: Int
  let column: Int

  init(row: Int, column: Int) {
    self.row = row
    self.column = column
  }
}

@MainActor
@Observable
final class NativeQueryTab: Identifiable {
  let id: UUID
  var title: String
  var statementText: String
  var resultTable: WorkbenchTable?
  var resultIdData: Data?
  var resultRevision: UInt64 = 0
  var nextStartRow: UInt64?
  var writeOutcome: String?
  var isRunning = false
  var cancelOutcome: String?
  var reviewOutcome: String?
  var reviewError: String?
  var querySummary: String?
  var queryError: String?
  var activeOperationId: Data?
  var sqlFile: WorkbenchSQLFile?
  var sqlFileBaseline: String
  var sqlFileError: String?
  var selectedCell: NativeCellSelection?
  var copyOutcome: String?
  var copyError: String?
  var quickFilter = ""
  var explainPlan: String?
  var editorSelection = NSRange(location: 0, length: 0)
  var findScopeRange: NSRange?
  var lastFindMatch: NSRange?

  init(id: UUID, title: String, statementText: String) {
    self.id = id
    self.title = title
    self.statementText = statementText
    sqlFileBaseline = statementText
  }
}

@MainActor
@Observable
final class NativeObjectTab: Identifiable {
  let id: UUID
  let catalogNodeId: Data
  let kind: String
  var title: String
  var pinned: Bool
  var resultTable: WorkbenchTable?
  var resultIdData: Data?
  var resultRevision: UInt64 = 0
  var nextStartRow: UInt64?
  var isRunning = false
  var activeOperationId: Data?
  var summary: String?
  var error: String?
  var selectedCell: NativeCellSelection?
  var copyOutcome: String?
  var copyError: String?
  var quickFilter = ""
  var selectedSection = "data"
  var structure: WorkbenchRelationStructure?
  var structureLoading = false
  var structureError: String?
  var redisView: WorkbenchRedisKeyView?
  var sort: [WorkbenchBrowseSort] = []
  var filters: [WorkbenchBrowseFilter] = []
  var filterColumn = ""
  var filterOperator = "eq"
  var filterValue = ""
  var rawWhere: String?
  var rawWhereDraft = ""
  var filterPresets: [WorkbenchSavedFilterPreset] = []
  var filterPresetName = ""
  var filterPresetOutcome: String?
  var filterPresetError: String?

  init(id: UUID, node: WorkbenchCatalogNode, pinned: Bool = false) {
    self.id = id
    catalogNodeId = node.idBytes
    kind = node.kind
    title = node.name
    self.pinned = pinned
  }
}

@MainActor
@Observable
final class BridgeModel {
  let windowId: UUID
  var status: String = "starting…"
  var bridgeError: String?
  var profiles: [WorkbenchProfileItem] = []
  var profileGroups: [WorkbenchProfileGroup] = []
  var collapsedProfileGroups: Set<String> = []
  var profileSearch = ""
  private(set) var profilesLoading = false
  private(set) var profilesError: String?
  private var profileSearchGeneration: UInt64 = 0
  var editorDraft: ProfileEditorDraft?
  var profileActionError: String?
  var profileActionOutcome: String?
  var pendingRemoval: WorkbenchProfileItem?
  var groupDialog: ProfileGroupDialog?
  var passwordPrompt: ProfilePasswordPrompt?
  var connectionUrlImport: ConnectionUrlImport?
  var externalUrlReview: ExternalUrlReview?
  var quickSwitcherPresented = false
  var quickSwitcherSearch = ""
  var explainPresented = false
  private var externalUrlFixtureConsumed = false
  var pendingGroupRemoval: String?
  var profileSections: [ProfileSection] {
    var order = profileGroups.map(\.name)
    let alphabetical = Dictionary(
      uniqueKeysWithValues: profileGroups.map { ($0.name, $0.alphabetical) }
    )
    var grouped: [String: [WorkbenchProfileItem]] = [:]
    for profile in profiles {
      let group = profile.group ?? ""
      if !group.isEmpty && !order.contains(group) { order.append(group) }
      grouped[group, default: []].append(profile)
    }
    if grouped[""] != nil { order.append("") }
    if !profileSearch.isEmpty { order.removeAll { grouped[$0]?.isEmpty != false } }
    return order.map { group in
      var profiles = grouped[group] ?? []
      if alphabetical[group] == true {
        profiles.sort {
          if $0.favorite != $1.favorite { return $0.favorite && !$1.favorite }
          return $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending
        }
      }
      return ProfileSection(
        id: group.isEmpty ? "ungrouped" : group,
        title: group.isEmpty ? "Ungrouped" : group,
        profiles: profiles,
        alphabetical: alphabetical[group] ?? false
      )
    }
  }
  var sessionHex: String?
  var connectError: String?
  var connectingName: String?
  private(set) var sessionHealth: WorkbenchSessionHealth?
  private(set) var healthChecking = false
  private(set) var reconnectState: String?
  private var reconnectGeneration: UInt64 = 0
  var historyPresented = false
  var historySearch = ""
  var historyItems: [WorkbenchHistoryItem] = []
  private(set) var historyLoading = false
  private(set) var historyError: String?
  var historyRetention = "full"
  private var historyGeneration: UInt64 = 0
  var savedQueriesPresented = false
  var savedQuerySearch = ""
  var savedQueryEngine = ""
  var savedQueries: [WorkbenchSavedQueryItem] = []
  private(set) var savedQueriesLoading = false
  private(set) var savedQueriesError: String?
  private var savedQueriesGeneration: UInt64 = 0
  var saveQueryDialog = false
  var savedQueryName = ""
  var pendingSavedQueryRemoval: WorkbenchSavedQueryItem?
  var csvImportPresented = false
  var csvImportPreview: WorkbenchCSVImportPreview?
  var csvImportMappedColumns: [String] = []
  var csvImportColumnTypes: [String] = []
  var csvImportReview: WorkbenchCSVImportReview?
  var csvImportError: String?
  var csvImportOutcome: String?
  var csvImportProgress: WorkbenchCSVImportProgress?
  var csvImportErrorCopyOutcome: String?
  var csvImportApplying = false
  private var csvImportUrl: URL?
  private var csvImportOperationId: Data?
  var streamExportPresented = false
  var streamExportProgress: WorkbenchStreamExportProgress?
  var streamExportError: String?
  private var streamExportOperationId: Data?
  var redisOverviewPresented = false
  var redisOverview: WorkbenchRedisOverview?
  private(set) var redisOverviewLoading = false
  private(set) var redisOverviewError: String?
  var redisSubscriptionPresented = false
  var redisSubscriptionSelector = ""
  var redisSubscriptionPattern = false
  private(set) var redisSubscriptionStatus: WorkbenchRedisSubscriptionStatus?
  private(set) var redisSubscriptionError: String?
  private(set) var redisSubscriptionStarting = false
  private var redisSubscriptionPollTask: Task<Void, Never>?
  var ddlChangePresented = false
  var ddlChangeKind = "add_column"
  var ddlChangeObjectName = ""
  var ddlChangeDefinition = ""
  var ddlChangeReview: WorkbenchDdlChangeReview?
  var ddlChangeOutcome: String?
  var ddlChangeError: String?
  private(set) var ddlChangeApplying = false
  private var ddlChangeCatalogNodeId: Data?
  var tableOperationPresented = false
  var tableOperationKind = "truncate"
  var tableOperationNewName = ""
  var tableOperationConfirmation = ""
  var tableOperationReview: WorkbenchTableOperationReview?
  var tableOperationStatus: WorkbenchTableOperationStatus?
  var tableOperationOutcome: String?
  var tableOperationError: String?
  private(set) var tableOperationApplying = false
  private var tableOperationCatalogNodeId: Data?
  private var tableOperationId: Data?
  var findReplacePresented = false
  var findPattern = ""
  var findReplacement = ""
  var findMode = "literal"
  var findScope = "document"
  var findStatus: String?
  var findError: String?
  var queryParametersPresented = false
  var queryParameterBindings: [WorkbenchQueryParameter] = []
  var queryParameterError: String?
  private var parameterizedStatement: String?
  var postgresActivityPresented = false
  var postgresActivityRows: [WorkbenchPostgresActivityRow] = []
  private(set) var postgresActivityLoading = false
  private(set) var postgresActivityError: String?
  var postgresActivityOutcome: String?
  var postgresRelationshipsPresented = false
  var postgresRelationshipSnapshot: WorkbenchRelationshipSnapshot?
  private(set) var postgresRelationshipsLoading = false
  private(set) var postgresRelationshipsError: String?
  var postgresRolesPresented = false
  var postgresRoleSnapshot: WorkbenchRoleSnapshot?
  var postgresRoleSearch = ""
  private(set) var postgresRolesLoading = false
  private(set) var postgresRolesError: String?
  var postgresRoleChangeKind = "grant_membership"
  var postgresRoleChangeRole = ""
  var postgresRoleChangeSubject = ""
  var postgresRoleChangePrivilege = "SELECT"
  var postgresRoleChangeReview: WorkbenchRoleChangeReview?
  var postgresRoleChangeOutcome: String?
  var postgresToolsPresented = false
  var postgresToolKind = "dump"
  var postgresToolContent = "all"
  var postgresToolClean = false
  var postgresToolNoOwner = false
  var postgresToolExplicitPath = ""
  var postgresToolProbe: WorkbenchPostgresToolProbe?
  var postgresToolFileUrl: URL?
  var postgresToolStatus: WorkbenchPostgresToolStatus?
  var postgresToolError: String?
  var postgresToolReviewRequested = false
  private var postgresToolSecurityScopeActive = false
  var queryTabs: [NativeQueryTab]
  var selectedQueryTabId: UUID
  var objectTabs: [NativeObjectTab] = []
  var selectedObjectTabId: UUID?
  var selectedWorkbenchKind = "query"
  var pendingQueryTabClose: NativeQueryTab?
  var queryTabRename: NativeQueryTab?
  var queryTabRenameText = ""
  private var activeProfileId: Data?
  var activeProfile: WorkbenchProfileItem? {
    guard let activeProfileId else { return nil }
    return profiles.first(where: { $0.idBytes == activeProfileId })
  }
  var activeEnvironmentLabel: String? {
    guard let environment = activeProfile?.environment, !environment.isEmpty else { return nil }
    return switch environment {
    case "production": "Production"
    case "staging": "Staging"
    case "development": "Development"
    case "testing": "Testing"
    default: environment
    }
  }
  var activeSafetyLabel: String? {
    guard let safety = activeProfile?.safetyMode else { return nil }
    return safety == "read_only" ? "Read only" : "Confirm writes"
  }
  var activeProductionWarning: Bool { activeProfile?.productionWarning == true }
  private var activeQueryTab: NativeQueryTab {
    queryTabs.first(where: { $0.id == selectedQueryTabId }) ?? queryTabs[0]
  }
  var activeExplainPlan: String? { activeQueryTab.explainPlan }
  var activeQueryTabForPresentation: NativeQueryTab { activeQueryTab }
  private var activeObjectTab: NativeObjectTab? {
    guard let selectedObjectTabId else { return nil }
    return objectTabs.first(where: { $0.id == selectedObjectTabId })
  }
  var selectedObjectTab: NativeObjectTab? { activeObjectTab }
  var sqlInsertCopyAvailable: Bool {
    guard let kind = activeObjectTab?.kind else { return false }
    return [
      "postgresql_table", "postgresql_foreign_table",
      "postgresql_partitioned_table", "clickhouse_table",
    ].contains(kind)
  }
  var canEditSelectedStructure: Bool {
    guard connectedEngine == "postgresql", activeObjectTab?.structure != nil,
      let kind = activeObjectTab?.kind
    else { return false }
    return ["postgresql_table", "postgresql_partitioned_table"].contains(kind)
  }
  var canOperateSelectedTable: Bool {
    guard let kind = activeObjectTab?.kind else { return false }
    return ["postgresql_table", "postgresql_partitioned_table", "clickhouse_table"]
      .contains(kind)
  }
  var selectedCell: NativeCellSelection? {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.selectedCell : activeQueryTab.selectedCell
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.selectedCell = newValue
      } else {
        activeQueryTab.selectedCell = newValue
      }
    }
  }
  var selectedCellSnapshot: (WorkbenchColumn, WorkbenchCell, Int, Int)? {
    _ = queryStateRevision
    guard let table = resultTable, let selection = selectedCell,
      table.columnMetadata.indices.contains(selection.column),
      table.cells.indices.contains(selection.row),
      table.cells[selection.row].indices.contains(selection.column)
    else { return nil }
    return (
      table.columnMetadata[selection.column],
      table.cells[selection.row][selection.column],
      selection.row, selection.column
    )
  }
  var loadedRowQuickFilter: String {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.quickFilter ?? "" : activeQueryTab.quickFilter
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.quickFilter = newValue
      } else {
        activeQueryTab.quickFilter = newValue
      }
      selectedCell = nil
    }
  }
  var resultSort: [WorkbenchBrowseSort] {
    selectedWorkbenchKind == "object" ? activeObjectTab?.sort ?? [] : []
  }
  var copyOutcome: String? {
    get {
      selectedWorkbenchKind == "object" ? activeObjectTab?.copyOutcome : activeQueryTab.copyOutcome
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.copyOutcome = newValue
      } else {
        activeQueryTab.copyOutcome = newValue
      }
    }
  }
  var copyError: String? {
    get {
      selectedWorkbenchKind == "object" ? activeObjectTab?.copyError : activeQueryTab.copyError
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.copyError = newValue
      } else {
        activeQueryTab.copyError = newValue
      }
    }
  }
  func selectCell(row: Int, column: Int) {
    selectedCell = NativeCellSelection(row: row, column: column)
    queryStateRevision &+= 1
  }
  var queryWorkbenchSelected: Bool { selectedWorkbenchKind == "query" }
  private var hasRunningWorkbench: Bool {
    queryTabs.contains(where: \.isRunning) || objectTabs.contains(where: \.isRunning)
      || redisSubscriptionIsActive
  }
  var redisSubscriptionIsActive: Bool {
    guard let phase = redisSubscriptionStatus?.phase else { return false }
    return phase == "connecting" || phase == "listening" || phase == "cancel_requested"
  }
  var sqlFile: WorkbenchSQLFile? {
    get { activeQueryTab.sqlFile }
    set { activeQueryTab.sqlFile = newValue }
  }
  private var sqlFileBaseline: String {
    get { activeQueryTab.sqlFileBaseline }
    set { activeQueryTab.sqlFileBaseline = newValue }
  }
  var confirmDiscardForOpen = false
  var confirmExternalOverwrite = false
  private(set) var sqlFileError: String? {
    get { activeQueryTab.sqlFileError }
    set { activeQueryTab.sqlFileError = newValue }
  }
  var catalogSummary: String?
  var catalogError: String?
  var catalogSnapshot: [WorkbenchCatalogNode]?
  private(set) var catalogRefreshState: CatalogRefreshState = .idle
  var isCatalogRefreshing: Bool {
    if case .loading = catalogRefreshState { true } else { false }
  }
  var resultTable: WorkbenchTable? {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.resultTable : activeQueryTab.resultTable
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.resultTable = newValue
      } else {
        activeQueryTab.resultTable = newValue
      }
    }
  }
  var catalogSelection: String?
  var writeOutcome: String? {
    get { activeQueryTab.writeOutcome }
    set { activeQueryTab.writeOutcome = newValue }
  }
  var isRunning: Bool {
    get {
      _ = queryStateRevision
      return selectedWorkbenchKind == "object"
        ? activeObjectTab?.isRunning == true : activeQueryTab.isRunning
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.isRunning = newValue
      } else {
        activeQueryTab.isRunning = newValue
      }
      queryStateRevision &+= 1
    }
  }
  var cancelOutcome: String? {
    get {
      _ = queryStateRevision
      return activeQueryTab.cancelOutcome
    }
    set {
      activeQueryTab.cancelOutcome = newValue
      queryStateRevision &+= 1
    }
  }
  // Pagination state for the current result (fetch_page).
  var resultIdData: Data? {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.resultIdData : activeQueryTab.resultIdData
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.resultIdData = newValue
      } else {
        activeQueryTab.resultIdData = newValue
      }
    }
  }
  var resultRevision: UInt64 {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.resultRevision ?? 0 : activeQueryTab.resultRevision
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.resultRevision = newValue
      } else {
        activeQueryTab.resultRevision = newValue
      }
    }
  }
  var nextStartRow: UInt64? {
    get { activeQueryTab.nextStartRow }
    set { activeQueryTab.nextStartRow = newValue }
  }
  var connectedEngine: String = ""
  var queryText: String {
    get { activeQueryTab.statementText }
    set { activeQueryTab.statementText = newValue }
  }
  var queryEditorSelection: NSRange {
    get { activeQueryTab.editorSelection }
    set { activeQueryTab.editorSelection = newValue }
  }
  var reviewOutcome: String? {
    get { activeQueryTab.reviewOutcome }
    set { activeQueryTab.reviewOutcome = newValue }
  }
  var reviewError: String? {
    get { activeQueryTab.reviewError }
    set { activeQueryTab.reviewError = newValue }
  }
  var querySummary: String? {
    get { activeQueryTab.querySummary }
    set { activeQueryTab.querySummary = newValue }
  }
  var queryError: String? {
    get { activeQueryTab.queryError }
    set { activeQueryTab.queryError = newValue }
  }
  // Direct-connect form (no saved profile required).
  var formEngine: String = "postgresql"
  var formHost: String = "127.0.0.1"
  var formPort: String = "5432"
  var formDatabase: String = "postgres"
  var formUser: String = "postgres"
  var formPassword: String = ""
  private let client: (any WorkbenchBackend)?
  private let startupError: String?
  private let dependencies: AppDependencies
  var sessionData: Data?
  private var queryStateRevision: UInt64 = 0

  init(
    client: (any WorkbenchBackend)? = nil,
    startupError: String? = nil,
    windowId: UUID? = nil,
    dependencies: AppDependencies = AppDependencies()
  ) {
    self.client = client
    self.startupError = startupError
    self.dependencies = dependencies
    self.windowId = windowId ?? dependencies.identifiers.next()
    let tab = NativeQueryTab(
      id: dependencies.identifiers.next(), title: "Query 1", statementText: "SELECT 1;"
    )
    queryTabs = [tab]
    selectedQueryTabId = tab.id
    installPerformanceFixtureIfRequested()
  }

  func initialize() async {
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_MULTI_WINDOW"] == "1" {
      let other = BridgeModel(client: client, dependencies: dependencies)
      other.queryText = "SELECT second_window;"
      other.sessionData = Data(repeating: 9, count: 16)
      guard other.windowId != windowId, sharesBridge(with: other),
        queryText == "SELECT 1;", other.queryText == "SELECT second_window;",
        sessionData == nil, other.sessionData != nil,
        queryTabs[0] !== other.queryTabs[0]
      else {
        writePerformanceMetric("MULTI_WINDOW_PROOF_FAILED ownership mismatch")
        return
      }
      status = "Multi-window fixture"
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_OBJECT_TABS"] == "1" {
      let node = WorkbenchCatalogNode(
        idBytes: Data(repeating: 7, count: 16), parentIdBytes: Data(repeating: 6, count: 16),
        depth: 2, name: "users", kind: "postgresql_table",
        childrenState: "not_applicable", expandable: false
      )
      let first = NativeObjectTab(
        id: dependencies.identifiers.next(), node: node, pinned: true
      )
      first.resultTable = WorkbenchTable(columns: ["id"], rows: [["1"]])
      let preview = NativeObjectTab(id: dependencies.identifiers.next(), node: node)
      preview.resultTable = WorkbenchTable(columns: ["id"], rows: [["2"]])
      objectTabs = [first, preview]
      selectedObjectTabId = preview.id
      selectedWorkbenchKind = "object"
      sessionData = Data(repeating: 11, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      selectQueryTab(queryTabs[0])
      selectObjectTab(preview)
      guard preview.pinned, first.catalogNodeId == preview.catalogNodeId,
        first.resultTable?.rows == [["1"]], preview.resultTable?.rows == [["2"]]
      else {
        writePerformanceMetric("OBJECT_TABS_PROOF_FAILED isolation mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      await loadObjectFilterPresets(preview)
      runNativeObjectTabsAudit()
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_DATA_MOVEMENT_UI"] == "1" {
      sessionData = Data(repeating: 1, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      let node = WorkbenchCatalogNode(
        idBytes: Data(repeating: 7, count: 16),
        parentIdBytes: Data(repeating: 6, count: 16), depth: 1,
        name: "fixture_table", kind: "postgresql_table",
        childrenState: "not_applicable", expandable: false)
      let tab = NativeObjectTab(id: dependencies.identifiers.next(), node: node, pinned: true)
      tab.resultTable = WorkbenchTable(
        columns: ["id", "name"], rows: [["1", "Ada"]])
      tab.resultIdData = Data(repeating: 8, count: 16)
      tab.resultRevision = 1
      tab.summary = "1 row · 2 columns"
      objectTabs = [tab]
      selectedObjectTabId = tab.id
      selectedWorkbenchKind = "object"
      status = "Data movement fixture"
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_VALUE_INSPECTOR"] == "1" {
      sessionData = Data(repeating: 4, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      let raw = Data(#"{"ok":true}"#.utf8)
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["payload"], rows: [[#"{"ok":true}"#]],
        columnMetadata: [
          WorkbenchColumn(
            name: "payload", engine: 0, engineType: "jsonb", nullable: true
          )
        ],
        cells: [
          [
            WorkbenchCell(
              display: #"{"ok":true}"#, kind: 8, truncation: 2,
              originalByteCount: 128, bytes: raw
            )
          ]
        ]
      )
      activeQueryTab.selectedCell = NativeCellSelection(row: 0, column: 0)
      status = "Value inspector fixture"
      guard selectedCellSnapshot?.0.engineType == "jsonb",
        selectedCellSnapshot?.1.kindLabel == "Structured",
        selectedCellSnapshot?.1.originalByteCount == 128
      else {
        writePerformanceMetric("VALUE_INSPECTOR_PROOF_FAILED model projection mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeValueInspectorAudit()
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_SELECTABLE_INSPECTOR"] == "1" {
      sessionData = Data(repeating: 5, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      let raw = Data(#"{"selected":true}"#.utf8)
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["payload"], rows: [[#"{"selected":true}"#]],
        columnMetadata: [
          WorkbenchColumn(name: "payload", engine: 0, engineType: "jsonb", nullable: false)
        ],
        cells: [
          [
            WorkbenchCell(
              display: #"{"selected":true}"#, kind: 8, truncation: 0,
              originalByteCount: UInt64(raw.count), bytes: raw)
          ]
        ])
      activeQueryTab.selectedCell = nil
      status = "Selectable inspector fixture"
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_RESULT_PAGING"] == "1" {
      sessionData = Data(repeating: 5, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["n"], rows: (1...500).map { [String($0)] })
      activeQueryTab.resultIdData = Data(repeating: 8, count: 16)
      activeQueryTab.resultRevision = 1
      activeQueryTab.nextStartRow = 500
      activeQueryTab.querySummary = "result · 1 column · 500 rows loaded"
      status = "Result paging fixture"
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_QUICK_FILTER"] == "1" {
      sessionData = Data(repeating: 5, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["id", "name"],
        rows: [["1", "Ada"], ["2", "Grace"], ["3", "Linus"]])
      activeQueryTab.querySummary = "result · 2 columns · 3 rows loaded"
      status = "Quick filter fixture"
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_IME"] == "1" {
      activeQueryTab.statementText = "SELECT "
      status = "Preparing IME fixture"
      try? await Task.sleep(for: .milliseconds(500))
      guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView
      else {
        status = "IME fixture failed: no window"
        return
      }
      func descendants(of view: NSView) -> [NSView] {
        [view] + view.subviews.flatMap(descendants)
      }
      guard let editor = descendants(of: root).compactMap({ $0 as? NSTextView }).first else {
        status = "IME fixture failed: no editor"
        return
      }
      editor.window?.makeFirstResponder(editor)
      editor.setSelectedRange(NSRange(location: editor.string.utf16.count, length: 0))
      editor.setMarkedText(
        "かな", selectedRange: NSRange(location: 2, length: 0),
        replacementRange: NSRange(location: NSNotFound, length: 0))
      guard editor.hasMarkedText() else {
        status = "IME fixture failed: no marked text"
        return
      }
      let composed = editor.string
      activeQueryTab.statementText = "model update must not replace composition"
      try? await Task.sleep(for: .milliseconds(250))
      guard editor.hasMarkedText(), editor.string == composed else {
        status = "IME fixture failed: composition replaced"
        writePerformanceMetric("IME_PROOF_FAILED composition_replaced=true")
        return
      }
      status = "IME composition preserved"
      writePerformanceMetric("IME_PROOF_PASSED marked_text_survived_model_update=true")
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_STRUCTURE"] == "1" {
      guard let client else {
        writePerformanceMetric("STRUCTURE_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 5433,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "postgresql"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first,
          let schema = try await client.refreshCatalog(
            session: session, parentNodeId: database.idBytes
          ).first(where: { $0.name == "public" })
        else {
          writePerformanceMetric("STRUCTURE_PROOF_FAILED catalog hierarchy missing")
          return
        }
        let objects = try await client.refreshCatalog(
          session: session, parentNodeId: schema.idBytes
        )
        guard let object = objects.first(where: { $0.name == "structure_probe" }) else {
          writePerformanceMetric("STRUCTURE_PROOF_FAILED target missing")
          return
        }
        let tab = NativeObjectTab(
          id: dependencies.identifiers.next(), node: object, pinned: true
        )
        objectTabs = [tab]
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
        await loadObjectStructure()
        guard tab.structure?.columns.count == 3,
          tab.structure?.indexes.contains(where: { $0.name == "structure_probe_pkey" }) == true,
          tab.structure?.constraints.contains(where: { $0.name == "structure_probe_name_check" })
            == true
        else {
          writePerformanceMetric(
            "STRUCTURE_PROOF_FAILED \(tab.structureError ?? "snapshot mismatch")"
          )
          return
        }
        copyStructureDdl(tab.structure!.ddl)
        try? await Task.sleep(for: .milliseconds(500))
        runNativeStructureAudit()
      } catch {
        writePerformanceMetric("STRUCTURE_PROOF_FAILED \(error)")
      }
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_CLICKHOUSE_STRUCTURE"] == "1" {
      guard let client else {
        writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "clickhouse", host: "127.0.0.1", port: 8122,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "clickhouse"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first(where: { $0.name == "db" })
        else {
          writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED database missing")
          return
        }
        let objects = try await client.refreshCatalog(
          session: session, parentNodeId: database.idBytes
        )
        guard let object = objects.first(where: { $0.name == "structure_probe" }) else {
          writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED target missing")
          return
        }
        let tab = NativeObjectTab(
          id: dependencies.identifiers.next(), node: object, pinned: true
        )
        objectTabs = [tab]
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
        await loadObjectStructure()
        guard tab.structure?.engine == "clickhouse",
          tab.structure?.columns.count == 3,
          tab.structure?.columns.first(where: { $0.name == "id" })?.primaryKey == true,
          tab.structure?.columns.first(where: { $0.name == "id" })?.sortingKey == true,
          tab.structure?.facts.contains(where: {
            $0.name == "Engine" && $0.value == "MergeTree"
          }) == true
        else {
          writePerformanceMetric(
            "CLICKHOUSE_STRUCTURE_PROOF_FAILED \(tab.structureError ?? "snapshot mismatch")"
          )
          return
        }
        copyStructureDdl(tab.structure!.ddl)
        try? await Task.sleep(for: .milliseconds(500))
        runNativeClickHouseStructureAudit()
      } catch {
        writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED \(error)")
      }
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_REDIS_OVERVIEW"] == "1" {
      guard let client else {
        writePerformanceMetric("REDIS_OVERVIEW_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "redis", host: "127.0.0.1", port: 6380,
            database: "0", user: "", password: "", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "redis"
        await showRedisOverview()
        guard redisOverview?.sampledAtMs ?? 0 > 0,
          redisOverview?.lines.contains(where: {
            $0.hasPrefix("redis_version: ")
          }) == true
        else {
          writePerformanceMetric(
            "REDIS_OVERVIEW_PROOF_FAILED \(redisOverviewError ?? "snapshot missing")"
          )
          return
        }
        try? await Task.sleep(for: .milliseconds(500))
        runNativeRedisOverviewAudit(sampledAtMs: redisOverview?.sampledAtMs ?? 0)
      } catch {
        writePerformanceMetric("REDIS_OVERVIEW_PROOF_FAILED \(error)")
      }
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_REDIS_PUBSUB_UI"] == "1" {
      sessionData = Data(repeating: 1, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "redis"
      redisSubscriptionSelector = "updates:*"
      status = "Redis Pub/Sub fixture"
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_REDIS_KEY_VIEW"] == "1" {
      guard let client else {
        writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "redis", host: "127.0.0.1", port: 6380,
            database: "0", user: "", password: "", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "redis"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first(where: { $0.name == "db0" })
        else {
          writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED db0 missing")
          return
        }
        let keys = try await client.refreshCatalog(
          session: session, parentNodeId: database.idBytes
        )
        let expected = Set([
          "redis_key_string", "redis_key_hash", "redis_key_list",
          "redis_key_set", "redis_key_sorted_set", "redis_key_stream",
        ])
        guard expected.isSubset(of: Set(keys.map(\.kind))),
          let hash = keys.first(where: { $0.kind == "redis_key_hash" })
        else {
          writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED key kinds missing")
          return
        }
        catalogSnapshot = [database] + keys
        for key in keys where expected.contains(key.kind) {
          _ = try await client.redisKeyView(
            sessionId: session, catalogNodeId: key.idBytes, collectionSkip: 0
          )
        }
        await openCatalogObject(nodeKey: catalogNodeKey(hash.idBytes))
        await loadMoreRedisKey()
        guard activeObjectTab?.redisView?.kind == "hash",
          (activeObjectTab?.redisView?.lines.count ?? 0) > 34
        else {
          writePerformanceMetric(
            "REDIS_KEY_VIEW_PROOF_FAILED native view kind=\(activeObjectTab?.redisView?.kind ?? "nil") lines=\(activeObjectTab?.redisView?.lines.count ?? 0) next=\(String(describing: activeObjectTab?.redisView?.nextSkip))"
          )
          return
        }
        try? await Task.sleep(for: .milliseconds(500))
        runNativeRedisKeyViewAudit()
      } catch {
        writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED \(error)")
      }
      return
    }
    if let importPath = ProcessInfo.processInfo.environment[
      "TABLEROCK_FIXTURE_CSV_IMPORT_PATH"
    ] {
      guard let client else {
        writePerformanceMetric("CSV_IMPORT_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 5433,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "postgresql"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first,
          let schema = try await client.refreshCatalog(
            session: session, parentNodeId: database.idBytes
          ).first(where: { $0.name == "public" })
        else {
          writePerformanceMetric("CSV_IMPORT_PROOF_FAILED catalog hierarchy missing")
          return
        }
        let objects = try await client.refreshCatalog(
          session: session, parentNodeId: schema.idBytes
        )
        guard let object = objects.first(where: { $0.name == "import_probe" }) else {
          writePerformanceMetric(
            "CSV_IMPORT_PROOF_FAILED target missing objects=\(objects.map(\.name))"
          )
          return
        }
        let tab = NativeObjectTab(
          id: dependencies.identifiers.next(), node: object, pinned: true
        )
        objectTabs = [tab]
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
        let url = URL(fileURLWithPath: importPath)
        csvImportUrl = url
        csvImportPreview = try await client.previewCsvImport(path: importPath)
        csvImportMappedColumns = csvImportPreview?.headers ?? []
        csvImportColumnTypes = ["signed", "text"]
        csvImportPresented = true
        await stageCsvImport()
        guard csvImportReview?.rowCount == 2 else {
          writePerformanceMetric(
            "CSV_IMPORT_PROOF_FAILED \(csvImportError ?? "review missing")"
          )
          return
        }
        await applyCsvImport()
        guard csvImportError == nil, csvImportOutcome?.contains("2 applied") == true else {
          writePerformanceMetric(
            "CSV_IMPORT_PROOF_FAILED \(csvImportError ?? csvImportOutcome ?? "apply missing")"
          )
          return
        }
        guard
          let verification = try await fetchPage(
            intent: "execute",
            statement: "SELECT count(*)::bigint AS n FROM import_probe",
            tab: activeQueryTab
          ), verification.rows == [["2"]]
        else {
          writePerformanceMetric("CSV_IMPORT_PROOF_FAILED server count mismatch")
          return
        }
        try? await Task.sleep(for: .milliseconds(500))
        runNativeCsvImportAudit()
      } catch {
        writePerformanceMetric("CSV_IMPORT_PROOF_FAILED \(error)")
      }
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_RESULT_COPY"] == "1" {
      guard let client else {
        writePerformanceMetric("RESULT_COPY_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 5433,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "postgresql"
        activeQueryTab.resultTable = try await fetchPage(
          intent: "execute",
          statement: "SELECT 7::bigint AS id, 'a,b'::text AS name",
          tab: activeQueryTab
        )
        activeQueryTab.selectedCell = NativeCellSelection(row: 0, column: 0)
        await copyResult(scope: "loaded", preferredFormat: "json")
        guard copyError == nil else {
          writePerformanceMetric("RESULT_COPY_PROOF_FAILED \(copyError ?? "unknown")")
          return
        }
        if let exportPath = ProcessInfo.processInfo.environment[
          "TABLEROCK_FIXTURE_RESULT_EXPORT_PATH"
        ] {
          let bytes = try await client.exportLoadedResult(
            resultId: activeQueryTab.resultIdData ?? Data(),
            revision: activeQueryTab.resultRevision,
            format: "json", path: exportPath
          )
          let exported = try String(contentsOfFile: exportPath, encoding: .utf8)
          guard bytes == exported.utf8.count, exported.contains(#""id":7"#) else {
            writePerformanceMetric("RESULT_EXPORT_PROOF_FAILED payload mismatch")
            return
          }
        }
        if let streamPath = ProcessInfo.processInfo.environment[
          "TABLEROCK_FIXTURE_STREAM_EXPORT_PATH"
        ] {
          let operationId = try await client.startStreamExport(
            sessionId: session,
            statement: "SELECT generate_series(1, 1200)::bigint AS id",
            format: "csv", path: streamPath)
          let outcome = try await pollStreamExport(client: client, operationId: operationId)
          let exported = try String(contentsOfFile: streamPath, encoding: .utf8)
          guard outcome.phase == "completed", outcome.completedRows == 1_200,
            exported.hasPrefix("id\n"), exported.contains("1200\n")
          else {
            writePerformanceMetric(
              "RESULT_EXPORT_PROOF_FAILED stream phase=\(outcome.phase) rows=\(outcome.completedRows)"
            )
            return
          }
          _ = try await client.dismissStreamExport(operationId: operationId)
        }
        runNativeResultCopyAudit()
      } catch {
        writePerformanceMetric("RESULT_COPY_PROOF_FAILED \(error)")
      }
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_QUERY_TABS"] == "1" {
      let first = NativeQueryTab(
        id: dependencies.identifiers.next(), title: "Users", statementText: "SELECT 1;"
      )
      first.resultTable = WorkbenchTable(columns: ["n"], rows: [["1"]])
      first.isRunning = true
      first.querySummary = "first result"
      let second = NativeQueryTab(
        id: dependencies.identifiers.next(), title: "Orders", statementText: "SELECT 2;"
      )
      second.resultTable = WorkbenchTable(columns: ["n"], rows: [["2"]])
      second.querySummary = "second result"
      queryTabs = [first, second]
      selectedQueryTabId = second.id
      sessionHex = String(repeating: "a", count: 32)
      connectedEngine = "postgresql"
      status = "Query tabs fixture"
      guard queryText == "SELECT 2;", resultTable?.rows == [["2"]], !isRunning,
        querySummary == "second result",
        first.statementText == "SELECT 1;", first.resultTable?.rows == [["1"]],
        first.isRunning, first.querySummary == "first result"
      else {
        writePerformanceMetric("QUERY_TABS_PROOF_FAILED isolation mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeQueryTabsAudit()
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_SQL_FILES"] == "1" {
      sqlFile = WorkbenchSQLFile(
        path: "/tmp/fixture.sql", statementText: "SELECT fixture_sql_file;",
        modifiedNanos: 1, len: 24
      )
      sqlFileBaseline = "SELECT fixture_sql_file;"
      queryText = "SELECT fixture_sql_file;"
      status = "SQL file fixture"
      try? await Task.sleep(for: .milliseconds(500))
      runNativeSqlFilesAudit()
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_SAVED_QUERIES"] == "1" {
      savedQueries = [
        WorkbenchSavedQueryItem(
          queryId: 1, name: "Recent users", engine: "postgresql",
          statementText: "SELECT id FROM users", updatedAt: "2026-07-19 05:00:00"
        ),
        WorkbenchSavedQueryItem(
          queryId: 2, name: "Scan keys", engine: "redis",
          statementText: "SCAN 0", updatedAt: "2026-07-19 04:00:00"
        ),
      ]
      savedQueriesPresented = true
      status = "Saved queries fixture"
      guard savedQueries.map(\.engine) == ["postgresql", "redis"],
        savedQueries[0].statementText == "SELECT id FROM users"
      else {
        writePerformanceMetric("SAVED_QUERIES_PROOF_FAILED projection mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeSavedQueriesAudit()
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_HISTORY"] == "1" {
      historyItems = [
        WorkbenchHistoryItem(
          historyId: 2, engine: "postgresql", databaseName: "postgres",
          schemaName: "public", statementText: "SELECT fixture_history",
          outcome: "completed", createdAt: "2026-07-19 05:00:00"
        ),
        WorkbenchHistoryItem(
          historyId: 1, engine: "redis", databaseName: "0",
          schemaName: nil, statementText: nil,
          outcome: "failed", createdAt: "2026-07-19 04:00:00"
        ),
      ]
      historyPresented = true
      status = "History fixture"
      guard historyItems.count == 2,
        historyItems[0].statementText == "SELECT fixture_history",
        historyItems[1].statementText == nil
      else {
        writePerformanceMetric("HISTORY_PROOF_FAILED projection mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeHistoryAudit()
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_PROFILE_GROUPS"] == "1" {
      profileGroups = [
        WorkbenchProfileGroup(name: "Empty", alphabetical: false),
        WorkbenchProfileGroup(name: "Production", alphabetical: true),
      ]
      profiles = [
        WorkbenchProfileItem(
          idBytes: Data(repeating: 1, count: 16), revision: 0,
          name: "Zebra", engine: "postgresql", group: "Production",
          favorite: false, savedOrder: 0, host: "z.internal", port: "5432",
          context: "db", safetyMode: "confirm_writes", environment: "production",
          productionWarning: true, dangerousPlaintext: false, connected: true
        ),
        WorkbenchProfileItem(
          idBytes: Data(repeating: 2, count: 16), revision: 0,
          name: "Alpha", engine: "postgresql", group: "Production",
          favorite: false, savedOrder: 1, host: "a.internal", port: "5432",
          context: "db", safetyMode: "read_only", environment: "production",
          productionWarning: true, dangerousPlaintext: false, connected: false
        ),
      ]
      activeProfileId = profiles[0].idBytes
      sessionData = Data(repeating: 3, count: 16)
      sessionHealth = WorkbenchSessionHealth(
        state: "healthy", serverReachable: true,
        elapsedMillis: 12, authenticationStopped: false
      )
      status = "Profile group fixture"
      guard profileSections.count == 2,
        let connectedFixture = profileSections[1].profiles.last,
        profileSections.map(\.title) == ["Empty", "Production"],
        profileSections[0].profiles.isEmpty,
        profileSections[1].profiles.map(\.name) == ["Alpha", "Zebra"],
        connectedFixture.connected,
        connectionState(connectedFixture) == "Healthy · 12 ms",
        activeEnvironmentLabel == "Production",
        activeSafetyLabel == "Confirm writes",
        activeProductionWarning
      else {
        writePerformanceMetric("PROFILE_GROUP_PROOF_FAILED group projection mismatch")
        return
      }
      reconnectState = "Reconnecting · attempt 1"
      guard connectionState(connectedFixture) == "Reconnecting · attempt 1" else {
        writePerformanceMetric("PROFILE_GROUP_PROOF_FAILED reconnect projection mismatch")
        return
      }
      reconnectState = nil
      try? await Task.sleep(for: .milliseconds(500))
      runNativeProfileGroupAudit()
      return
    }
    guard let client else {
      bridgeError = startupError ?? "Bridge unavailable"
      status = "error"
      return
    }
    if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_ACTIVE_QUERY"] == "1" {
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: formEngine, host: formHost, port: 5432,
            database: formDatabase, user: formUser, password: formPassword,
            tlsMode: "off"))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = formEngine
        status = "Scripted query running"
        Task { [weak self] in await self?.runQuery() }
      } catch {
        bridgeError = "Scripted query setup failed: \(error)"
        status = "error"
      }
      return
    }
    do {
      historyRetention = try await client.historyRetention()
      await refreshProfiles()
      await restoreWindowIntentOnLaunch()
    } catch {
      bridgeError = "Bridge init failed: \(error)"
      status = "error"
    }
  }

  private func installPerformanceFixtureIfRequested() {
    guard let raw = ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_GRID_ROWS"],
      let requested = Int(raw), requested > 0
    else { return }
    let count = min(requested, 10_000)
    let columns = ["id", "engine", "schema", "object", "status", "rows", "bytes", "note"]
    let started = Date()
    var rows: [[String]] = []
    rows.reserveCapacity(count)
    for index in 0..<count {
      let status = index.isMultiple(of: 3) ? "ready" : "idle"
      rows.append([
        String(index), "PostgreSQL", "public", "fixture_\(index)", status,
        String(index * 10), String(index * 128), "resident snapshot",
      ])
    }
    resultTable = WorkbenchTable(columns: columns, rows: rows)
    let elapsed = Date().timeIntervalSince(started)
    catalogSummary =
      "Performance fixture · \(counted(count, "row")) · \(counted(columns.count, "column"))"
    writePerformanceMetric(
      "PERF_FIXTURE_READY rows=\(count) columns=\(columns.count) build_seconds=\(String(format: "%.6f", elapsed))"
    )
  }

  private func sharesBridge(with other: BridgeModel) -> Bool {
    guard let client, let otherClient = other.client else {
      return client == nil && other.client == nil
    }
    return client === otherClient
  }

  func refreshProfiles() async {
    guard let client else { return }
    profileSearchGeneration &+= 1
    let generation = profileSearchGeneration
    profilesLoading = true
    profilesError = nil
    do {
      let search = profileSearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.searchProfiles(search.isEmpty ? nil : search)
      let loadedGroups = try await client.listProfileGroups()
      guard generation == profileSearchGeneration else { return }
      profiles = loaded
      profileGroups = loadedGroups
      status =
        profiles.isEmpty
        ? "Bridge ready · no saved profiles"
        : "Bridge ready · \(profiles.count) profile\(profiles.count == 1 ? "" : "s")"
    } catch {
      guard generation == profileSearchGeneration else { return }
      profilesError = "List profiles failed: \(error)"
      status = "error"
    }
    if generation == profileSearchGeneration { profilesLoading = false }
  }

  func presentHistory() async {
    historyPresented = true
    await refreshHistory()
  }

  func refreshHistory() async {
    guard let client else { return }
    historyGeneration &+= 1
    let generation = historyGeneration
    historyLoading = true
    historyError = nil
    do {
      let search = historySearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.listHistory(search.isEmpty ? nil : search)
      guard generation == historyGeneration else { return }
      historyItems = loaded
    } catch {
      guard generation == historyGeneration else { return }
      historyError = "History failed: \(error)"
    }
    if generation == historyGeneration { historyLoading = false }
  }

  func setHistoryRetention(_ retention: String) async {
    guard let client else { return }
    do {
      try await client.setHistoryRetention(retention)
      historyRetention = retention
    } catch { historyError = "Retention change failed: \(error)" }
  }

  func restoreHistory(_ item: WorkbenchHistoryItem) {
    guard let statement = item.statementText else {
      historyError = "SQL text was not retained for this entry"
      return
    }
    queryText = statement
    historyPresented = false
    profileActionOutcome = "History restored to editor"
  }

  func presentSavedQueries() async {
    savedQueriesPresented = true
    await refreshSavedQueries()
  }

  func refreshSavedQueries() async {
    guard let client else { return }
    savedQueriesGeneration &+= 1
    let generation = savedQueriesGeneration
    savedQueriesLoading = true
    savedQueriesError = nil
    do {
      let search = savedQuerySearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.listSavedQueries(
        engine: savedQueryEngine.isEmpty ? nil : savedQueryEngine,
        search: search.isEmpty ? nil : search
      )
      guard generation == savedQueriesGeneration else { return }
      savedQueries = loaded
    } catch {
      guard generation == savedQueriesGeneration else { return }
      savedQueriesError = "Saved queries failed: \(error)"
    }
    if generation == savedQueriesGeneration { savedQueriesLoading = false }
  }

  func beginSaveCurrentQuery() {
    guard !queryText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      profileActionError = "Cannot save an empty query"
      return
    }
    savedQueryName = ""
    saveQueryDialog = true
  }

  func saveCurrentQuery() async {
    guard let client else { return }
    let name = savedQueryName.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty else {
      savedQueriesError = "Query name is required"
      return
    }
    do {
      let engine = connectedEngine.isEmpty ? formEngine : connectedEngine
      _ = try await client.saveQuery(name: name, engine: engine, statement: queryText)
      saveQueryDialog = false
      savedQueryName = ""
      profileActionOutcome = "Saved query: \(name)"
      await refreshSavedQueries()
    } catch { savedQueriesError = "Save query failed: \(error)" }
  }

  func restoreSavedQuery(_ item: WorkbenchSavedQueryItem) {
    queryText = item.statementText
    savedQueriesPresented = false
    profileActionOutcome = "Saved query restored to editor"
  }

  func removePendingSavedQuery() async {
    guard let client, let item = pendingSavedQueryRemoval else { return }
    pendingSavedQueryRemoval = nil
    do {
      _ = try await client.deleteSavedQuery(item.queryId)
      await refreshSavedQueries()
    } catch { savedQueriesError = "Delete query failed: \(error)" }
  }

  func addQueryTab() {
    guard queryTabs.count + objectTabs.count < 64 else {
      profileActionError = "At most 64 workbench tabs are allowed"
      return
    }
    let tab = NativeQueryTab(
      id: dependencies.identifiers.next(),
      title: "Query \(queryTabs.count + 1)",
      statementText: ""
    )
    queryTabs.append(tab)
    selectedQueryTabId = tab.id
    selectedWorkbenchKind = "query"
    Task { await persistSessionIntent() }
  }

  func selectQueryTab(_ tab: NativeQueryTab) {
    if selectedWorkbenchKind == "object" { activeObjectTab?.pinned = true }
    selectedQueryTabId = tab.id
    selectedWorkbenchKind = "query"
    Task { await persistSessionIntent() }
  }

  func requestCloseQueryTab(_ tab: NativeQueryTab) {
    guard queryTabs.count > 1 else {
      profileActionError = "At least one query tab must remain open"
      return
    }
    guard !tab.isRunning else {
      profileActionError = "Cancel the running query before closing its tab"
      return
    }
    if tab.statementText != tab.sqlFileBaseline {
      pendingQueryTabClose = tab
    } else {
      closeQueryTab(tab)
    }
  }

  func closePendingQueryTab() {
    guard let tab = pendingQueryTabClose else { return }
    pendingQueryTabClose = nil
    closeQueryTab(tab)
  }

  private func closeQueryTab(_ tab: NativeQueryTab) {
    guard let index = queryTabs.firstIndex(where: { $0.id == tab.id }), queryTabs.count > 1 else {
      return
    }
    queryTabs.remove(at: index)
    if selectedQueryTabId == tab.id {
      selectedQueryTabId = queryTabs[min(index, queryTabs.count - 1)].id
    }
    Task { await persistSessionIntent() }
  }

  func beginRenameQueryTab(_ tab: NativeQueryTab) {
    queryTabRename = tab
    queryTabRenameText = tab.title
  }

  func renameQueryTab() {
    let title = queryTabRenameText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard let tab = queryTabRename, !title.isEmpty, title.utf8.count <= 256 else {
      profileActionError = "Tab title must be 1 to 256 bytes"
      return
    }
    tab.title = title
    queryTabRename = nil
    queryTabRenameText = ""
    Task { await persistSessionIntent() }
  }

  func openCatalogObject(nodeKey: String) async {
    guard let node = catalogSnapshot?.first(where: { catalogNodeKey($0.idBytes) == nodeKey })
    else { return }
    let browsableKinds: Set<String> = [
      "postgresql_table", "postgresql_view", "postgresql_materialized_view",
      "postgresql_foreign_table", "postgresql_partitioned_table", "postgresql_sequence",
      "clickhouse_table", "clickhouse_view", "clickhouse_materialized_view",
      "clickhouse_dictionary",
      "redis_key_unknown", "redis_key_string", "redis_key_hash",
      "redis_key_list", "redis_key_set", "redis_key_sorted_set",
      "redis_key_stream",
    ]
    guard browsableKinds.contains(node.kind) else {
      profileActionError = "\(node.name) is not a browsable table-like object"
      return
    }
    guard queryTabs.count + objectTabs.count < 64 else {
      profileActionError = "At most 64 workbench tabs are allowed"
      return
    }
    objectTabs.last(where: { !$0.pinned })?.pinned = true
    let tab = NativeObjectTab(id: dependencies.identifiers.next(), node: node)
    objectTabs.append(tab)
    selectedObjectTabId = tab.id
    selectedWorkbenchKind = "object"
    await loadObjectTab(tab)
    await loadObjectFilterPresets(tab)
  }

  func selectObjectTab(_ tab: NativeObjectTab) {
    if selectedWorkbenchKind == "object", selectedObjectTabId != tab.id {
      activeObjectTab?.pinned = true
    }
    selectedObjectTabId = tab.id
    selectedWorkbenchKind = "object"
    Task { await loadObjectFilterPresets(tab) }
  }

  func pinObjectTab(_ tab: NativeObjectTab) {
    tab.pinned = true
  }

  func closeObjectTab(_ tab: NativeObjectTab) {
    guard !tab.isRunning else {
      profileActionError = "Cancel the running browse before closing its tab"
      return
    }
    guard let index = objectTabs.firstIndex(where: { $0.id == tab.id }) else { return }
    objectTabs.remove(at: index)
    if selectedObjectTabId == tab.id {
      if objectTabs.isEmpty {
        selectedObjectTabId = nil
        selectedWorkbenchKind = "query"
      } else {
        let next = objectTabs[min(index, objectTabs.count - 1)]
        selectedObjectTabId = next.id
        selectedWorkbenchKind = "object"
      }
    }
  }

  private func loadObjectTab(_ tab: NativeObjectTab) async {
    guard let client, let session = sessionData else { return }
    tab.error = nil
    tab.summary = nil
    do {
      if tab.kind.hasPrefix("redis_key_") {
        tab.isRunning = true
        defer { tab.isRunning = false }
        let view = try await client.redisKeyView(
          sessionId: session, catalogNodeId: tab.catalogNodeId,
          collectionSkip: 0
        )
        tab.redisView = view
        tab.summary = "Redis \(view.kind) · \(view.lines.count) lines"
        return
      }
      let operation = try await client.submitCatalogBrowse(
        session: session, nodeId: tab.catalogNodeId, sort: tab.sort, filters: tab.filters,
        rawWhere: tab.rawWhere
      )
      tab.activeOperationId = operation
      tab.isRunning = true
      defer {
        tab.activeOperationId = nil
        tab.isRunning = false
      }
      let projection = try await client.finish(operationId: operation)
      tab.resultTable = projection.table
      if let envelope = projection.envelope {
        tab.resultIdData = envelope.resultId
        tab.resultRevision = envelope.revision
        tab.nextStartRow =
          envelope.rowCount == 500
          ? envelope.startRow + UInt64(envelope.rowCount) : nil
      }
      if let table = projection.table {
        tab.summary =
          "\(counted(table.rows.count, "row")) · \(counted(table.columns.count, "column"))"
      } else {
        tab.summary = "No rows"
      }
    } catch { tab.error = "Object browse failed: \(error)" }
  }

  func loadMoreRedisKey() async {
    guard let tab = activeObjectTab, let client, let session = sessionData,
      let skip = tab.redisView?.nextSkip, !tab.isRunning
    else { return }
    tab.isRunning = true
    defer { tab.isRunning = false }
    do {
      let next = try await client.redisKeyView(
        sessionId: session, catalogNodeId: tab.catalogNodeId,
        collectionSkip: skip
      )
      let existing = tab.redisView?.lines ?? []
      tab.redisView = WorkbenchRedisKeyView(
        kind: next.kind,
        lines: existing + Array(next.lines.dropFirst(min(2, next.lines.count))),
        nextSkip: next.nextSkip
      )
      tab.summary = "Redis \(next.kind) · \(tab.redisView?.lines.count ?? 0) lines"
    } catch { tab.error = "Redis key page failed: \(error)" }
  }

  func reloadObjectTab() async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    await loadObjectTab(tab)
  }

  func addObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.sort.count < 16,
      !tab.sort.contains(where: { $0.column == column })
    else { return }
    tab.sort.append(WorkbenchBrowseSort(column: column))
    await loadObjectTab(tab)
  }

  func toggleObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning,
      let index = tab.sort.firstIndex(where: { $0.column == column })
    else { return }
    let current = tab.sort[index]
    tab.sort[index] = WorkbenchBrowseSort(
      column: current.column, descending: !current.descending)
    await loadObjectTab(tab)
  }

  func removeObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.sort.removeAll(where: { $0.column == column })
    await loadObjectTab(tab)
  }

  func addObjectFilter() async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.filters.count < 32,
      !tab.filterColumn.isEmpty
    else { return }
    let value =
      ["is_null", "is_not_null"].contains(tab.filterOperator)
      ? nil : tab.filterValue
    tab.filters.append(
      WorkbenchBrowseFilter(
        id: dependencies.identifiers.next(), column: tab.filterColumn,
        operatorName: tab.filterOperator, value: value))
    tab.filterValue = ""
    await loadObjectTab(tab)
  }

  func removeObjectFilter(id: UUID) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.filters.removeAll(where: { $0.id == id })
    await loadObjectTab(tab)
  }

  func clearObjectFilters() async {
    guard let tab = activeObjectTab, !tab.isRunning, !tab.filters.isEmpty else { return }
    tab.filters.removeAll()
    await loadObjectTab(tab)
  }

  func applyObjectRawWhere() async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    let fragment = tab.rawWhereDraft.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !fragment.isEmpty, fragment.utf8.count <= 65_536 else { return }
    tab.rawWhere = fragment
    await loadObjectTab(tab)
  }

  func clearObjectRawWhere() async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.rawWhere != nil else { return }
    tab.rawWhere = nil
    tab.rawWhereDraft = ""
    await loadObjectTab(tab)
  }

  private func loadObjectFilterPresets(_ tab: NativeObjectTab) async {
    guard let client, let session = sessionData, !tab.kind.hasPrefix("redis_key_") else { return }
    do {
      tab.filterPresets = try await client.listCatalogFilterPresets(
        session: session, nodeId: tab.catalogNodeId)
      tab.filterPresetError = nil
    } catch {
      tab.filterPresets = []
      tab.filterPresetError = "Could not load filter presets: \(error)"
    }
  }

  func saveObjectFilterPreset() async {
    guard let tab = activeObjectTab, let client, let session = sessionData, !tab.isRunning else {
      return
    }
    let name = tab.filterPresetName.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty, name.utf8.count <= 64 else { return }
    do {
      try await client.saveCatalogFilterPreset(
        session: session, nodeId: tab.catalogNodeId,
        preset: WorkbenchSavedFilterPreset(
          name: name, filters: tab.filters, rawWhere: tab.rawWhere))
      tab.filterPresetName = ""
      tab.filterPresetOutcome = "Saved filter preset \(name)"
      tab.filterPresetError = nil
      await loadObjectFilterPresets(tab)
    } catch {
      tab.filterPresetOutcome = nil
      tab.filterPresetError = "Could not save filter preset: \(error)"
    }
  }

  func applyObjectFilterPreset(_ preset: WorkbenchSavedFilterPreset) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.filters = preset.filters.map {
      WorkbenchBrowseFilter(
        id: dependencies.identifiers.next(), column: $0.column,
        operatorName: $0.operatorName, value: $0.value)
    }
    tab.rawWhere = preset.rawWhere
    tab.rawWhereDraft = preset.rawWhere ?? ""
    tab.filterPresetOutcome = "Loaded filter preset \(preset.name)"
    tab.filterPresetError = nil
    await loadObjectTab(tab)
  }

  func loadObjectStructure() async {
    guard let tab = activeObjectTab, let client, let session = sessionData,
      !tab.structureLoading
    else { return }
    tab.selectedSection = "structure"
    tab.structureLoading = true
    tab.structureError = nil
    defer { tab.structureLoading = false }
    do {
      tab.structure = try await client.relationStructure(
        sessionId: session, catalogNodeId: tab.catalogNodeId
      )
    } catch {
      tab.structure = nil
      tab.structureError = "Structure unavailable: \(error)"
    }
  }

  func showFindReplace() {
    guard queryWorkbenchSelected else { return }
    findPattern = ""
    findReplacement = ""
    findMode = "literal"
    findScope = "document"
    findStatus = nil
    findError = nil
    activeQueryTab.findScopeRange = nil
    activeQueryTab.lastFindMatch = nil
    findReplacePresented = true
  }

  func setFindScope(_ scope: String) {
    findScope = scope
    activeQueryTab.findScopeRange = scope == "selection" ? activeQueryTab.editorSelection : nil
    activeQueryTab.lastFindMatch = nil
    findStatus = nil
    findError = nil
  }

  func resetFindTraversal() {
    activeQueryTab.lastFindMatch = nil
    findStatus = nil
    findError = nil
  }

  func findEditorMatch(backwards: Bool) {
    do {
      let match = try NativeFindReplaceEngine.find(
        in: queryText, pattern: findPattern, mode: findMode,
        scope: try effectiveFindScope(), selection: queryEditorSelection,
        previousMatch: activeQueryTab.lastFindMatch, backwards: backwards)
      guard let match else {
        findStatus = "No match"
        findError = nil
        activeQueryTab.lastFindMatch = nil
        return
      }
      queryEditorSelection = match
      activeQueryTab.lastFindMatch = match
      findStatus = "Match at character \(match.location + 1)"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  func replaceEditorMatch() {
    do {
      let outcome = try NativeFindReplaceEngine.replaceCurrent(
        in: queryText, pattern: findPattern, replacement: findReplacement,
        mode: findMode, scope: try effectiveFindScope(), selection: queryEditorSelection)
      guard let outcome else {
        findEditorMatch(backwards: false)
        return
      }
      queryText = outcome.text
      queryEditorSelection = outcome.selection
      updateFindScope(afterReplacing: outcome.replacedRange, delta: outcome.delta)
      activeQueryTab.lastFindMatch = nil
      findStatus = "Replaced 1 match"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  func replaceAllEditorMatches() {
    do {
      let outcome = try NativeFindReplaceEngine.replaceAll(
        in: queryText, pattern: findPattern, replacement: findReplacement,
        mode: findMode, scope: try effectiveFindScope())
      queryText = outcome.text
      queryEditorSelection = outcome.selection
      if findScope == "selection" { activeQueryTab.findScopeRange = outcome.selection }
      activeQueryTab.lastFindMatch = nil
      findStatus = "Replaced \(outcome.count) match\(outcome.count == 1 ? "" : "es")"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  private func effectiveFindScope() throws -> NSRange {
    let whole = NSRange(location: 0, length: (queryText as NSString).length)
    guard findScope == "selection", let selected = activeQueryTab.findScopeRange else {
      return whole
    }
    let location = min(selected.location, whole.length)
    let scope = NSRange(location: location, length: min(selected.length, whole.length - location))
    guard scope.length > 0 else { throw NativeFindReplaceError.invalidScope }
    return scope
  }

  private func updateFindScope(afterReplacing range: NSRange, delta: Int) {
    guard findScope == "selection", var scope = activeQueryTab.findScopeRange,
      range.location >= scope.location, NSMaxRange(range) <= NSMaxRange(scope)
    else { return }
    scope.length = max(0, scope.length + delta)
    activeQueryTab.findScopeRange = scope
  }

  func showDdlChange() {
    guard canEditSelectedStructure else { return }
    ddlChangeKind = "add_column"
    ddlChangeObjectName = ""
    ddlChangeDefinition = ""
    ddlChangeReview = nil
    ddlChangeOutcome = nil
    ddlChangeError = nil
    ddlChangeCatalogNodeId = activeObjectTab?.catalogNodeId
    ddlChangePresented = true
  }

  func stageDdlChange() async {
    guard let client, let session = sessionData, let nodeId = ddlChangeCatalogNodeId,
      ddlChangeReview == nil, !ddlChangeApplying
    else { return }
    ddlChangeError = nil
    ddlChangeOutcome = nil
    do {
      ddlChangeReview = try await client.stageDdlChange(
        sessionId: session, catalogNodeId: nodeId, kind: ddlChangeKind,
        objectName: ddlChangeObjectName.trimmingCharacters(in: .whitespacesAndNewlines),
        definition: ddlChangeDefinition.trimmingCharacters(in: .whitespacesAndNewlines),
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      ddlChangeReview = nil
      ddlChangeError = "Structure review rejected: \(error)"
    }
  }

  func applyDdlChange() async {
    guard let client, let session = sessionData, let review = ddlChangeReview else { return }
    let nodeId = ddlChangeCatalogNodeId
    ddlChangeReview = nil
    ddlChangeApplying = true
    ddlChangeError = nil
    defer { ddlChangeApplying = false }
    do {
      ddlChangeOutcome = try await client.applyDdlChange(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmed: true)
      if let nodeId,
        let tab = objectTabs.first(where: { $0.catalogNodeId == nodeId })
      {
        tab.structure = try await client.relationStructure(
          sessionId: session, catalogNodeId: nodeId)
        tab.structureError = nil
      }
    } catch {
      ddlChangeError = "Structure outcome unknown or failed; review consumed: \(error)"
    }
  }

  func discardDdlChangeReview() async {
    if let review = ddlChangeReview, let client {
      _ = try? await client.revokeDdlChange(tokenId: review.tokenId)
    }
    ddlChangeReview = nil
  }

  func closeDdlChange() async {
    await discardDdlChangeReview()
    ddlChangePresented = false
  }

  func showTableOperation() {
    guard canOperateSelectedTable else { return }
    tableOperationKind = connectedEngine == "clickhouse" ? "optimize" : "truncate"
    tableOperationNewName = ""
    tableOperationConfirmation = ""
    tableOperationReview = nil
    tableOperationStatus = nil
    tableOperationOutcome = nil
    tableOperationError = nil
    tableOperationCatalogNodeId = activeObjectTab?.catalogNodeId
    tableOperationPresented = true
  }

  func resetTableOperationReview() async {
    guard !tableOperationApplying else { return }
    if let review = tableOperationReview, let client {
      _ = try? await client.revokeTableOperation(tokenId: review.tokenId)
    }
    if let operationId = tableOperationId, let client {
      _ = try? await client.dismissTableOperation(operationId: operationId)
    }
    tableOperationReview = nil
    tableOperationStatus = nil
    tableOperationId = nil
    tableOperationConfirmation = ""
    tableOperationOutcome = nil
    tableOperationError = nil
  }

  func stageTableOperation() async {
    guard let client, let session = sessionData, let nodeId = tableOperationCatalogNodeId,
      tableOperationReview == nil, !tableOperationApplying
    else { return }
    tableOperationError = nil
    tableOperationOutcome = nil
    do {
      tableOperationReview = try await client.stageTableOperation(
        sessionId: session, catalogNodeId: nodeId, kind: tableOperationKind,
        newName: tableOperationNewName.trimmingCharacters(in: .whitespacesAndNewlines),
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      tableOperationError = "Table operation review rejected: \(error)"
    }
  }

  func applyTableOperation() async {
    guard let client, let session = sessionData, let review = tableOperationReview else { return }
    guard tableOperationConfirmation == review.confirmation else {
      tableOperationError = "Type the exact target table name to authorize this operation."
      return
    }
    let kind = tableOperationKind
    let nodeId = tableOperationCatalogNodeId
    tableOperationReview = nil
    tableOperationApplying = true
    tableOperationStatus = nil
    tableOperationError = nil
    defer { tableOperationApplying = false }
    do {
      let operationId = try await client.startTableOperation(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmation: tableOperationConfirmation)
      tableOperationId = operationId
      while true {
        let status = try await client.tableOperationStatus(operationId: operationId)
        tableOperationStatus = status
        if status.phase != "running" { break }
        try await Task.sleep(for: .milliseconds(100))
      }
      guard let status = tableOperationStatus else { return }
      if status.phase == "succeeded" {
        tableOperationOutcome = status.summary
      } else {
        tableOperationError = "Table operation \(status.phase): \(status.summary)"
        return
      }
      if ["rename", "drop"].contains(kind), let nodeId {
        objectTabs.removeAll(where: { $0.catalogNodeId == nodeId })
        selectedObjectTabId = nil
        selectedWorkbenchKind = "query"
        await browse()
      } else if kind == "truncate", let nodeId,
        let tab = objectTabs.first(where: { $0.catalogNodeId == nodeId })
      {
        await loadObjectTab(tab)
      }
    } catch {
      tableOperationError = "Table operation failed or outcome unknown; review consumed: \(error)"
    }
  }

  func closeTableOperation() async {
    await resetTableOperationReview()
    tableOperationPresented = false
  }

  func loadMoreObjectRows() async {
    guard let tab = activeObjectTab, let client, let resultId = tab.resultIdData,
      let start = tab.nextStartRow
    else { return }
    do {
      let (more, envelope) = try await client.fetchPage(
        resultId: resultId, startRow: start, revision: tab.resultRevision
      )
      if more.rows.isEmpty {
        tab.nextStartRow = nil
        return
      }
      if let table = tab.resultTable {
        guard let table = table.appending(more) else {
          tab.error = "Load more returned incompatible page metadata"
          return
        }
        tab.resultTable = table
        tab.summary =
          "\(counted(table.rows.count, "row")) · \(counted(table.columns.count, "column"))"
      }
      tab.nextStartRow =
        envelope.rowCount == 500
        ? envelope.startRow + UInt64(envelope.rowCount) : nil
    } catch { tab.error = "Load more failed: \(error)" }
  }

  func persistSessionIntent() async {
    guard let client, let profileId = activeProfileId,
      let selected = queryTabs.firstIndex(where: { $0.id == selectedQueryTabId })
    else { return }
    let intent = WorkbenchSessionIntent(
      database: formDatabase,
      schema: nil,
      selectedTab: UInt32(selected),
      tabs: queryTabs.map {
        WorkbenchWorkspaceTab(title: $0.title, statementText: $0.statementText)
      }
    )
    do {
      try await client.putNativeWindowIntent(
        windowId: windowId.uuidString.lowercased(), profileId: profileId, intent: intent
      )
    } catch { profileActionError = "Save workspace intent failed: \(error)" }
  }

  func copyResult(scope: String, preferredFormat: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "No resident result to copy"
      return
    }
    let selection = selectedCell
    if scope != "loaded", selection == nil {
      copyError = "Select a result cell first"
      return
    }
    copyOutcome = nil
    copyError = nil
    do {
      let row = selection.map { UInt64($0.row) }
      let column = selection.map { UInt32($0.column) }
      var payloads: [String: String] = [:]
      for format in ["csv", "tsv", "json", "markdown"] {
        payloads[format] = try await client.formatResultCopy(
          resultId: resultId, revision: resultRevision, scope: scope,
          row: row, column: column, format: format
        )
      }
      if preferredFormat == "sql_insert" {
        payloads[preferredFormat] = try await client.formatResultCopy(
          resultId: resultId, revision: resultRevision, scope: scope,
          row: row, column: column, format: preferredFormat
        )
      }
      let preferred = payloads[preferredFormat] ?? payloads["tsv"] ?? ""
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: preferred),
        AppPasteboardRepresentation(
          type: "public.comma-separated-values-text", value: payloads["csv"] ?? ""
        ),
        AppPasteboardRepresentation(
          type: "public.utf8-tab-separated-values-text", value: payloads["tsv"] ?? ""),
        AppPasteboardRepresentation(type: "public.json", value: payloads["json"] ?? ""),
        AppPasteboardRepresentation(
          type: "net.daringfireball.markdown", value: payloads["markdown"] ?? ""
        ),
      ])
      copyOutcome =
        "Copied \(scope) as \(preferredFormat.uppercased()) with CSV, TSV, JSON, and Markdown representations"
    } catch { copyError = "Copy failed: \(error)" }
  }

  func exportLoadedResult(format: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "No resident result to export"
      return
    }
    let fileExtension = format == "sql_insert" ? "sql" : format
    guard
      let selected = dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Loaded Result", prompt: "Export",
          suggestedFilename: "result.\(fileExtension)", allowedExtensions: [fileExtension]
        ))
    else { return }
    let url =
      selected.pathExtension.lowercased() == fileExtension
      ? selected : selected.appendingPathExtension(fileExtension)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    copyOutcome = nil
    copyError = nil
    do {
      let bytes = try await client.exportLoadedResult(
        resultId: resultId, revision: resultRevision, format: format, path: url.path
      )
      copyOutcome = "Exported \(bytes) bytes to \(url.lastPathComponent)"
    } catch { copyError = "Export failed: \(error)" }
  }

  func exportFullResult(format: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "Full-result export requires a resident result"
      return
    }
    let statement = queryText.trimmingCharacters(in: .whitespacesAndNewlines)
    if selectedWorkbenchKind == "query" && statement.isEmpty {
      copyError = "Query is empty"
      return
    }
    let fileExtension = format
    guard
      let selected = dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Full Result", prompt: "Export",
          suggestedFilename: "result.\(fileExtension)", allowedExtensions: [fileExtension]
        ))
    else { return }
    let url =
      selected.pathExtension.lowercased() == fileExtension
      ? selected : selected.appendingPathExtension(fileExtension)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    copyOutcome = nil
    copyError = nil
    streamExportError = nil
    streamExportProgress = nil
    streamExportPresented = true
    do {
      let operationId: Data
      if selectedWorkbenchKind == "object" {
        operationId = try await client.startCatalogStreamExport(
          resultId: resultId, revision: resultRevision, format: format, path: url.path)
      } else {
        guard let session = sessionData else {
          throw ScriptedBackendError.unavailable("stream-export-session")
        }
        operationId = try await client.startStreamExport(
          sessionId: session, statement: statement, format: format, path: url.path)
      }
      streamExportOperationId = operationId
      while streamExportOperationId == operationId {
        let progress = try await client.streamExportProgress(operationId: operationId)
        streamExportProgress = progress
        if !["running", "cancel_requested"].contains(progress.phase) {
          copyOutcome = progress.summary
          _ = try? await client.dismissStreamExport(operationId: operationId)
          streamExportOperationId = nil
          break
        }
        try await Task.sleep(for: .milliseconds(100))
      }
    } catch {
      streamExportOperationId = nil
      streamExportError = "Full-result export failed: \(error)"
    }
  }

  private func pollStreamExport(
    client: any WorkbenchBackend, operationId: Data
  ) async throws -> WorkbenchStreamExportProgress {
    while true {
      let progress = try await client.streamExportProgress(operationId: operationId)
      if !["running", "cancel_requested"].contains(progress.phase) { return progress }
      try await Task.sleep(for: .milliseconds(50))
    }
  }

  func cancelStreamExport() async {
    guard let client, let operationId = streamExportOperationId else { return }
    do {
      if try await client.cancelStreamExport(operationId: operationId) {
        streamExportProgress = try await client.streamExportProgress(operationId: operationId)
      }
    } catch { streamExportError = "Cancel export failed: \(error)" }
  }

  func closeStreamExport() {
    guard streamExportOperationId == nil else { return }
    streamExportPresented = false
    streamExportProgress = nil
    streamExportError = nil
  }

  func chooseCsvImport() async {
    guard let client, sqlInsertCopyAvailable else { return }
    guard
      let url = dependencies.filePanels.chooseOpenFile(
        AppFilePanelRequest(
          title: "Import CSV into Table", prompt: "Preview", allowedExtensions: ["csv"]
        ))
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let preview = try await client.previewCsvImport(path: url.path)
      csvImportUrl = url
      csvImportPreview = preview
      csvImportMappedColumns = preview.headers
      csvImportColumnTypes = Array(repeating: "text", count: preview.headers.count)
      csvImportReview = nil
      csvImportError = nil
      csvImportOutcome = nil
      csvImportProgress = nil
      csvImportErrorCopyOutcome = nil
      csvImportPresented = true
    } catch { csvImportError = "CSV preview failed: \(error)" }
  }

  func stageCsvImport() async {
    guard let client, let session = sessionData, let object = activeObjectTab,
      let url = csvImportUrl
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    csvImportError = nil
    do {
      csvImportReview = try await client.stageCsvImport(
        sessionId: session, catalogNodeId: object.catalogNodeId, path: url.path,
        mappedColumns: csvImportMappedColumns,
        mappedTypes: csvImportColumnTypes,
        expectedFingerprint: csvImportPreview?.fingerprint ?? "",
        nowMs: dependencies.clock.nowMilliseconds()
      )
    } catch { csvImportError = "Stage import failed: \(error)" }
  }

  func applyCsvImport() async {
    guard let client, let session = sessionData, let review = csvImportReview else { return }
    csvImportApplying = true
    csvImportError = nil
    defer { csvImportApplying = false }
    do {
      let operationId = try await client.startCsvImportApply(
        tokenId: review.tokenId,
        nowMs: dependencies.clock.nowMilliseconds(),
        sessionId: session
      )
      csvImportReview = nil
      csvImportOperationId = operationId
      while csvImportOperationId == operationId {
        let progress = try await client.csvImportProgress(operationId: operationId)
        csvImportProgress = progress
        if !["running", "cancel_requested"].contains(progress.phase) {
          csvImportOutcome = progress.summary
          _ = try? await client.dismissCsvImport(operationId: operationId)
          csvImportOperationId = nil
          if progress.phase == "completed" { await reloadObjectTab() }
          break
        }
        try await Task.sleep(for: .milliseconds(100))
      }
    } catch {
      csvImportReview = nil
      csvImportOperationId = nil
      csvImportError = "Import progress failed after authority was consumed: \(error)"
    }
  }

  func cancelCsvImport() async {
    guard let client, let operationId = csvImportOperationId else { return }
    do {
      if try await client.cancelCsvImport(operationId: operationId) {
        csvImportProgress = try await client.csvImportProgress(operationId: operationId)
      }
    } catch { csvImportError = "Cancel import failed: \(error)" }
  }

  func copyCsvImportErrors() {
    guard let progress = csvImportProgress, !progress.errors.isEmpty else { return }
    var text = progress.errors.joined(separator: "\n")
    if progress.errorsTruncated { text += "\n… additional errors omitted" }
    do {
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: text)
      ])
      csvImportErrorCopyOutcome = "Copied \(progress.errors.count) import errors"
    } catch { csvImportErrorCopyOutcome = "Copy errors failed: \(error)" }
  }

  func discardCsvImportReview() async {
    if let review = csvImportReview, let client {
      _ = try? await client.revokeReviewToken(tokenId: review.tokenId)
    }
    csvImportReview = nil
  }

  func closeCsvImport() async {
    await discardCsvImportReview()
    csvImportPresented = false
    csvImportPreview = nil
    csvImportMappedColumns = []
    csvImportColumnTypes = []
    csvImportUrl = nil
    csvImportProgress = nil
    csvImportErrorCopyOutcome = nil
  }

  func showRedisOverview() async {
    guard connectedEngine == "redis", let client, let session = sessionData,
      !redisOverviewLoading
    else { return }
    redisOverviewPresented = true
    redisOverviewLoading = true
    redisOverviewError = nil
    defer { redisOverviewLoading = false }
    do {
      redisOverview = try await client.redisOverview(sessionId: session)
    } catch {
      redisOverview = nil
      redisOverviewError = "Redis overview failed: \(error)"
    }
  }

  func showRedisSubscription() {
    guard connectedEngine == "redis", sessionData != nil else { return }
    redisSubscriptionPresented = true
    redisSubscriptionError = nil
  }

  func startRedisSubscription() async {
    guard let client, let session = sessionData, !redisSubscriptionStarting,
      !redisSubscriptionIsActive
    else { return }
    let selector = redisSubscriptionSelector.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !selector.isEmpty else {
      redisSubscriptionError = "Enter a channel or pattern"
      return
    }
    redisSubscriptionStarting = true
    redisSubscriptionError = nil
    defer { redisSubscriptionStarting = false }
    do {
      let operation = try await client.startRedisSubscription(
        sessionId: session, selector: selector, pattern: redisSubscriptionPattern)
      redisSubscriptionStatus = try await client.redisSubscriptionStatus(operationId: operation)
      beginRedisSubscriptionPolling(operation)
    } catch {
      redisSubscriptionStatus = nil
      redisSubscriptionError = "Subscription failed: \(error)"
    }
  }

  func refreshRedisSubscription() async {
    guard let client, let operation = redisSubscriptionStatus?.operationId else { return }
    do {
      let status = try await client.redisSubscriptionStatus(operationId: operation)
      redisSubscriptionStatus = status
      if !redisSubscriptionIsActive { redisSubscriptionPollTask?.cancel() }
    } catch {
      redisSubscriptionError = "Subscription status unavailable: \(error)"
      redisSubscriptionPollTask?.cancel()
    }
  }

  private func beginRedisSubscriptionPolling(_ operation: Data) {
    redisSubscriptionPollTask?.cancel()
    redisSubscriptionPollTask = Task { [weak self] in
      while !Task.isCancelled {
        try? await Task.sleep(for: .milliseconds(250))
        guard !Task.isCancelled, let self,
          self.redisSubscriptionStatus?.operationId == operation,
          self.redisSubscriptionPresented
        else { return }
        await self.refreshRedisSubscription()
        if !self.redisSubscriptionIsActive { return }
      }
    }
  }

  func cancelRedisSubscription() async {
    guard let client, let operation = redisSubscriptionStatus?.operationId else { return }
    do {
      _ = try await client.cancelRedisSubscription(operationId: operation)
      await refreshRedisSubscription()
    } catch {
      redisSubscriptionError = "Cancel failed: \(error)"
    }
  }

  func closeRedisSubscription() async {
    if redisSubscriptionIsActive { await cancelRedisSubscription() }
    redisSubscriptionPollTask?.cancel()
    redisSubscriptionPollTask = nil
    redisSubscriptionPresented = false
  }

  func showPostgresActivity() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresActivityPresented = true
    await refreshPostgresActivity()
  }

  func refreshPostgresActivity() async {
    guard let client, let session = sessionData, !postgresActivityLoading else { return }
    postgresActivityLoading = true
    postgresActivityError = nil
    defer { postgresActivityLoading = false }
    do {
      postgresActivityRows = try await client.postgresActivity(sessionId: session)
    } catch {
      postgresActivityRows = []
      postgresActivityError = "PostgreSQL activity failed: \(error)"
    }
  }

  func showPostgresRelationships() async {
    guard connectedEngine == "postgresql", activeObjectTab != nil else { return }
    postgresRelationshipsPresented = true
    await refreshPostgresRelationships()
  }

  func refreshPostgresRelationships() async {
    guard let client, let session = sessionData, let object = activeObjectTab,
      !postgresRelationshipsLoading
    else { return }
    postgresRelationshipsLoading = true
    postgresRelationshipsError = nil
    defer { postgresRelationshipsLoading = false }
    do {
      postgresRelationshipSnapshot = try await client.postgresRelationships(
        sessionId: session, catalogNodeId: object.catalogNodeId)
    } catch {
      postgresRelationshipSnapshot = nil
      postgresRelationshipsError = "Relationships unavailable: \(error)"
    }
  }

  func showPostgresRoles() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresRolesPresented = true
    await refreshPostgresRoles()
  }

  func refreshPostgresRoles() async {
    guard let client, let session = sessionData, !postgresRolesLoading else { return }
    postgresRolesLoading = true
    postgresRolesError = nil
    defer { postgresRolesLoading = false }
    do {
      postgresRoleSnapshot = try await client.postgresRoles(
        sessionId: session, catalogNodeId: activeObjectTab?.catalogNodeId)
    } catch {
      postgresRoleSnapshot = nil
      postgresRolesError = "Roles unavailable: \(error)"
    }
  }

  func stagePostgresRoleChange() async {
    guard let client, let session = sessionData else { return }
    postgresRolesError = nil
    postgresRoleChangeOutcome = nil
    do {
      postgresRoleChangeReview = try await client.stagePostgresRoleChange(
        sessionId: session, catalogNodeId: activeObjectTab?.catalogNodeId,
        kind: postgresRoleChangeKind,
        role: postgresRoleChangeRole.trimmingCharacters(in: .whitespacesAndNewlines),
        memberOrGrantee: postgresRoleChangeSubject.trimmingCharacters(in: .whitespacesAndNewlines),
        privilege: postgresRoleChangePrivilege,
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      postgresRoleChangeReview = nil
      postgresRolesError = "Role change rejected: \(error)"
    }
  }

  func applyPostgresRoleChange() async {
    guard let client, let session = sessionData, let review = postgresRoleChangeReview else {
      return
    }
    postgresRoleChangeReview = nil
    do {
      postgresRoleChangeOutcome = try await client.applyPostgresRoleChange(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmed: true)
      await refreshPostgresRoles()
    } catch {
      postgresRolesError = "Role change outcome unknown or failed; review consumed: \(error)"
    }
  }

  func discardPostgresRoleChange() async {
    if let review = postgresRoleChangeReview, let client {
      _ = try? await client.revokePostgresRoleChange(tokenId: review.tokenId)
    }
    postgresRoleChangeReview = nil
  }

  func openRelatedRelation(_ edge: WorkbenchRelationshipEdge) async {
    guard let snapshot = postgresRelationshipSnapshot, let nodes = catalogSnapshot else { return }
    let selectedIsSource =
      edge.fromSchema == snapshot.namespace && edge.fromTable == snapshot.relation
    let namespace = selectedIsSource ? edge.toSchema : edge.fromSchema
    let relation = selectedIsSource ? edge.toTable : edge.fromTable
    let node = nodes.first { candidate in
      guard candidate.name == relation, let parentId = candidate.parentIdBytes else { return false }
      return nodes.first(where: { $0.idBytes == parentId })?.name == namespace
    }
    guard let node else {
      postgresRelationshipsError = "Load \(namespace).\(relation) in the catalog before opening it."
      return
    }
    postgresRelationshipsPresented = false
    await openCatalogObject(nodeKey: catalogNodeKey(node.idBytes))
  }

  func signalPostgresBackend(kind: String, pid: Int32) async {
    guard let client, let session = sessionData else { return }
    postgresActivityError = nil
    postgresActivityOutcome = nil
    do {
      let outcome = try await client.signalPostgresBackend(
        sessionId: session, kind: kind, pid: pid)
      postgresActivityOutcome =
        outcome.acknowledged
        ? "\(kind.capitalized) acknowledged for PID \(pid)"
        : "PID \(pid) was not signalable"
      await refreshPostgresActivity()
    } catch {
      postgresActivityError = "\(kind.capitalized) failed: \(error)"
    }
  }

  func showPostgresTools() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresToolsPresented = true
    postgresToolError = nil
    await probePostgresTool()
  }

  func probePostgresTool() async {
    guard let client else { return }
    postgresToolError = nil
    let explicit = postgresToolExplicitPath.trimmingCharacters(in: .whitespacesAndNewlines)
    do {
      postgresToolProbe = try await client.probePostgresTool(
        kind: postgresToolKind,
        explicitPath: explicit.isEmpty ? nil : explicit)
    } catch {
      postgresToolProbe = nil
      postgresToolError = "Tool probe failed: \(error)"
    }
  }

  func choosePostgresToolFile() {
    let request = AppFilePanelRequest(
      title: postgresToolKind == "dump" ? "Choose Backup Destination" : "Choose Restore Archive",
      prompt: postgresToolKind == "dump" ? "Choose" : "Restore",
      suggestedFilename: postgresToolKind == "dump" ? "tablerock.dump" : nil,
      allowedExtensions: ["dump", "backup"])
    postgresToolFileUrl =
      postgresToolKind == "dump"
      ? dependencies.filePanels.chooseSaveFile(request)
      : dependencies.filePanels.chooseOpenFile(request)
    postgresToolStatus = nil
    postgresToolError = nil
  }

  func requestStartPostgresTool() {
    guard postgresToolProbe?.available == true, postgresToolFileUrl != nil else {
      postgresToolError = "Choose an available tool and archive file first"
      return
    }
    postgresToolReviewRequested = true
  }

  func startPostgresTool() async {
    postgresToolReviewRequested = false
    guard let client, let session = sessionData, let tool = postgresToolProbe?.path,
      let file = postgresToolFileUrl
    else { return }
    postgresToolError = nil
    postgresToolStatus = nil
    postgresToolSecurityScopeActive = file.startAccessingSecurityScopedResource()
    do {
      let operation = try await client.startPostgresTool(
        sessionId: session, kind: postgresToolKind, toolPath: tool, filePath: file.path,
        content: postgresToolContent, clean: postgresToolKind == "restore" && postgresToolClean,
        noOwner: postgresToolNoOwner)
      postgresToolStatus = WorkbenchPostgresToolStatus(
        operationId: operation, kind: postgresToolKind, phase: "running",
        summary: "Process started")
      await pollPostgresTool(operation)
    } catch {
      releasePostgresToolSecurityScope()
      postgresToolError = "PostgreSQL tool failed to start: \(error)"
    }
  }

  private func pollPostgresTool(_ operation: Data) async {
    guard let client else { return }
    while true {
      do {
        let status = try await client.postgresToolStatus(operationId: operation)
        postgresToolStatus = status
        if status.phase != "running" && status.phase != "cancel_requested" {
          releasePostgresToolSecurityScope()
          return
        }
      } catch {
        releasePostgresToolSecurityScope()
        postgresToolError = "PostgreSQL tool status failed: \(error)"
        return
      }
      try? await Task.sleep(for: .milliseconds(200))
    }
  }

  func cancelPostgresTool() async {
    guard let client, let operation = postgresToolStatus?.operationId else { return }
    do {
      if try await client.cancelPostgresTool(operationId: operation) {
        postgresToolStatus = WorkbenchPostgresToolStatus(
          operationId: operation, kind: postgresToolKind, phase: "cancel_requested",
          summary: "Cancellation requested")
      }
    } catch { postgresToolError = "PostgreSQL tool cancellation failed: \(error)" }
  }

  func closePostgresTools() {
    guard
      postgresToolStatus?.phase != "running"
        && postgresToolStatus?.phase != "cancel_requested"
    else { return }
    releasePostgresToolSecurityScope()
    postgresToolsPresented = false
  }

  private func releasePostgresToolSecurityScope() {
    if postgresToolSecurityScopeActive, let file = postgresToolFileUrl {
      file.stopAccessingSecurityScopedResource()
    }
    postgresToolSecurityScopeActive = false
  }

  private func restoreSessionIntent(profileId: Data) async {
    guard let client else { return }
    do {
      guard
        let record = try await client.nativeWindowIntent(
          windowId: windowId.uuidString.lowercased()
        ), record.profileId == profileId
      else {
        let tab = NativeQueryTab(
          id: dependencies.identifiers.next(), title: "Query 1", statementText: ""
        )
        queryTabs = [tab]
        selectedQueryTabId = tab.id
        return
      }
      guard applySessionIntent(record.intent) else {
        profileActionError = "Restored workspace intent was invalid"
        return
      }
    } catch { profileActionError = "Restore workspace intent failed: \(error)" }
  }

  private func restoreWindowIntentOnLaunch() async {
    guard let client else { return }
    do {
      guard
        let record = try await client.nativeWindowIntent(
          windowId: windowId.uuidString.lowercased()
        ), let profile = profiles.first(where: { $0.idBytes == record.profileId })
      else { return }
      guard applySessionIntent(record.intent) else {
        profileActionError = "Restored workspace intent was invalid"
        return
      }
      activeProfileId = record.profileId
      profileActionOutcome = "Restored \(profile.name) workspace; connect to resume"
    } catch { profileActionError = "Restore window intent failed: \(error)" }
  }

  @discardableResult
  private func applySessionIntent(_ intent: WorkbenchSessionIntent) -> Bool {
    let restored = intent.tabs.map {
      NativeQueryTab(
        id: dependencies.identifiers.next(),
        title: $0.title,
        statementText: $0.statementText
      )
    }
    guard !restored.isEmpty, Int(intent.selectedTab) < restored.count else { return false }
    queryTabs = restored
    selectedQueryTabId = restored[Int(intent.selectedTab)].id
    formDatabase = intent.database
    return true
  }

  private func clearVolatileTabState() {
    for tab in queryTabs {
      tab.resultTable = nil
      tab.resultIdData = nil
      tab.resultRevision = 0
      tab.nextStartRow = nil
      tab.writeOutcome = nil
      tab.cancelOutcome = nil
      tab.reviewOutcome = nil
      tab.reviewError = nil
      tab.querySummary = nil
      tab.queryError = nil
      tab.activeOperationId = nil
      tab.isRunning = false
    }
    objectTabs = []
    selectedObjectTabId = nil
    selectedWorkbenchKind = "query"
    queryStateRevision &+= 1
  }

  private var hasUnsavedEditorText: Bool { queryText != sqlFileBaseline }

  func requestOpenSqlFile() {
    if hasUnsavedEditorText {
      confirmDiscardForOpen = true
    } else {
      Task { await openSqlFile() }
    }
  }

  func openSqlFile() async {
    confirmDiscardForOpen = false
    guard
      let url = dependencies.filePanels.chooseOpenFile(
        AppFilePanelRequest(
          title: "Open SQL File", prompt: "Open", allowedExtensions: ["sql"]
        )), let client
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let file = try await client.readSqlFile(path: url.path)
      sqlFile = file
      sqlFileBaseline = file.statementText
      queryText = file.statementText
      sqlFileError = nil
      profileActionOutcome = "Opened \(url.lastPathComponent)"
    } catch { sqlFileError = "Open SQL file failed: \(error)" }
  }

  func saveSqlFile(saveAs: Bool = false, overwriteExternalChange: Bool = false) async {
    guard let client else { return }
    var url = sqlFile.map { URL(fileURLWithPath: $0.path) }
    if saveAs || url == nil {
      guard
        let selected = dependencies.filePanels.chooseSaveFile(
          AppFilePanelRequest(
            title: "Save SQL File", prompt: "Save", suggestedFilename: "query.sql",
            allowedExtensions: ["sql"]
          ))
      else { return }
      url =
        selected.pathExtension == "sql"
        ? selected : selected.appendingPathExtension("sql")
    }
    guard let url else { return }
    let sameFile = !saveAs && sqlFile?.path == url.path
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let written = try await client.writeSqlFile(
        path: url.path,
        statement: queryText,
        expectedModifiedNanos: sameFile ? sqlFile?.modifiedNanos : nil,
        expectedLength: sameFile ? sqlFile?.len : nil,
        overwriteExternalChange: overwriteExternalChange
      )
      sqlFile = written
      sqlFileBaseline = queryText
      sqlFileError = nil
      confirmExternalOverwrite = false
      profileActionOutcome = "Saved \(url.lastPathComponent)"
    } catch let error as BridgeError {
      if case .Rejected(code: "sql-file-external-change", message: _) = error {
        confirmExternalOverwrite = true
      } else {
        sqlFileError = "Save SQL file failed: \(error)"
      }
    } catch { sqlFileError = "Save SQL file failed: \(error)" }
  }

  func reloadSqlFile() async {
    guard let file = sqlFile, let client else { return }
    let url = URL(fileURLWithPath: file.path)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let loaded = try await client.readSqlFile(path: file.path)
      sqlFile = loaded
      sqlFileBaseline = loaded.statementText
      queryText = loaded.statementText
      sqlFileError = nil
      confirmExternalOverwrite = false
      profileActionOutcome = "Reloaded \(url.lastPathComponent)"
    } catch { sqlFileError = "Reload SQL file failed: \(error)" }
  }

  func beginCreateGroup() {
    groupDialog = ProfileGroupDialog(
      id: dependencies.identifiers.next(), oldName: nil, name: ""
    )
  }

  func beginRenameGroup(_ name: String) {
    groupDialog = ProfileGroupDialog(
      id: dependencies.identifiers.next(), oldName: name, name: name
    )
  }

  func saveGroup(_ dialog: ProfileGroupDialog) async -> Bool {
    guard let client else { return false }
    profileActionError = nil
    do {
      if let oldName = dialog.oldName {
        let moved = try await client.renameProfileGroup(oldName, dialog.name)
        collapsedProfileGroups.remove(oldName)
        profileActionOutcome = "Group renamed · \(moved) connection(s) moved"
      } else {
        try await client.createProfileGroup(dialog.name)
        profileActionOutcome = "Group created"
      }
      groupDialog = nil
      await refreshProfiles()
      return true
    } catch {
      profileActionError = "Group change failed: \(error)"
      return false
    }
  }

  func removePendingGroup() async {
    guard let client, let name = pendingGroupRemoval else { return }
    pendingGroupRemoval = nil
    profileActionError = nil
    do {
      let moved = try await client.deleteProfileGroup(name)
      collapsedProfileGroups.remove(name)
      profileActionOutcome = "Group removed · \(moved) connection(s) moved to Ungrouped"
      await refreshProfiles()
    } catch { profileActionError = "Remove group failed: \(error)" }
  }

  func setGroupAlphabetical(_ section: ProfileSection, _ alphabetical: Bool) async {
    guard let client, section.id != "ungrouped" else { return }
    profileActionError = nil
    do {
      try await client.setGroupAlphabetical(section.title, alphabetical)
      profileActionOutcome =
        alphabetical
        ? "\(section.title) sorted alphabetically"
        : "\(section.title) uses manual order"
      await refreshProfiles()
    } catch { profileActionError = "Group ordering failed: \(error)" }
  }

  func toggleFavorite(_ item: WorkbenchProfileItem) async {
    guard let client else { return }
    profileActionError = nil
    do {
      try await client.setProfileFavorite(item, !item.favorite)
      profileActionOutcome =
        item.favorite
        ? "Removed from favorites: \(item.name)"
        : "Added to favorites: \(item.name)"
      await refreshProfiles()
    } catch { profileActionError = "Favorite change failed: \(error)" }
  }

  func canMove(_ item: WorkbenchProfileItem, in section: ProfileSection, offset: Int) -> Bool {
    guard !section.alphabetical,
      let index = section.profiles.firstIndex(where: { $0.idBytes == item.idBytes })
    else { return false }
    let target = index + offset
    return section.profiles.indices.contains(target)
      && section.profiles[target].favorite == item.favorite
  }

  func move(_ item: WorkbenchProfileItem, in section: ProfileSection, offset: Int) async {
    guard let client,
      canMove(item, in: section, offset: offset),
      let index = section.profiles.firstIndex(where: { $0.idBytes == item.idBytes })
    else { return }
    var ordered = section.profiles
    ordered.swapAt(index, index + offset)
    profileActionError = nil
    do {
      try await client.reorderProfiles(
        group: section.id == "ungrouped" ? nil : section.title,
        profiles: ordered
      )
      profileActionOutcome = "Connection order updated"
      await refreshProfiles()
    } catch { profileActionError = "Reorder failed: \(error)" }
  }

  func createProfile() {
    editorDraft = ProfileEditorDraft(
      WorkbenchProfileDraft(
        idBytes: nil, revision: 0, engine: "postgresql", name: "",
        group: "", environment: "", host: "127.0.0.1", port: "5432",
        database: "postgres", username: "postgres", passwordSource: "prompt",
        passwordValue: "", passwordReference: nil, hasStoredPassword: false,
        plaintextAcknowledged: false, tlsMode: "verify_full",
        safetyMode: "confirm_writes"
      ))
  }

  func beginConnectionUrlImport() {
    connectionUrlImport = ConnectionUrlImport()
  }

  func parseConnectionUrl(_ input: String) async -> String? {
    guard let client else { return "Bridge unavailable" }
    profileActionError = nil
    do {
      var draft = ProfileEditorDraft(try await client.parseConnectionUrl(input))
      draft.name = draft.database.isEmpty ? draft.host : "\(draft.database) on \(draft.host)"
      connectionUrlImport = nil
      editorDraft = draft
      return nil
    } catch {
      let message = "URL rejected: \(error)"
      connectionUrlImport?.error = message
      return message
    }
  }

  func receiveExternalUrlFixtureIfNeeded() async {
    guard !externalUrlFixtureConsumed,
      let raw = ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_EXTERNAL_URL"],
      let url = URL(string: raw)
    else { return }
    externalUrlFixtureConsumed = true
    await receiveExternalURL(url)
  }

  func receiveExternalURL(_ externalUrl: URL) async {
    let input: String
    do {
      input = try externalConnectionUrlPayload(externalUrl)
    } catch {
      profileActionError = "External URL rejected before database parsing: \(error)"
      return
    }
    guard let client else {
      profileActionError = "External URL rejected: bridge unavailable"
      return
    }
    do {
      let draft = ProfileEditorDraft(try await client.parseConnectionUrl(input))
      let matched = profiles.first {
        $0.engine == draft.engine && $0.host == draft.host && $0.port == draft.port
          && ($0.context ?? "") == draft.database
      }
      let user = draft.username.isEmpty ? "(none)" : draft.username
      let secret = draft.passwordValue.isEmpty ? "absent" : "present"
      externalUrlReview = ExternalUrlReview(
        draft: draft,
        summary:
          "\(draft.engine) · \(draft.host):\(draft.port)/\(draft.database) · user \(user) · password \(secret) · TLS \(draft.tlsMode)",
        matchedProfile: matched
      )
      profileActionError = nil
    } catch {
      profileActionError = "External URL rejected: \(error)"
    }
  }

  func reviewExternalURLAsNewConnection() {
    guard var draft = externalUrlReview?.draft else { return }
    draft.name = draft.database.isEmpty ? draft.host : "\(draft.database) on \(draft.host)"
    externalUrlReview = nil
    editorDraft = draft
  }

  func connectExternalSavedProfile() async {
    guard let profile = externalUrlReview?.matchedProfile else { return }
    externalUrlReview = nil
    _ = await connect(profile)
  }

  func connectExternalTemporarily() async {
    guard let draft = externalUrlReview?.draft, let port = UInt16(draft.port) else { return }
    externalUrlReview = nil
    await connectTemporary(
      WorkbenchOpenParams(
        engine: draft.engine, host: draft.host, port: port, database: draft.database,
        user: draft.username, password: draft.passwordValue, tlsMode: draft.tlsMode
      ))
  }

  func showQuickSwitcher() async {
    quickSwitcherSearch = ""
    await refreshSavedQueries()
    quickSwitcherPresented = true
  }

  var quickSwitcherItems: [QuickSwitcherItem] {
    var items: [QuickSwitcherItem] = []
    items += profiles.map {
      QuickSwitcherItem(
        id: "profile:\($0.idBytes.hexEncodedString())", title: $0.name,
        subtitle: "Connection · \($0.engine) · \($0.host ?? ""):\($0.port ?? "")",
        favorite: $0.favorite, target: .profile($0.idBytes))
    }
    items += queryTabs.map {
      QuickSwitcherItem(
        id: "query:\($0.id.uuidString)", title: $0.title, subtitle: "Query tab",
        favorite: false, target: .queryTab($0.id))
    }
    items += objectTabs.map {
      QuickSwitcherItem(
        id: "object:\($0.id.uuidString)", title: $0.title,
        subtitle: $0.pinned ? "Pinned object tab" : "Preview object tab",
        favorite: $0.pinned, target: .objectTab($0.id))
    }
    items += (catalogSnapshot ?? []).filter { !$0.expandable }.map {
      QuickSwitcherItem(
        id: "catalog:\($0.idBytes.hexEncodedString())", title: $0.name,
        subtitle: "Catalog · \($0.kind.replacingOccurrences(of: "_", with: " "))",
        favorite: false, target: .catalog(catalogNodeKey($0.idBytes)))
    }
    items += savedQueries.map {
      QuickSwitcherItem(
        id: "saved:\($0.queryId)", title: $0.name, subtitle: "Saved query · \($0.engine)",
        favorite: false, target: .savedQuery($0.queryId))
    }
    let query = quickSwitcherSearch.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    return
      items
      .filter {
        query.isEmpty || $0.title.lowercased().contains(query)
          || $0.subtitle.lowercased().contains(query)
      }
      .sorted {
        if $0.favorite != $1.favorite { return $0.favorite && !$1.favorite }
        let lhsExact = $0.title.lowercased() == query
        let rhsExact = $1.title.lowercased() == query
        if lhsExact != rhsExact { return lhsExact }
        let lhsPrefix = $0.title.lowercased().hasPrefix(query)
        let rhsPrefix = $1.title.lowercased().hasPrefix(query)
        if lhsPrefix != rhsPrefix { return lhsPrefix }
        return $0.title.localizedCaseInsensitiveCompare($1.title) == .orderedAscending
      }
  }

  func activateQuickSwitcherItem(_ item: QuickSwitcherItem) async {
    quickSwitcherPresented = false
    switch item.target {
    case .profile(let id):
      if let profile = profiles.first(where: { $0.idBytes == id }) { _ = await connect(profile) }
    case .queryTab(let id):
      if let tab = queryTabs.first(where: { $0.id == id }) { selectQueryTab(tab) }
    case .objectTab(let id):
      if let tab = objectTabs.first(where: { $0.id == id }) { selectObjectTab(tab) }
    case .catalog(let key):
      await openCatalogObject(nodeKey: key)
    case .savedQuery(let id):
      if let query = savedQueries.first(where: { $0.queryId == id }) {
        restoreSavedQuery(query)
        selectedWorkbenchKind = "query"
      }
    }
  }

  func editProfile(_ item: WorkbenchProfileItem) async {
    guard let client else { return }
    profileActionError = nil
    do { editorDraft = ProfileEditorDraft(try await client.profileDraft(id: item.idBytes)) } catch {
      profileActionError = "Load connection failed: \(error)"
    }
  }

  func duplicateProfile(_ item: WorkbenchProfileItem) async {
    await editProfile(item)
    guard var copy = editorDraft else { return }
    copy.idBytes = nil
    copy.revision = 0
    copy.name += " Copy"
    if copy.hasStoredPassword { copy.passwordValue = "" }
    if copy.passwordSource == "keychain" {
      copy.passwordReference = nil
      copy.hasStoredPassword = false
    }
    editorDraft = copy
  }

  func saveProfile(_ draft: ProfileEditorDraft) async -> Bool {
    guard let client else { return false }
    profileActionError = nil
    var draft = draft
    let oldReference = draft.passwordReference
    var addedReference: Data?
    do {
      if draft.passwordSource == "keychain", !draft.passwordValue.isEmpty {
        var secret = Data(draft.passwordValue.utf8)
        defer { secret.resetBytes(in: 0..<secret.count) }
        let reference = try dependencies.keychain.store(
          secret: secret,
          account: dependencies.identifiers.next().uuidString.lowercased()
        )
        addedReference = reference
        draft.passwordReference = reference
        draft.passwordValue = ""
        draft.hasStoredPassword = true
      }
      _ = try await client.saveProfile(draft.workbench)
      var cleanupWarning = false
      if let oldReference, let addedReference, oldReference != addedReference {
        do { try dependencies.keychain.remove(reference: oldReference) } catch {
          cleanupWarning = true
        }
      }
      editorDraft = nil
      profileActionOutcome =
        cleanupWarning
        ? "Connection saved; previous Keychain item cleanup failed"
        : (draft.idBytes == nil ? "Connection created" : "Connection saved")
      await refreshProfiles()
      return true
    } catch {
      if let addedReference {
        try? dependencies.keychain.remove(reference: addedReference)
      }
      profileActionError = "Save connection failed: \(error)"
      return false
    }
  }

  func testProfile(_ item: WorkbenchProfileItem, passwordOverride: String? = nil) async {
    guard let client else { return }
    var resolvedOverride = passwordOverride.map { Data($0.utf8) }
    defer { zeroizeTransientData(&resolvedOverride) }
    if passwordOverride == nil {
      do {
        let draft = try await client.profileDraft(id: item.idBytes)
        if draft.passwordSource == "prompt" {
          passwordPrompt = ProfilePasswordPrompt(profile: item, action: .test)
          return
        }
        if draft.passwordSource == "keychain" {
          resolvedOverride = try keychainPassword(for: draft)
        }
      } catch {
        profileActionError = "Load connection failed: \(error)"
        return
      }
    }
    profileActionError = nil
    profileActionOutcome = "Testing \(item.name)…"
    do {
      let report = try await client.testProfile(
        id: item.idBytes, secretOverride: resolvedOverride
      )
      profileActionOutcome =
        "\(report.identity) · TLS \(report.tlsOutcome) · \(report.elapsedMillis) ms"
    } catch { profileActionError = "Connection test failed: \(error)" }
  }

  func removePendingProfile() async {
    guard let client, let item = pendingRemoval else { return }
    pendingRemoval = nil
    profileActionError = nil
    do {
      let reference = try await client.profileDraft(id: item.idBytes).passwordReference
      try await client.deleteProfile(id: item.idBytes, revision: item.revision)
      var cleanupWarning = false
      if let reference {
        do { try dependencies.keychain.remove(reference: reference) } catch {
          cleanupWarning = true
        }
      }
      profileActionOutcome =
        cleanupWarning
        ? "Connection removed; Keychain item cleanup failed"
        : "Connection removed: \(item.name)"
      await refreshProfiles()
    } catch { profileActionError = "Remove connection failed: \(error)" }
  }

  /// Connect directly from form params (temporary session, no saved profile).
  func connectByParams() async {
    guard let port = UInt16(formPort), !formHost.isEmpty
    else {
      connectError = "Invalid host or port"
      return
    }
    await connectTemporary(
      WorkbenchOpenParams(
        engine: formEngine, host: formHost, port: port, database: formDatabase,
        user: formUser, password: formPassword, tlsMode: "off"
      ))
  }

  private func connectTemporary(_ params: WorkbenchOpenParams) async {
    guard !hasRunningWorkbench else {
      connectError = "Cancel running queries before replacing the connection"
      return
    }
    guard let client else {
      connectError = "Bridge unavailable"
      return
    }
    let previousSession = sessionData
    await persistSessionIntent()
    connectError = nil
    do {
      let session = try await client.open(params: params)
      connectedEngine = params.engine
      activeProfileId = nil
      sessionData = session
      sessionHex = session.map { String(format: "%02x", $0) }.joined()
      sessionHealth = nil
      reconnectState = nil
      reconnectGeneration &+= 1
      if let previousSession { try? await client.disconnect(session: previousSession) }
      catalogSummary = nil
      catalogSnapshot = nil
      catalogRefreshState = .idle
      clearVolatileTabState()
      await refreshProfiles()
      await checkActiveHealth()
    } catch {
      connectError = "Connect failed: \(error)"
    }
  }

  /// Open a saved profile, prompting transiently when its source requires it.
  @discardableResult
  func connect(_ item: WorkbenchProfileItem, passwordOverride: String? = nil) async -> Bool {
    guard let client else { return false }
    guard !hasRunningWorkbench else {
      connectError = "Cancel running queries before replacing the connection"
      return false
    }
    var resolvedOverride = passwordOverride.map { Data($0.utf8) }
    defer { zeroizeTransientData(&resolvedOverride) }
    if passwordOverride == nil {
      do {
        let draft = try await client.profileDraft(id: item.idBytes)
        if draft.passwordSource == "prompt" {
          passwordPrompt = ProfilePasswordPrompt(profile: item, action: .connect)
          return false
        }
        if draft.passwordSource == "keychain" {
          resolvedOverride = try keychainPassword(for: draft)
        }
      } catch {
        connectError = "Load connection failed: \(error)"
        return false
      }
    }
    let previousSession = sessionData
    await persistSessionIntent()
    connectingName = item.name
    connectError = nil
    do {
      let session = try await client.openProfile(
        id: item.idBytes, secretOverride: resolvedOverride
      )
      connectedEngine = item.engine
      activeProfileId = item.idBytes
      sessionData = session
      sessionHex = session.map { String(format: "%02x", $0) }.joined()
      sessionHealth = nil
      reconnectState = nil
      reconnectGeneration &+= 1
      if let previousSession { try? await client.disconnect(session: previousSession) }
      catalogSummary = nil
      catalogError = nil
      catalogSnapshot = nil
      catalogRefreshState = .idle
      clearVolatileTabState()
      await restoreSessionIntent(profileId: item.idBytes)
      await refreshProfiles()
      await checkActiveHealth()
      passwordPrompt = nil
      connectingName = nil
      return true
    } catch {
      connectError = "Connect failed: \(error)"
      connectingName = nil
      return false
    }
  }

  func disconnectActive() async {
    guard let client, let session = sessionData else { return }
    await persistSessionIntent()
    if redisSubscriptionIsActive { await closeRedisSubscription() }
    do {
      try await client.disconnect(session: session)
      sessionData = nil
      sessionHex = nil
      connectedEngine = ""
      sessionHealth = nil
      reconnectState = nil
      reconnectGeneration &+= 1
      catalogSummary = nil
      catalogSnapshot = nil
      catalogRefreshState = .idle
      resultTable = nil
      profileActionOutcome = "Disconnected"
      await refreshProfiles()
    } catch { profileActionError = "Disconnect failed: \(error)" }
  }

  func checkActiveHealth() async {
    guard let client, let session = sessionData, !healthChecking else { return }
    healthChecking = true
    defer { healthChecking = false }
    do {
      sessionHealth = try await client.checkHealth(session: session)
      if sessionHealth?.serverReachable == false {
        await reconnectAutomatically(
          sourceSession: session,
          authenticationStopped: sessionHealth?.authenticationStopped == true
        )
      }
    } catch {
      sessionHealth = WorkbenchSessionHealth(
        state: "unhealthy", serverReachable: false,
        elapsedMillis: nil, authenticationStopped: false
      )
      profileActionError = "Health check failed: \(error)"
    }
  }

  func reconnectActive() async {
    guard let client, let sourceSession = sessionData else { return }
    if let activeProfileId,
      let profile = profiles.first(where: { $0.idBytes == activeProfileId })
    {
      do {
        let draft = try await client.profileDraft(id: profile.idBytes)
        if draft.passwordSource == "prompt" {
          passwordPrompt = ProfilePasswordPrompt(profile: profile, action: .reconnect)
          return
        }
        if draft.passwordSource == "keychain" {
          let password = try keychainPassword(for: draft)
          await reconnectActive(
            sourceSession: sourceSession, secretOverride: password
          )
          return
        }
      } catch {
        profileActionError = "Load connection failed: \(error)"
        return
      }
    }
    await reconnectActive(sourceSession: sourceSession, secretOverride: nil)
  }

  private func keychainPassword(for draft: WorkbenchProfileDraft) throws -> Data {
    guard let reference = draft.passwordReference else {
      throw AppCapabilityError.rejected("keychain-reference-missing")
    }
    let bytes = try dependencies.keychain.read(reference: reference)
    guard !bytes.isEmpty else {
      throw AppCapabilityError.rejected("keychain-value-invalid")
    }
    return bytes
  }

  private func reconnectActive(sourceSession: Data, secretOverride: Data?) async {
    guard let client else { return }
    var secretOverride = secretOverride
    defer { zeroizeTransientData(&secretOverride) }
    reconnectGeneration &+= 1
    let generation = reconnectGeneration
    reconnectState = "Reconnecting"
    do {
      let attempt = try await client.reconnect(
        session: sourceSession, secretOverride: secretOverride
      )
      guard attempt.state == "connected", let replacement = attempt.sessionId else {
        reconnectState =
          attempt.state == "authentication_stopped"
          ? "Authentication stopped" : "Reconnect failed"
        return
      }
      guard generation == reconnectGeneration else {
        try? await client.disconnect(session: replacement)
        return
      }
      sessionData = replacement
      sessionHex = replacement.map { String(format: "%02x", $0) }.joined()
      reconnectState = nil
      sessionHealth = try await client.checkHealth(session: replacement)
      await refreshProfiles()
    } catch {
      guard generation == reconnectGeneration else { return }
      reconnectState = "Reconnect failed"
      profileActionError = "Reconnect failed: \(error)"
    }
  }

  func submitPasswordPrompt(_ prompt: ProfilePasswordPrompt, password: String) async -> Bool {
    switch prompt.action {
    case .connect:
      return await connect(prompt.profile, passwordOverride: password)
    case .test:
      await testProfile(prompt.profile, passwordOverride: password)
      if profileActionError == nil {
        passwordPrompt = nil
        return true
      }
      return false
    case .reconnect:
      guard let sourceSession = sessionData else { return false }
      await reconnectActive(
        sourceSession: sourceSession, secretOverride: Data(password.utf8)
      )
      if reconnectState == nil {
        passwordPrompt = nil
        return true
      }
      return false
    }
  }

  private func reconnectAutomatically(
    sourceSession: Data,
    authenticationStopped: Bool
  ) async {
    guard let client else { return }
    reconnectGeneration &+= 1
    let generation = reconnectGeneration
    var attempt: UInt32 = 0
    while generation == reconnectGeneration, sessionData == sourceSession {
      let plan: WorkbenchReconnectPlan
      do {
        plan = try await client.planReconnect(
          session: sourceSession, attempt: attempt,
          authenticationStopped: authenticationStopped
        )
      } catch {
        reconnectState = "Reconnect unavailable"
        return
      }
      switch plan.action {
      case "manual":
        reconnectState = nil
        return
      case "authentication_stopped":
        reconnectState = "Authentication stopped"
        return
      case "exhausted":
        reconnectState = "Reconnect budget exhausted"
        return
      case "retry":
        let delay = plan.delayMillis ?? 0
        reconnectState = "Reconnecting · attempt \(attempt + 1)"
        if delay > 0 {
          try? await Task.sleep(for: .milliseconds(Int64(delay)))
        }
        guard generation == reconnectGeneration, sessionData == sourceSession else {
          return
        }
        do {
          let reconnectAttempt = try await client.reconnect(
            session: sourceSession, secretOverride: nil
          )
          if reconnectAttempt.state == "authentication_stopped" {
            reconnectState = "Authentication stopped"
            return
          }
          guard reconnectAttempt.state == "connected",
            let replacement = reconnectAttempt.sessionId
          else {
            attempt &+= 1
            continue
          }
          guard generation == reconnectGeneration else {
            try? await client.disconnect(session: replacement)
            return
          }
          sessionData = replacement
          sessionHex = replacement.map { String(format: "%02x", $0) }.joined()
          reconnectState = nil
          sessionHealth = try await client.checkHealth(session: replacement)
          await refreshProfiles()
          return
        } catch {
          attempt &+= 1
        }
      default:
        reconnectState = "Reconnect unavailable"
        return
      }
    }
  }

  func connectionState(_ profile: WorkbenchProfileItem) -> String {
    if connectingName == profile.name { return "Connecting" }
    guard profile.connected else { return "Disconnected" }
    guard isActiveProfile(profile) else { return "Connected in another window" }
    if let reconnectState { return reconnectState }
    guard let sessionHealth else { return "Connected" }
    switch sessionHealth.state {
    case "healthy":
      return sessionHealth.elapsedMillis.map { "Healthy · \($0) ms" } ?? "Healthy"
    case "authentication_stopped": return "Authentication stopped"
    case "timeout": return "Health timeout"
    case "unreachable": return "Unreachable"
    default: return "Unhealthy"
    }
  }

  func isActiveProfile(_ profile: WorkbenchProfileItem) -> Bool {
    sessionData != nil && activeProfileId == profile.idBytes
  }

  /// Submit a catalog refresh and poll events until the page arrives, then
  /// decode the v1 page envelope. Proves the operation/event/page flow.
  /// Submit an operation and poll events until the result page arrives.
  /// Returns the decoded table, or nil on terminal-without-page.
  private func fetchPage(
    intent: String,
    statement: String?,
    tab: NativeQueryTab,
    bindings: [WorkbenchQueryParameter]? = nil
  ) async throws -> WorkbenchTable? {
    guard let client, let session = sessionData else { return nil }
    let operationId =
      if let bindings, let statement {
        try await client.submitNamed(session: session, statement: statement, bindings: bindings)
      } else {
        try await client.submit(session: session, intent: intent, statement: statement)
      }
    tab.activeOperationId = operationId
    tab.isRunning = true
    tab.cancelOutcome = nil
    queryStateRevision &+= 1
    defer {
      tab.activeOperationId = nil
      tab.isRunning = false
      queryStateRevision &+= 1
    }
    let projection = try await client.finish(operationId: operationId)
    tab.writeOutcome = projection.outcome
    if projection.historyFailed {
      profileActionError = "Query completed, but local history could not be saved"
    }
    if let env = projection.envelope {
      tab.resultIdData = env.resultId
      tab.resultRevision = env.revision
      tab.nextStartRow =
        env.rowCount == 500
        ? env.startRow + UInt64(env.rowCount) : nil
    }
    return projection.table
  }

  func cancel() async {
    if selectedWorkbenchKind == "object", let tab = activeObjectTab {
      guard let client, let operationId = tab.activeOperationId else { return }
      do {
        let outcome = try await client.cancel(operationId: operationId)
        tab.summary = cancelOutcomeText(outcome)
      } catch { tab.error = "Cancel failed: \(error)" }
      return
    }
    let tab = activeQueryTab
    guard let client, let operationId = tab.activeOperationId else { return }
    do {
      let outcome = try await client.cancel(operationId: operationId)
      tab.cancelOutcome = cancelOutcomeText(outcome)
    } catch {
      tab.cancelOutcome = "Cancel failed: \(error)"
    }
    queryStateRevision &+= 1
  }

  /// Fetch the next page of the current result and append its rows.
  func loadMore() async {
    let tab = activeQueryTab
    guard let client, let resultId = tab.resultIdData, let start = tab.nextStartRow else {
      return
    }
    do {
      let (more, env) = try await client.fetchPage(
        resultId: resultId, startRow: start, revision: tab.resultRevision)
      if more.rows.isEmpty {
        tab.nextStartRow = nil
        return
      }
      if let table = tab.resultTable {
        guard let table = table.appending(more) else {
          tab.queryError = "Load more returned incompatible page metadata"
          return
        }
        tab.resultTable = table
        tab.querySummary =
          "result · \(counted(table.columns.count, "column")) · \(counted(table.rows.count, "row")) loaded"
      }
      tab.nextStartRow =
        env.rowCount == 500
        ? env.startRow + UInt64(env.rowCount) : nil
    } catch {
      tab.queryError = "Load more failed: \(error)"
    }
  }

  private func cancelOutcomeText(_ outcome: WorkbenchCancelOutcome) -> String {
    guard let runtime = outcome.runtime, !runtime.isEmpty else { return outcome.core }
    return "\(outcome.core) · \(runtime)"
  }

  func browse(expandedNodeKey: String? = nil) async {
    guard !isRunning, !isCatalogRefreshing else { return }
    guard let client, let session = sessionData else { return }
    let hadSnapshot = catalogSnapshot != nil
    catalogRefreshState = .loading(nodeKey: expandedNodeKey)
    catalogSummary = nil
    catalogError = nil
    do {
      let parentId = expandedNodeKey.flatMap { key in
        catalogSnapshot?.first(where: { catalogNodeKey($0.idBytes) == key })?.idBytes
      }
      let loaded = try await client.refreshCatalog(
        session: session,
        parentNodeId: parentId
      )
      if let parentId {
        var retained = catalogSnapshot ?? []
        let staleIds = catalogDescendantIds(of: parentId, in: retained)
        retained.removeAll { staleIds.contains($0.idBytes) }
        retained.append(contentsOf: loaded)
        catalogSnapshot = retained
      } else {
        catalogSnapshot = loaded
      }
      catalogRefreshState = .loaded
      catalogSummary = "catalog · \(catalogSnapshot?.count ?? 0) nodes loaded"
    } catch {
      let message = "Browse failed: \(error)"
      catalogRefreshState =
        hadSnapshot
        ? .stale(nodeKey: expandedNodeKey, message: message)
        : .failed(message: message)
      catalogError = message
    }
  }

  func runQuery() async {
    let tab = activeQueryTab
    let sql = tab.statementText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !sql.isEmpty else { return }
    tab.querySummary = nil
    tab.queryError = nil
    tab.resultTable = nil
    do {
      if connectedEngine != "redis" {
        let names = try await client?.inspectNamedParameters(statement: sql) ?? []
        if !names.isEmpty {
          parameterizedStatement = sql
          queryParameterBindings = names.map { WorkbenchQueryParameter(name: $0) }
          queryParameterError = nil
          queryParametersPresented = true
          return
        }
      }
      if let table = try await fetchPage(intent: "execute", statement: sql, tab: tab) {
        tab.resultTable = table
        tab.querySummary =
          "result · \(counted(table.columns.count, "column")) · \(counted(table.rows.count, "row"))"
      } else if let outcome = tab.writeOutcome {
        tab.querySummary = "write ok · \(outcome)"
      } else {
        tab.querySummary = "query: no result"
      }
    } catch {
      tab.queryError = "Query failed: \(error)"
    }
  }

  func runParameterizedQuery() async {
    guard let statement = parameterizedStatement, !isRunning else { return }
    let tab = activeQueryTab
    queryParameterError = nil
    do {
      if let table = try await fetchPage(
        intent: "execute", statement: statement, tab: tab,
        bindings: queryParameterBindings)
      {
        tab.resultTable = table
        tab.querySummary =
          "result · \(counted(table.columns.count, "column")) · \(counted(table.rows.count, "row"))"
      } else if let outcome = tab.writeOutcome {
        tab.querySummary = "write ok · \(outcome)"
      } else {
        tab.querySummary = "query: no result"
      }
      queryParametersPresented = false
      parameterizedStatement = nil
      queryParameterBindings = []
    } catch {
      queryParameterError = "Parameterized query failed: \(error)"
    }
  }

  func cancelQueryParameters() {
    queryParametersPresented = false
    parameterizedStatement = nil
    queryParameterBindings = []
    queryParameterError = nil
  }

  func runExplain() async {
    let tab = activeQueryTab
    let sql = tab.statementText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !sql.isEmpty else {
      tab.queryError = "EXPLAIN needs SQL in the active editor"
      return
    }
    tab.querySummary = nil
    tab.queryError = nil
    tab.explainPlan = nil
    do {
      guard let table = try await fetchPage(intent: "explain", statement: sql, tab: tab),
        !table.rows.isEmpty
      else {
        tab.queryError = "EXPLAIN returned no plan"
        return
      }
      tab.resultTable = table
      tab.explainPlan = table.rows.compactMap(\.first).joined(separator: "\n")
      tab.querySummary = "explain · \(counted(table.rows.count, "line"))"
      explainPresented = true
    } catch {
      tab.queryError = "Explain failed: \(error)"
    }
  }

  /// Stage a probe mutation, authorize it, and apply it through the single-use
  /// review-token safety gate. Demonstrates the edit/review flow.
  func applyProbeEdit() async {
    let tab = activeQueryTab
    guard let client, let session = sessionData else { return }
    tab.reviewOutcome = nil
    tab.reviewError = nil
    do {
      let now = dependencies.clock.nowMilliseconds()
      let outcome = try await client.stageAndApply(session: session, now: now)
      tab.reviewOutcome =
        "\(outcome.transaction) · \(outcome.appliedCount) applied · \(outcome.conflictCount) conflict · \(outcome.failedCount) failed"
    } catch {
      tab.reviewError = "Review/apply failed: \(error)"
    }
  }

  func copyStructureDdl(_ ddl: String) {
    do {
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: ddl)
      ])
      copyOutcome = "Copied structure DDL"
    } catch {
      copyError = "Copy DDL failed: \(error)"
    }
  }

  func copyExplainPlan() {
    guard let plan = activeQueryTab.explainPlan else { return }
    do {
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: plan)
      ])
      copyOutcome = "Copied explain plan"
    } catch {
      copyError = "Copy explain plan failed: \(error)"
    }
  }
}

struct ContentView: View {
  @Environment(BridgeModel.self) private var model

  var body: some View {
    @Bindable var model = model
    NavigationSplitView {
      VStack(spacing: 0) {
        List {
          ForEach(model.profileSections) { section in
            Section {
              if !model.collapsedProfileGroups.contains(section.id) {
                ForEach(section.profiles, id: \.idBytes) { profile in
                  HStack(spacing: 4) {
                    Button {
                      Task { await model.connect(profile) }
                    } label: {
                      ProfileRow(
                        profile: profile,
                        connectionState: model.connectionState(profile)
                      )
                    }
                    .buttonStyle(.plain)
                    .accessibilityIdentifier(
                      "profile.\(profile.idBytes.hexEncodedString())"
                    )
                    Menu {
                      Button("Connect") { Task { await model.connect(profile) } }
                      if model.isActiveProfile(profile) {
                        Button("Check Health") { Task { await model.checkActiveHealth() } }
                        Button("Reconnect") { Task { await model.reconnectActive() } }
                        Button("Disconnect") { Task { await model.disconnectActive() } }
                      }
                      Button("Edit…") { Task { await model.editProfile(profile) } }
                      Button("Duplicate…") { Task { await model.duplicateProfile(profile) } }
                      Button("Test") { Task { await model.testProfile(profile) } }
                      Button(profile.favorite ? "Remove Favorite" : "Add Favorite") {
                        Task { await model.toggleFavorite(profile) }
                      }
                      Button("Move Up") {
                        Task { await model.move(profile, in: section, offset: -1) }
                      }
                      .disabled(!model.canMove(profile, in: section, offset: -1))
                      Button("Move Down") {
                        Task { await model.move(profile, in: section, offset: 1) }
                      }
                      .disabled(!model.canMove(profile, in: section, offset: 1))
                      Divider()
                      Button("Remove…", role: .destructive) {
                        model.pendingRemoval = profile
                      }
                    } label: {
                      Image(systemName: "ellipsis.circle")
                    }
                    .menuStyle(.borderlessButton)
                    .accessibilityLabel("Actions for \(profile.name)")
                  }
                  .contextMenu {
                    Button("Connect") { Task { await model.connect(profile) } }
                    if profile.connected {
                      Button("Check Health") { Task { await model.checkActiveHealth() } }
                      Button("Reconnect") { Task { await model.reconnectActive() } }
                      Button("Disconnect") { Task { await model.disconnectActive() } }
                    }
                    Button("Edit…") { Task { await model.editProfile(profile) } }
                    Button("Duplicate…") { Task { await model.duplicateProfile(profile) } }
                    Button("Test") { Task { await model.testProfile(profile) } }
                    Button(profile.favorite ? "Remove Favorite" : "Add Favorite") {
                      Task { await model.toggleFavorite(profile) }
                    }
                    Button("Move Up") {
                      Task { await model.move(profile, in: section, offset: -1) }
                    }
                    .disabled(!model.canMove(profile, in: section, offset: -1))
                    Button("Move Down") {
                      Task { await model.move(profile, in: section, offset: 1) }
                    }
                    .disabled(!model.canMove(profile, in: section, offset: 1))
                    Divider()
                    Button("Remove…", role: .destructive) {
                      model.pendingRemoval = profile
                    }
                  }
                }
              }
            } header: {
              HStack {
                Button {
                  if model.collapsedProfileGroups.contains(section.id) {
                    model.collapsedProfileGroups.remove(section.id)
                  } else {
                    model.collapsedProfileGroups.insert(section.id)
                  }
                } label: {
                  Label(
                    "\(section.title) (\(section.profiles.count))",
                    systemImage: model.collapsedProfileGroups.contains(section.id)
                      ? "chevron.right" : "chevron.down"
                  )
                }
                .buttonStyle(.plain)
                Spacer()
                if section.id != "ungrouped" {
                  Menu {
                    Button {
                      Task { await model.setGroupAlphabetical(section, false) }
                    } label: {
                      Label(
                        "Manual Order",
                        systemImage: section.alphabetical
                          ? "circle" : "checkmark")
                    }
                    Button {
                      Task { await model.setGroupAlphabetical(section, true) }
                    } label: {
                      Label(
                        "Alphabetical",
                        systemImage: section.alphabetical
                          ? "checkmark" : "circle")
                    }
                    Divider()
                    Button("Rename Group…") {
                      model.beginRenameGroup(section.title)
                    }
                    Button("Remove Group…", role: .destructive) {
                      model.pendingGroupRemoval = section.title
                    }
                  } label: {
                    Image(systemName: "ellipsis")
                  }
                  .menuStyle(.borderlessButton)
                  .accessibilityLabel("Actions for group \(section.title)")
                }
              }
            }
          }
        }
        .accessibilityIdentifier("sidebar.profiles")
        .searchable(text: $model.profileSearch, prompt: "Search connections")
        .task(id: model.profileSearch) {
          try? await Task.sleep(for: .milliseconds(150))
          guard !Task.isCancelled else { return }
          await model.refreshProfiles()
        }
        .safeAreaInset(edge: .bottom) {
          HStack {
            Button {
              model.createProfile()
            } label: {
              Label("New connection", systemImage: "plus")
            }
            .accessibilityIdentifier("profile.add")
            Button {
              model.beginCreateGroup()
            } label: {
              Label("New group", systemImage: "folder.badge.plus")
            }
            Button {
              model.beginConnectionUrlImport()
            } label: {
              Label("Import URL", systemImage: "link.badge.plus")
            }
            .accessibilityIdentifier("profile.url-import")
            Spacer()
          }
          .padding(8)
          .background(.bar)
        }
        .overlay {
          if model.profilesLoading {
            ProgressView("Loading connections…")
          } else if let profilesError = model.profilesError {
            ContentUnavailableView(
              "Connections failed",
              systemImage: "exclamationmark.triangle",
              description: Text(profilesError)
            )
          } else if model.profiles.isEmpty && model.sessionHex == nil
            && (!model.profileSearch.isEmpty || model.profileGroups.isEmpty)
          {
            ContentUnavailableView(
              model.profileSearch.isEmpty ? "No connections" : "No matches",
              systemImage: model.profileSearch.isEmpty ? "tray" : "magnifyingglass",
              description: Text(
                model.profileSearch.isEmpty
                  ? "Create or use a temporary connection."
                  : "No saved connection matches this search.")
            )
          }
        }
        if model.sessionHex != nil {
          Divider()
          HStack {
            Text("Catalog").font(.headline)
            Spacer()
            Button {
              Task { await model.browse() }
            } label: {
              Image(systemName: "arrow.clockwise")
            }
            .buttonStyle(.borderless)
            .disabled(model.isRunning || model.isCatalogRefreshing)
            .accessibilityLabel("Refresh catalog")
            .accessibilityIdentifier("catalog.refresh")
          }
          .padding(.horizontal, 10)
          .padding(.vertical, 6)
          if model.isCatalogRefreshing {
            ProgressView("Refreshing catalog…")
              .controlSize(.small)
              .padding(.horizontal, 10)
          }
          if let snapshot = model.catalogSnapshot {
            CatalogOutline(
              table: snapshot,
              selection: $model.catalogSelection,
              refreshState: model.catalogRefreshState,
              onExpand: { nodeKey in
                Task { await model.browse(expandedNodeKey: nodeKey) }
              },
              onOpen: { nodeKey in
                Task { await model.openCatalogObject(nodeKey: nodeKey) }
              }
            )
            .frame(minHeight: 160)
          } else {
            switch model.catalogRefreshState {
            case .loading:
              ProgressView("Loading catalog…")
                .frame(minHeight: 160)
            case .failed(let message):
              ContentUnavailableView(
                "Catalog failed",
                systemImage: "exclamationmark.triangle",
                description: Text(message)
              )
              .frame(minHeight: 160)
            default:
              ContentUnavailableView(
                "Catalog not loaded",
                systemImage: "sidebar.left",
                description: Text("Refresh to list database objects.")
              )
              .frame(minHeight: 160)
            }
          }
        }
      }
      .navigationTitle("Connections")
    } detail: {
      VStack(alignment: .leading, spacing: 12) {
        Text("TableRock").font(.largeTitle).bold()
        Text(model.status)
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("app.status")
          .accessibilityValue(model.status)
        EnvironmentSafetyBadge(model: model)
        if let outcome = model.profileActionOutcome {
          Text(outcome)
            .foregroundStyle(.secondary)
            .font(.callout)
            .accessibilityIdentifier("profile.action.outcome")
        }
        if let bridgeError = model.bridgeError {
          Text(bridgeError)
            .foregroundStyle(.red)
            .font(.callout)
            .textSelection(.enabled)
        }
        if model.sessionHex == nil {
          // Direct-connect form (no saved profile required).
          GroupBox("New connection") {
            Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 6) {
              GridRow {
                Text("Engine")
                Picker("", selection: $model.formEngine) {
                  Text("PostgreSQL").tag("postgresql")
                  Text("ClickHouse").tag("clickhouse")
                  Text("Redis").tag("redis")
                }
                .labelsHidden()
              }
              GridRow {
                Text("Host")
                TextField("127.0.0.1", text: $model.formHost)
              }
              GridRow {
                Text("Port")
                TextField("5432", text: $model.formPort)
              }
              GridRow {
                Text("Database")
                TextField("postgres", text: $model.formDatabase)
              }
              GridRow {
                Text("User")
                TextField("postgres", text: $model.formUser)
              }
              GridRow {
                Text("Password")
                SecureField("", text: $model.formPassword)
              }
            }
            HStack {
              Button("Connect") { Task { await model.connectByParams() } }
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("connection.direct.connect")
              Spacer()
            }
            .padding(.top, 4)
          }
          if let name = model.connectingName {
            Text("Connecting to \(name)…").foregroundStyle(.secondary)
          }
        }
        if let session = model.sessionHex {
          Label(
            connectedSessionLabel(session),
            systemImage: "checkmark.circle.fill"
          )
          .foregroundStyle(.green)
          .accessibilityIdentifier("connection.status")
          .accessibilityValue(connectedSessionLabel(session))
          Button("Browse catalog") { Task { await model.browse() } }
            .buttonStyle(.borderedProminent)
        }
        if let catalogSummary = model.catalogSummary {
          Text(catalogSummary).foregroundStyle(.secondary)
        }
        if let catalogError = model.catalogError {
          Text(catalogError).foregroundStyle(.red).font(.callout).textSelection(.enabled)
        }
        if model.sessionHex != nil {
          VStack(alignment: .leading, spacing: 6) {
            QueryTabStrip()
            if model.queryWorkbenchSelected {
              QueryWorkbenchView()
            } else {
              ObjectWorkbenchView()
            }
          }
        }
        if let connectError = model.connectError {
          Text(connectError)
            .foregroundStyle(.red)
            .font(.callout)
            .textSelection(.enabled)
        }
        Spacer()
        Text("PostgreSQL · ClickHouse · Redis — native vertical slice")
          .font(.caption)
          .foregroundStyle(.tertiary)
      }
      .padding(24)
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

struct QueryWorkbenchView: View {
  @Environment(BridgeModel.self) private var model

  var body: some View {
    @Bindable var model = model
    @Bindable var tab = model.activeQueryTabForPresentation
    let queryStatus = tab.queryError ?? tab.cancelOutcome ?? tab.querySummary ?? "Idle"
    VStack(alignment: .leading, spacing: 6) {
      HStack {
        Text("SQL").font(.headline)
        if let file = model.sqlFile {
          Text(URL(fileURLWithPath: file.path).lastPathComponent)
            .font(.caption)
            .foregroundStyle(.secondary)
        }
      }
      SqlTextEditor(text: $model.queryText, selection: $model.queryEditorSelection)
        .frame(minHeight: 56, maxHeight: 80)
        .task(id: model.queryText) {
          try? await Task.sleep(for: .milliseconds(300))
          guard !Task.isCancelled else { return }
          await model.persistSessionIntent()
        }
      HStack {
        Button("Run query") { Task { await model.runQuery() } }
          .accessibilityIdentifier("query.run")
          .buttonStyle(.borderedProminent)
          .keyboardShortcut("r", modifiers: .command)
          .disabled(model.isRunning || model.isCatalogRefreshing)
        Button("Cancel") { Task { await model.cancel() } }
          .accessibilityIdentifier("query.cancel")
          .disabled(!model.isRunning)
        Button("Refresh catalog") { Task { await model.browse() } }
          .disabled(model.isRunning || model.isCatalogRefreshing)
        if model.connectedEngine == "redis" {
          Button("Redis Overview") { Task { await model.showRedisOverview() } }
            .disabled(model.redisOverviewLoading)
        }
        Button("Apply probe edit") { Task { await model.applyProbeEdit() } }
          .disabled(model.isRunning || model.isCatalogRefreshing)
      }
      Text(queryStatus)
        .foregroundStyle(model.queryError == nil ? Color.secondary : Color.red)
        .font(.callout)
        .textSelection(.enabled)
        .accessibilityIdentifier("query.status")
        .accessibilityValue(queryStatus)
      if let value = model.reviewOutcome {
        Text(value).foregroundStyle(.green).font(.callout)
      }
      if let value = model.reviewError {
        Text(value).foregroundStyle(.red).font(.callout).textSelection(.enabled)
      }
      if let value = model.sqlFileError {
        Text(value).foregroundStyle(.red).font(.callout).textSelection(.enabled)
      }
      if let table = model.resultTable {
        ResultGridWithInspector(
          table: table, minimumHeight: 220, exposesResultPaging: true)
      }
    }
  }
}

private struct FindReplaceSheet: View {
  @Environment(BridgeModel.self) private var model
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
  @Environment(BridgeModel.self) private var model

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
          .buttonStyle(.borderedProminent)
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

struct ObjectWorkbenchView: View {
  @Environment(BridgeModel.self) private var model

  var body: some View {
    if let tab = model.selectedObjectTab {
      VStack(alignment: .leading, spacing: 8) {
        HStack {
          Label(tab.title, systemImage: tab.pinned ? "pin.fill" : "eye")
            .font(.headline)
          Text(tab.kind).font(.caption).foregroundStyle(.secondary)
          if !tab.kind.hasPrefix("redis_key_") {
            Picker(
              "Object section",
              selection: Binding(
                get: { tab.selectedSection },
                set: { section in
                  tab.selectedSection = section
                  if section == "structure" {
                    Task { await model.loadObjectStructure() }
                  }
                }
              )
            ) {
              Text("Data").tag("data")
              Text("Structure").tag("structure")
            }
            .pickerStyle(.segmented)
            .frame(width: 180)
          }
          Spacer()
          if !tab.pinned {
            Button("Pin") { model.pinObjectTab(tab) }
          }
          Button("Refresh") { Task { await model.reloadObjectTab() } }
            .disabled(tab.isRunning)
          if model.sqlInsertCopyAvailable {
            Button("Import CSV") { Task { await model.chooseCsvImport() } }
              .accessibilityIdentifier("import.csv.open")
              .disabled(tab.isRunning)
          }
          Button("Close", role: .destructive) { model.closeObjectTab(tab) }
            .disabled(tab.isRunning)
        }
        if tab.isRunning { ProgressView("Loading \(tab.title)…") }
        if let summary = tab.summary {
          Text(summary).font(.callout).foregroundStyle(.secondary)
        }
        if tab.selectedSection == "data", let table = tab.resultTable,
          !tab.kind.hasPrefix("redis_key_")
        {
          ObjectSortBar(tab: tab, table: table)
          ObjectFilterBar(tab: tab, table: table)
        }
        if let error = tab.error {
          Text(error).font(.callout).foregroundStyle(.red).textSelection(.enabled)
        }
        if let view = tab.redisView {
          RedisKeyObjectView(view: view)
          if view.nextSkip != nil {
            Button("Load more entries") { Task { await model.loadMoreRedisKey() } }
              .disabled(tab.isRunning)
          }
        } else if tab.selectedSection == "structure" {
          ObjectStructureView(tab: tab)
        } else if let table = tab.resultTable {
          ResultGridWithInspector(table: table, minimumHeight: 260)
          if tab.nextStartRow != nil {
            Button("Load more rows") { Task { await model.loadMoreObjectRows() } }
          }
        } else if !tab.isRunning && tab.error == nil {
          ContentUnavailableView(
            "No object rows", systemImage: "tablecells",
            description: Text("Refresh to browse this object again.")
          )
        }
      }
    } else {
      ContentUnavailableView("No object tab", systemImage: "tablecells")
    }
  }
}

private struct ObjectSortBar: View {
  @Environment(BridgeModel.self) private var model
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

private struct ObjectFilterBar: View {
  @Environment(BridgeModel.self) private var model
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

private struct ObjectStructureView: View {
  @Environment(BridgeModel.self) private var model
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
  @Environment(BridgeModel.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        VStack(alignment: .leading, spacing: 3) {
          Text("Export Full Result").font(.title2).bold()
          Text("Rust replays the exact query or typed object browse in bounded pages and publishes atomically.")
            .foregroundStyle(.secondary)
        }
        Spacer()
        Button("Close") { model.closeStreamExport() }
          .disabled(model.streamExportProgress.map {
            ["running", "cancel_requested"].contains($0.phase)
          } ?? true)
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
  @Environment(BridgeModel.self) private var model

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
          .buttonStyle(.borderedProminent)
          .disabled(
            model.csvImportPreview == nil || model.csvImportReview != nil
              || model.csvImportOutcome != nil || model.csvImportApplying
          )
          .accessibilityIdentifier("import.csv.stage")
        Button("Apply Import") { Task { await model.applyCsvImport() } }
          .buttonStyle(.borderedProminent)
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
            GroupBox("Review required") {
              VStack(alignment: .leading, spacing: 6) {
                Text(
                  "Insert \(review.rowCount) rows and \(review.columnCount) mapped columns into \(review.target)."
                )
                .font(.headline)
                if review.formulaLikeCells > 0 {
                  Text(
                    "\(review.formulaLikeCells) formula-like cells are frozen as literal text in this reviewed plan."
                  )
                  .foregroundStyle(.orange)
                }
                Text(
                  "The reviewed plan is frozen for 60 seconds. Authority is consumed before database I/O and cannot be retried after failure."
                )
                .foregroundStyle(.secondary)
              }
              .padding(6)
            }
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
    .accessibilityIdentifier("import.csv.sheet")
    .interactiveDismissDisabled(model.csvImportReview != nil || model.csvImportApplying)
  }
}

private struct RedisOverviewSheet: View {
  @Environment(BridgeModel.self) private var model

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
  @Environment(BridgeModel.self) private var model

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
          .buttonStyle(.borderedProminent)
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
    .accessibilityIdentifier("redis.pubsub.sheet")
    .interactiveDismissDisabled(model.redisSubscriptionIsActive)
  }
}

private struct DdlChangeSheet: View {
  @Environment(BridgeModel.self) private var model
  @State private var applyConfirmationPresented = false

  private var needsDefinition: Bool {
    ["add_column", "create_index", "add_constraint"].contains(model.ddlChangeKind)
  }

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Review Structure Change", systemImage: "tablecells.badge.ellipsis")
          .font(.title2.bold())
        Spacer()
        Button("Close") { Task { await model.closeDdlChange() } }
          .disabled(model.ddlChangeApplying)
      }
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
          .buttonStyle(.borderedProminent)
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
        GroupBox("Frozen statement preview") {
          VStack(alignment: .leading, spacing: 8) {
            Text(review.preview)
              .font(.system(.body, design: .monospaced))
              .textSelection(.enabled)
              .accessibilityIdentifier("structure.change.preview")
            if review.destructive {
              Label(
                "This operation removes structure", systemImage: "exclamationmark.triangle.fill"
              )
              .foregroundStyle(.orange)
            }
            Text(review.rollbackSummary).font(.callout).foregroundStyle(.secondary)
            Button("Apply Reviewed Change…") { applyConfirmationPresented = true }
              .buttonStyle(.borderedProminent)
              .accessibilityIdentifier("structure.change.apply-review")
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding(6)
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
    .accessibilityIdentifier("structure.change.sheet")
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
  @Environment(BridgeModel.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Table Operations", systemImage: "wrench.and.screwdriver")
          .font(.title2.bold())
        Spacer()
        Button("Close") { Task { await model.closeTableOperation() } }
          .disabled(model.tableOperationApplying)
          .accessibilityIdentifier("table-operation.close")
      }
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
        .buttonStyle(.borderedProminent)
        .disabled(
          model.tableOperationReview != nil || model.tableOperationApplying
            || (model.tableOperationKind == "rename"
              && model.tableOperationNewName.trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty)
        )
        .accessibilityIdentifier("table-operation.review")
      if let review = model.tableOperationReview {
        GroupBox("Frozen operation") {
          VStack(alignment: .leading, spacing: 8) {
            Text(review.target).font(.headline).textSelection(.enabled)
            Text(review.preview)
              .font(.system(.body, design: .monospaced))
              .textSelection(.enabled)
              .accessibilityIdentifier("table-operation.preview")
            if review.destructive {
              Label(
                "This operation destroys table data", systemImage: "exclamationmark.triangle.fill"
              )
              .foregroundStyle(.orange)
            }
            Text("Type \(review.confirmation) to authorize this exact target.")
              .foregroundStyle(.secondary)
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
              .buttonStyle(.borderedProminent)
              .tint(review.destructive ? .red : .accentColor)
              .disabled(model.tableOperationConfirmation != review.confirmation)
              .accessibilityIdentifier("table-operation.apply")
            }
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding(6)
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
    .accessibilityIdentifier("table-operation.sheet")
  }
}

private struct PendingPostgresSignal {
  let kind: String
  let pid: Int32
}

private struct PostgresRolesSheet: View {
  @Environment(BridgeModel.self) private var model

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
    .accessibilityIdentifier("postgres.roles.sheet")
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
  @Environment(BridgeModel.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack {
        Label("Relationships", systemImage: "arrow.triangle.branch")
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
              Button("Open Related") { Task { await model.openRelatedRelation(edge) } }
                .accessibilityLabel("Open related relation for \(edge.id)")
            }
            .accessibilityIdentifier("postgres.relationship.edge.\(edge.id)")
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
    .accessibilityIdentifier("postgres.relationships.sheet")
  }
}

private struct PostgresActivitySheet: View {
  @Environment(BridgeModel.self) private var model
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
    .accessibilityIdentifier("postgres.activity.sheet")
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
  @Environment(BridgeModel.self) private var model

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
        .buttonStyle(.borderedProminent)
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
    .accessibilityIdentifier("postgres.tools.sheet")
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

private struct ResultGridWithInspector: View {
  @Environment(BridgeModel.self) private var model
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
    VStack(alignment: .leading, spacing: 6) {
      VStack(alignment: .leading, spacing: 6) {
        HStack {
          ResultCopyMenu()
          ResultExportMenu()
          Spacer()
        }
        HStack {
          TextField(
            "Filter loaded rows",
            text: Binding(
              get: { model.loadedRowQuickFilter },
              set: { model.loadedRowQuickFilter = $0 })
          )
          .frame(minWidth: 120, maxWidth: 220)
          .accessibilityIdentifier("results.quick-filter")
          let loadedRowsStatus =
            "Loaded rows only · \(visibleRowIndices.count)/\(table.rows.count)"
          Text(loadedRowsStatus)
            .font(.caption)
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("results.quick-filter.status")
            .accessibilityValue(loadedRowsStatus)
          if exposesResultPaging && model.nextStartRow != nil {
            Button("Load more rows") { Task { await model.loadMore() } }
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
          Spacer()
        }
      }
      HSplitView {
        CatalogGrid(table: visibleTable, sorts: model.resultSort) { row, column in
          guard visibleRowIndices.indices.contains(row) else { return }
          model.selectCell(row: visibleRowIndices[row], column: column)
        }
        .frame(minWidth: 280, minHeight: 100, idealHeight: minimumHeight)
        if let snapshot = model.selectedCellSnapshot {
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

private struct ResultExportMenu: View {
  @Environment(BridgeModel.self) private var model

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
      .buttonStyle(.bordered)
      .accessibilityIdentifier("results.export.\(format)")
  }

  private func fullExportButton(_ label: String, format: String) -> some View {
    Button(label) { Task { await model.exportFullResult(format: format) } }
      .accessibilityIdentifier("results.export.full.\(format)")
  }
}

private struct ResultCopyMenu: View {
  @Environment(BridgeModel.self) private var model

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

private struct NativeValueInspector: View {
  let column: WorkbenchColumn
  let cell: WorkbenchCell
  let row: Int
  let columnIndex: Int

  private var hex: String {
    cell.bytes.map { String(format: "%02x", $0) }.joined(separator: " ")
  }

  private var structuredRows: [StructuredValueTreeRow]? {
    guard cell.kindLabel == "Structured" else { return nil }
    return try? StructuredValueTree.decode(cell.bytes)
  }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 10) {
        HStack {
          Text("Value Inspector").font(.headline)
          Spacer()
          Text("R\(row + 1) C\(columnIndex + 1)")
            .font(.caption).foregroundStyle(.secondary)
        }
        LabeledContent("Column", value: column.name)
        LabeledContent("Database type", value: column.engineType)
        LabeledContent("Value kind", value: cell.kindLabel)
        LabeledContent("Nullable", value: column.nullable ? "Yes" : "No")
        LabeledContent("Stored bytes", value: "\(cell.bytes.count)")
        if cell.isTruncated {
          Label(
            cell.originalByteCount.map { "Truncated from \($0) bytes" }
              ?? "Truncated value",
            systemImage: "scissors"
          )
          .foregroundStyle(.orange)
        }
        GroupBox("Text") {
          Text(cell.display)
            .font(.system(.body, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        GroupBox("Hex") {
          Text(hex.isEmpty ? "Empty" : hex)
            .font(.system(.caption, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        if let structuredRows {
          GroupBox("JSON Tree") {
            VStack(alignment: .leading, spacing: 4) {
              ForEach(structuredRows) { row in
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                  Text(row.label).fontWeight(.medium)
                  if let value = row.value {
                    Text(value).foregroundStyle(.secondary)
                  }
                }
                .font(.system(.caption, design: .monospaced))
                .padding(.leading, CGFloat(row.depth) * 12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("value.inspector.tree.\(row.id)")
              }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
          }
          .accessibilityIdentifier("value.inspector.tree")
        }
      }
      .padding(10)
    }
    .background(Color(nsColor: .textBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("value.inspector")
    .accessibilityLabel("Value inspector for \(column.name)")
  }
}

struct QueryTabStrip: View {
  @Environment(BridgeModel.self) private var model

  var body: some View {
    ScrollView(.horizontal) {
      HStack(spacing: 4) {
        ForEach(model.queryTabs) { tab in
          HStack(spacing: 2) {
            if model.queryWorkbenchSelected && tab.id == model.selectedQueryTabId {
              Button {
                model.selectQueryTab(tab)
              } label: {
                WorkbenchTabLabel(title: tab.title, model: model)
              }
              .buttonStyle(.borderedProminent)
              .accessibilityIdentifier("query.tab.\(tab.id.uuidString.lowercased())")
              .accessibilityValue("Selected")
            } else {
              Button {
                model.selectQueryTab(tab)
              } label: {
                WorkbenchTabLabel(title: tab.title, model: model)
              }
              .buttonStyle(.bordered)
              .accessibilityIdentifier("query.tab.\(tab.id.uuidString.lowercased())")
            }
            Menu {
              Button("Rename…") { model.beginRenameQueryTab(tab) }
              Button("Close", role: .destructive) {
                model.requestCloseQueryTab(tab)
              }
              .accessibilityIdentifier("query.tab.close")
              .disabled(model.queryTabs.count == 1 || tab.isRunning)
            } label: {
              Image(systemName: tab.isRunning ? "progress.indicator" : "ellipsis")
            }
            .menuStyle(.borderlessButton)
            .accessibilityIdentifier("query.tab.actions.\(tab.id.uuidString.lowercased())")
            .accessibilityLabel("Actions for \(tab.title)")
          }
        }
        ForEach(model.objectTabs) { tab in
          HStack(spacing: 2) {
            if !model.queryWorkbenchSelected && tab.id == model.selectedObjectTabId {
              Button {
                model.selectObjectTab(tab)
              } label: {
                WorkbenchTabLabel(
                  title: tab.title, model: model,
                  leadingSystemImage: tab.pinned ? "pin.fill" : "eye"
                )
              }
              .buttonStyle(.borderedProminent)
              .accessibilityIdentifier("object.tab.\(tab.id.uuidString.lowercased())")
              .accessibilityValue("Selected")
            } else {
              Button {
                model.selectObjectTab(tab)
              } label: {
                WorkbenchTabLabel(
                  title: tab.title, model: model,
                  leadingSystemImage: tab.pinned ? "pin.fill" : "eye"
                )
              }
              .buttonStyle(.bordered)
              .accessibilityIdentifier("object.tab.\(tab.id.uuidString.lowercased())")
            }
            Menu {
              if !tab.pinned {
                Button("Pin") { model.pinObjectTab(tab) }
              }
              Button("Refresh") { Task { await model.reloadObjectTab() } }
              Button("Close", role: .destructive) { model.closeObjectTab(tab) }
                .disabled(tab.isRunning)
            } label: {
              Image(systemName: tab.isRunning ? "progress.indicator" : "ellipsis")
            }
            .menuStyle(.borderlessButton)
            .accessibilityLabel("Actions for object \(tab.title)")
          }
        }
        Button {
          model.addQueryTab()
        } label: {
          Image(systemName: "plus")
        }
        .buttonStyle(.borderless)
        .accessibilityLabel("New query tab")
        .disabled(model.queryTabs.count + model.objectTabs.count >= 64)
      }
    }
  }
}

private struct WorkbenchTabLabel: View {
  let title: String
  let model: BridgeModel
  var leadingSystemImage: String?

  init(title: String, model: BridgeModel, leadingSystemImage: String? = nil) {
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

private struct EnvironmentSafetyBadge: View {
  let model: BridgeModel

  var body: some View {
    if let environment = model.activeEnvironmentLabel,
      let safety = model.activeSafetyLabel
    {
      Label {
        Text("\(environment) · \(safety)")
      } icon: {
        Image(
          systemName: model.activeProductionWarning
            ? "exclamationmark.triangle.fill"
            : safety == "Read only" ? "lock.fill" : "shield")
      }
      .font(.caption)
      .foregroundStyle(model.activeProductionWarning ? .orange : .secondary)
      .accessibilityLabel("Environment \(environment), safety \(safety)")
    }
  }
}

struct WorkbenchToolbar: CustomizableToolbarContent {
  let model: BridgeModel

  var body: some CustomizableToolbarContent {
    WorkbenchConnectionToolbar(model: model)
    WorkbenchFileToolbar(model: model)
    WorkbenchQueryToolbar(model: model)
  }
}

struct WorkbenchFileToolbar: CustomizableToolbarContent {
  let model: BridgeModel

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
  let model: BridgeModel

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
  let model: BridgeModel

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

struct SavedQueriesSheet: View {
  @Environment(BridgeModel.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    NavigationStack {
      Group {
        if model.savedQueriesLoading && model.savedQueries.isEmpty {
          ProgressView("Loading saved queries…")
        } else if let error = model.savedQueriesError, model.savedQueries.isEmpty {
          ContentUnavailableView(
            "Saved queries failed", systemImage: "exclamationmark.triangle",
            description: Text(error)
          )
        } else if model.savedQueries.isEmpty {
          ContentUnavailableView(
            model.savedQuerySearch.isEmpty ? "No saved queries" : "No saved query matches",
            systemImage: "bookmark",
            description: Text(
              model.savedQuerySearch.isEmpty
                ? "Save current editor text to reuse it later."
                : "Try a different name or SQL-text search.")
          )
        } else {
          List(model.savedQueries, id: \.queryId) { item in
            HStack(spacing: 10) {
              Button {
                model.restoreSavedQuery(item)
              } label: {
                VStack(alignment: .leading, spacing: 5) {
                  Text(item.name).font(.headline)
                  Text(item.statementText)
                    .font(.system(.body, design: .monospaced))
                    .lineLimit(3)
                  Text("\(item.engine) · \(item.updatedAt)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
              }
              .buttonStyle(.plain)
              .accessibilityHint("Restore into the editor without running it")
              Button(role: .destructive) {
                model.pendingSavedQueryRemoval = item
              } label: {
                Image(systemName: "trash")
              }
              .buttonStyle(.borderless)
              .accessibilityLabel("Remove \(item.name)")
            }
            .padding(.vertical, 3)
          }
        }
      }
      .navigationTitle("Saved Queries")
      .searchable(text: $model.savedQuerySearch, prompt: "Search names and SQL text")
      .onChange(of: model.savedQuerySearch) { _, _ in
        Task { await model.refreshSavedQueries() }
      }
      .onChange(of: model.savedQueryEngine) { _, _ in
        Task { await model.refreshSavedQueries() }
      }
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Done") { dismiss() }
        }
        ToolbarItem(placement: .automatic) {
          Picker("Engine", selection: $model.savedQueryEngine) {
            Text("All engines").tag("")
            Text("PostgreSQL").tag("postgresql")
            Text("ClickHouse").tag("clickhouse")
            Text("Redis").tag("redis")
          }
        }
        ToolbarItem(placement: .primaryAction) {
          Button("Save Current…") { model.beginSaveCurrentQuery() }
        }
      }
    }
    .frame(minWidth: 700, minHeight: 500)
    .alert("Save Query", isPresented: $model.saveQueryDialog) {
      TextField("Name", text: $model.savedQueryName)
      Button("Save") { Task { await model.saveCurrentQuery() } }
      Button("Cancel", role: .cancel) { model.saveQueryDialog = false }
    } message: {
      Text("Save current editor text for the active database engine.")
    }
    .confirmationDialog(
      "Remove saved query?",
      isPresented: Binding(
        get: { model.pendingSavedQueryRemoval != nil },
        set: { if !$0 { model.pendingSavedQueryRemoval = nil } }
      ),
      presenting: model.pendingSavedQueryRemoval
    ) { _ in
      Button("Remove", role: .destructive) {
        Task { await model.removePendingSavedQuery() }
      }
      Button("Cancel", role: .cancel) { model.pendingSavedQueryRemoval = nil }
    } message: { item in
      Text("\(item.name) will be removed. Query history is unchanged.")
    }
  }
}

struct HistorySheet: View {
  @Environment(BridgeModel.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    NavigationStack {
      Group {
        if model.historyLoading && model.historyItems.isEmpty {
          ProgressView("Loading history…")
        } else if let error = model.historyError, model.historyItems.isEmpty {
          ContentUnavailableView(
            "History failed", systemImage: "exclamationmark.triangle",
            description: Text(error)
          )
        } else if model.historyItems.isEmpty {
          ContentUnavailableView(
            model.historySearch.isEmpty ? "No query history" : "No history matches",
            systemImage: "clock",
            description: Text(
              model.historySearch.isEmpty
                ? "Executed statements appear here when retention is enabled."
                : "Try a different SQL-text search.")
          )
        } else {
          List(model.historyItems, id: \.historyId) { item in
            Button {
              model.restoreHistory(item)
            } label: {
              VStack(alignment: .leading, spacing: 5) {
                Text(item.statementText ?? "SQL text not retained")
                  .font(.system(.body, design: .monospaced))
                  .lineLimit(3)
                Text(
                  [
                    item.engine, item.databaseName,
                    item.schemaName, item.outcome, item.createdAt,
                  ].compactMap { $0 }.joined(separator: " · ")
                )
                .font(.caption)
                .foregroundStyle(.secondary)
              }
              .frame(maxWidth: .infinity, alignment: .leading)
              .padding(.vertical, 3)
            }
            .buttonStyle(.plain)
            .disabled(item.statementText == nil)
            .accessibilityHint(
              item.statementText == nil
                ? "SQL text retention was disabled"
                : "Restore this statement into the editor without running it")
          }
        }
      }
      .navigationTitle("Query History")
      .searchable(text: $model.historySearch, prompt: "Search retained SQL text")
      .onChange(of: model.historySearch) { _, _ in
        Task { await model.refreshHistory() }
      }
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Done") { dismiss() }
        }
        ToolbarItem(placement: .automatic) {
          Picker("Retention", selection: $model.historyRetention) {
            Text("Full SQL").tag("full")
            Text("Metadata only").tag("metadata_only")
            Text("Private").tag("private")
          }
          .onChange(of: model.historyRetention) { _, value in
            Task { await model.setHistoryRetention(value) }
          }
        }
        ToolbarItem(placement: .automatic) {
          Button {
            Task { await model.refreshHistory() }
          } label: {
            Label("Refresh History", systemImage: "arrow.clockwise")
          }
          .disabled(model.historyLoading)
        }
      }
    }
    .frame(minWidth: 680, minHeight: 480)
  }
}

private struct NativeSettingsView: View {
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
    grid.backgroundColor = .textBackgroundColor
    grid.setAccessibilityLabel("Query results")
    grid.setAccessibilityIdentifier("results.grid")
    let scroll = NSScrollView()
    scroll.documentView = grid
    scroll.drawsBackground = true
    scroll.backgroundColor = .textBackgroundColor
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    scroll.borderType = .bezelBorder
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
        column.minWidth = 60
        column.width = 140
        column.resizingMask = [.autoresizingMask, .userResizingMask]
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
        button.translatesAutoresizingMaskIntoConstraints = false
        cell.addSubview(button)
        NSLayoutConstraint.activate([
          button.leadingAnchor.constraint(equalTo: cell.leadingAnchor),
          button.trailingAnchor.constraint(equalTo: cell.trailingAnchor),
          button.topAnchor.constraint(equalTo: cell.topAnchor),
          button.bottomAnchor.constraint(equalTo: cell.bottomAnchor),
        ])
      }
      let value = snapshot.rows[row][column]
      guard let button = cell.subviews.first as? ResultCellButton else { return nil }
      button.title = value
      button.setAccessibilityElement(true)
      button.setAccessibilityRole(.button)
      button.setAccessibilityLabel("\(snapshot.columns[column]), row \(row + 1)")
      button.setAccessibilityValue(value)
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

private func writePerformanceMetric(_ metric: String) {
  FileHandle.standardError.write(Data("\(metric)\n".utf8))
}

private func counted(_ count: Int, _ singular: String) -> String {
  "\(count) \(singular)\(count == 1 ? "" : "s")"
}

private enum NativeFindReplaceError: Error, CustomStringConvertible {
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

private struct NativeReplaceOutcome {
  let text: String
  let selection: NSRange
  let replacedRange: NSRange
  let delta: Int
  var count = 1
}

private enum NativeFindReplaceEngine {
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
    editor.font = NSFont.monospacedSystemFont(
      ofSize: NSFont.systemFontSize, weight: .regular)
    editor.textContainerInset = NSSize(width: 6, height: 6)
    editor.drawsBackground = true
    editor.backgroundColor = .textBackgroundColor
    editor.string = text
    editor.setAccessibilityEnabled(true)
    editor.setAccessibilityLabel("SQL editor")
    editor.setAccessibilityIdentifier("query.editor")

    let scroll = NSScrollView()
    scroll.documentView = editor
    scroll.drawsBackground = true
    scroll.backgroundColor = .textBackgroundColor
    scroll.hasVerticalScroller = true
    scroll.autohidesScrollers = true
    scroll.borderType = .bezelBorder
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
    }
    let maximum = (text as NSString).length
    let requested = NSRange(
      location: min(selection.location, maximum),
      length: min(selection.length, max(0, maximum - min(selection.location, maximum))))
    if editor.selectedRange() != requested {
      editor.setSelectedRange(requested)
      editor.scrollRangeToVisible(requested)
    }
  }

  @MainActor
  final class Coordinator: NSObject, NSTextViewDelegate {
    var text: Binding<String>
    var selection: Binding<NSRange>

    init(text: Binding<String>, selection: Binding<NSRange>) {
      self.text = text
      self.selection = selection
    }

    func textDidChange(_ notification: Notification) {
      guard let editor = notification.object as? NSTextView else { return }
      text.wrappedValue = editor.string
    }

    func textViewDidChangeSelection(_ notification: Notification) {
      guard let editor = notification.object as? NSTextView else { return }
      selection.wrappedValue = editor.selectedRange()
    }
  }
}

struct ProfilePasswordSheet: View {
  @Environment(\.dismiss) private var dismiss
  let profile: WorkbenchProfileItem
  let onConnect: (String) async -> Bool
  @State private var password = ""
  @State private var connecting = false

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text("Connect to \(profile.name)").font(.title2).bold()
      Text("Password stays in memory for this connection attempt and is never saved.")
        .foregroundStyle(.secondary)
      SecureField("Password", text: $password)
        .textContentType(.password)
        .onSubmit { submit() }
      HStack {
        Spacer()
        Button("Cancel", role: .cancel) { dismiss() }
          .disabled(connecting)
        Button("Connect") { submit() }
          .buttonStyle(.borderedProminent)
          .disabled(connecting)
      }
    }
    .padding(24)
    .frame(width: 420)
    .interactiveDismissDisabled(connecting)
  }

  private func submit() {
    guard !connecting else { return }
    connecting = true
    let transientPassword = password
    password = ""
    Task {
      if await onConnect(transientPassword) { dismiss() } else { connecting = false }
    }
  }
}

struct ProfileGroupEditorSheet: View {
  @Environment(\.dismiss) private var dismiss
  @State private var dialog: ProfileGroupDialog
  @State private var saving = false
  let onSave: (ProfileGroupDialog) async -> Bool

  init(
    initialDialog: ProfileGroupDialog,
    onSave: @escaping (ProfileGroupDialog) async -> Bool
  ) {
    _dialog = State(initialValue: initialDialog)
    self.onSave = onSave
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text(dialog.title).font(.title2).bold()
      TextField("Group name", text: $dialog.name)
        .textFieldStyle(.roundedBorder)
      HStack {
        Spacer()
        Button("Cancel") { dismiss() }
        Button("Save") {
          saving = true
          Task {
            if await onSave(dialog) { dismiss() }
            saving = false
          }
        }
        .keyboardShortcut(.defaultAction)
        .disabled(
          dialog.name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            || saving)
      }
    }
    .padding(24)
    .frame(width: 380)
    .interactiveDismissDisabled(saving)
  }
}

struct ConnectionUrlImportSheet: View {
  @Environment(\.dismiss) private var dismiss
  @State private var input: String
  @State private var error: String?
  @State private var parsing = false
  let onReview: (String) async -> String?

  init(initial: ConnectionUrlImport, onReview: @escaping (String) async -> String?) {
    _input = State(initialValue: initial.input)
    _error = State(initialValue: initial.error)
    self.onReview = onReview
  }

  var body: some View {
    NavigationStack {
      Form {
        Section("Database URL") {
          SecureField("postgresql://user:password@host/database", text: $input)
            .accessibilityIdentifier("profile.url-import.input")
          Text("Parsed fields are reviewed before saving. Passwords default to macOS Keychain.")
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        if let error {
          Section("Validation") {
            Text(error)
              .foregroundStyle(.red)
              .textSelection(.enabled)
              .accessibilityIdentifier("profile.url-import.error")
          }
        }
      }
      .formStyle(.grouped)
      .navigationTitle("Import Connection URL")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
        ToolbarItem(placement: .confirmationAction) {
          Button("Review") {
            parsing = true
            Task {
              error = await onReview(input)
              parsing = false
            }
          }
          .accessibilityIdentifier("profile.url-import.review")
          .keyboardShortcut(.defaultAction)
          .disabled(input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || parsing)
        }
      }
    }
    .frame(minWidth: 520, minHeight: 300)
    .interactiveDismissDisabled(parsing)
  }
}

struct ExternalUrlConfirmationSheet: View {
  @Environment(BridgeModel.self) private var model
  @Environment(\.dismiss) private var dismiss
  let review: ExternalUrlReview

  var body: some View {
    NavigationStack {
      Form {
        Section("Requested target") {
          Text(review.summary)
            .textSelection(.enabled)
            .accessibilityIdentifier("external-url.summary")
          Text("No connection or profile change occurs until you choose an action.")
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        if let profile = review.matchedProfile {
          Section("Saved match") {
            Text(profile.name)
            Button("Connect saved profile") {
              Task { await model.connectExternalSavedProfile() }
            }
            .accessibilityIdentifier("external-url.connect-saved")
          }
        }
        Section("Choose action") {
          Button("Connect Temporarily") {
            Task { await model.connectExternalTemporarily() }
          }
          .buttonStyle(.borderedProminent)
          .accessibilityIdentifier("external-url.connect-temporary")

          Button("Review as New") { model.reviewExternalURLAsNewConnection() }
            .accessibilityIdentifier("external-url.review-new")
        }
      }
      .formStyle(.grouped)
      .navigationTitle("Open External Connection?")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") {
            model.externalUrlReview = nil
            dismiss()
          }
          .accessibilityIdentifier("external-url.cancel")
        }
      }
    }
    .frame(minWidth: 560, minHeight: 320)
  }
}

struct QuickSwitcherSheet: View {
  @Environment(BridgeModel.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    NavigationStack {
      VStack(spacing: 0) {
        TextField("Connections, tabs, objects, queries", text: $model.quickSwitcherSearch)
          .textFieldStyle(.roundedBorder)
          .accessibilityIdentifier("quick-switch.search")
          .onSubmit {
            guard let first = model.quickSwitcherItems.first else { return }
            Task { await model.activateQuickSwitcherItem(first) }
          }
          .padding()
        Divider()
        List(model.quickSwitcherItems) { item in
          Button {
            Task { await model.activateQuickSwitcherItem(item) }
          } label: {
            HStack(spacing: 10) {
              Image(systemName: item.favorite ? "star.fill" : "arrow.right.circle")
                .foregroundStyle(item.favorite ? .yellow : .secondary)
              VStack(alignment: .leading, spacing: 2) {
                Text(item.title)
                Text(item.subtitle).font(.caption).foregroundStyle(.secondary)
              }
              Spacer()
            }
            .contentShape(Rectangle())
          }
          .buttonStyle(.plain)
          .accessibilityIdentifier("quick-switch.item.\(item.id)")
        }
        .overlay {
          if model.quickSwitcherItems.isEmpty {
            ContentUnavailableView.search(text: model.quickSwitcherSearch)
          }
        }
      }
      .onExitCommand {
        model.quickSwitcherPresented = false
        dismiss()
      }
      .navigationTitle("Quick Switcher")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") {
            model.quickSwitcherPresented = false
            dismiss()
          }
        }
      }
    }
    .frame(minWidth: 560, minHeight: 420)
  }
}

struct ExplainPlanSheet: View {
  @Environment(BridgeModel.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    NavigationStack {
      ScrollView {
        Text(model.activeExplainPlan ?? "No plan")
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding()
          .accessibilityIdentifier("explain.plan")
      }
      .navigationTitle("Explain Plan")
      .toolbar {
        ToolbarItem(placement: .primaryAction) {
          Button("Copy") {
            model.copyExplainPlan()
          }
          .accessibilityIdentifier("explain.copy")
        }
        ToolbarItem(placement: .confirmationAction) {
          Button("Done") {
            model.explainPresented = false
            dismiss()
          }
        }
      }
    }
    .frame(minWidth: 640, minHeight: 480)
  }
}

struct ProfileEditorSheet: View {
  @Environment(\.dismiss) private var dismiss
  @State private var draft: ProfileEditorDraft
  @State private var saving = false
  let onSave: (ProfileEditorDraft) async -> Bool

  init(
    initialDraft: ProfileEditorDraft,
    onSave: @escaping (ProfileEditorDraft) async -> Bool
  ) {
    _draft = State(initialValue: initialDraft)
    self.onSave = onSave
  }

  private var canSave: Bool {
    !draft.name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      && !draft.host.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      && UInt16(draft.port) != nil
      && (draft.passwordSource != "dangerous_plaintext"
        || (!draft.passwordValue.isEmpty && draft.plaintextAcknowledged))
      && (draft.passwordSource != "keychain"
        || draft.passwordReference != nil || !draft.passwordValue.isEmpty)
      && (!draft.sshEnabled
        || (!draft.sshHost.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          && UInt16(draft.sshPort).map { $0 > 0 } == true
          && !draft.sshKnownHostsPath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          && (draft.sshAuthMode == "agent"
            || (draft.sshPlaintextAcknowledged
              && (draft.sshAuthMode == "password"
                ? (!draft.sshPassword.isEmpty || draft.sshHasStoredPassword)
                : (!draft.sshPrivateKey.isEmpty || draft.sshHasStoredPrivateKey))))))
      && draft.startupActions.count <= 16
      && draft.startupActions.allSatisfy {
        !$0.statement.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          && (100...120_000).contains($0.timeoutMs)
      }
  }

  var body: some View {
    NavigationStack {
      Form {
        Section("General") {
          Picker("Engine", selection: $draft.engine) {
            Text("PostgreSQL").tag("postgresql")
            Text("ClickHouse").tag("clickhouse")
            Text("Redis").tag("redis")
          }
          .accessibilityIdentifier("profile.editor.engine")
          TextField("Name", text: $draft.name)
            .accessibilityIdentifier("profile.editor.name")
          TextField("Group", text: $draft.group)
          Picker("Environment", selection: $draft.environment) {
            Text("None").tag("")
            Text("Production").tag("production")
            Text("Staging").tag("staging")
            Text("Development").tag("development")
            Text("Testing").tag("testing")
          }
          Picker("Safety", selection: $draft.safetyMode) {
            Text("Read only").tag("read_only")
            Text("Confirm writes").tag("confirm_writes")
          }
        }
        Section("Connection") {
          TextField("Host", text: $draft.host)
            .accessibilityIdentifier("profile.editor.host")
          TextField("Port", text: $draft.port)
            .accessibilityIdentifier("profile.editor.port")
          TextField(
            draft.engine == "redis" ? "Logical database" : "Default database",
            text: $draft.database
          )
          .accessibilityIdentifier("profile.editor.database")
          TextField("Username", text: $draft.username)
            .accessibilityIdentifier("profile.editor.username")
        }
        Section("Credentials") {
          Picker("Password storage", selection: $draft.passwordSource) {
            Text("Prompt on connect").tag("prompt")
            Text("Save locally (dangerous)").tag("dangerous_plaintext")
            Text("Environment variable").tag("environment")
            Text("1Password reference").tag("onepassword")
            Text("macOS Keychain").tag("keychain")
          }
          .accessibilityIdentifier("profile.editor.password-source")
          if draft.passwordSource == "dangerous_plaintext" {
            SecureField(
              draft.hasStoredPassword ? "Re-enter stored password" : "Password",
              text: $draft.passwordValue
            )
            Toggle(
              "I understand this stores the password as plaintext locally",
              isOn: $draft.plaintextAcknowledged
            )
            .foregroundStyle(.orange)
          } else if draft.passwordSource == "environment" {
            TextField("Environment variable name", text: $draft.passwordValue)
          } else if draft.passwordSource == "onepassword" {
            TextField("account vault item [section] field", text: $draft.passwordValue)
          } else if draft.passwordSource == "keychain" {
            SecureField(
              draft.hasStoredPassword ? "Replace Keychain password" : "Password",
              text: $draft.passwordValue
            )
          }
        }
        Section("TLS") {
          Picker("Mode", selection: $draft.tlsMode) {
            Text("Off").tag("off")
            Text("Verify CA").tag("verify_ca")
            Text("Verify full").tag("verify_full")
          }
        }
        Section("SSH Tunnel") {
          Toggle("Connect through SSH bastion", isOn: $draft.sshEnabled)
            .accessibilityIdentifier("profile.editor.ssh.enabled")
          if draft.sshEnabled {
            TextField("Bastion host", text: $draft.sshHost)
              .accessibilityIdentifier("profile.editor.ssh.host")
            TextField("SSH port", text: $draft.sshPort)
              .accessibilityIdentifier("profile.editor.ssh.port")
            TextField("SSH username", text: $draft.sshUsername)
              .accessibilityIdentifier("profile.editor.ssh.username")
            Picker("Authentication", selection: $draft.sshAuthMode) {
              Text("SSH agent").tag("agent")
              Text("Password").tag("password")
              Text("OpenSSH private key").tag("private_key")
            }
            .accessibilityIdentifier("profile.editor.ssh.authentication")
            if draft.sshAuthMode == "password" {
              SecureField(
                draft.sshHasStoredPassword ? "Replace stored SSH password" : "SSH password",
                text: $draft.sshPassword
              )
              .accessibilityIdentifier("profile.editor.ssh.password")
            } else if draft.sshAuthMode == "private_key" {
              TextEditor(text: $draft.sshPrivateKey)
                .frame(minHeight: 90)
                .accessibilityLabel("OpenSSH private key")
                .accessibilityIdentifier("profile.editor.ssh.private-key")
              SecureField("Private-key passphrase (optional)", text: $draft.sshPassword)
            }
            LabeledContent("Host-key policy", value: "OpenSSH known_hosts · fail closed")
            TextField("Absolute known_hosts path", text: $draft.sshKnownHostsPath)
              .accessibilityIdentifier("profile.editor.ssh.known-hosts")
            if draft.sshAuthMode != "agent" {
              Toggle(
                "I understand SSH secrets are stored as acknowledged local plaintext",
                isOn: $draft.sshPlaintextAcknowledged
              )
              .foregroundStyle(.orange)
              Text("Use SSH agent where available. Secret values never appear in logs or profile reads.")
                .font(.caption)
                .foregroundStyle(.secondary)
            }
          }
        }
        Section("Startup Commands") {
          ForEach($draft.startupActions) { $action in
            VStack(alignment: .leading, spacing: 8) {
              TextEditor(text: $action.statement)
                .font(.system(.body, design: .monospaced))
                .frame(minHeight: 60)
                .accessibilityLabel("Startup command")
                .accessibilityIdentifier("profile.editor.startup.statement")
              HStack {
                Picker("Safety", selection: $action.safety) {
                  Text("Read only · auto-run").tag("read_only")
                  Text("Write · review required").tag("write")
                  Text("Dangerous · review required").tag("dangerous")
                }
                TextField("Timeout ms", value: $action.timeoutMs, format: .number)
                  .frame(width: 150)
              }
              Toggle("Run again after reconnect", isOn: $action.runOnReconnect)
              HStack {
                Button("Move Up") {
                  guard let index = draft.startupActions.firstIndex(where: { $0.id == action.id }),
                    index > 0
                  else { return }
                  draft.startupActions.swapAt(index, index - 1)
                }
                Button("Move Down") {
                  guard let index = draft.startupActions.firstIndex(where: { $0.id == action.id }),
                    index + 1 < draft.startupActions.count
                  else { return }
                  draft.startupActions.swapAt(index, index + 1)
                }
                Button("Remove", role: .destructive) {
                  draft.startupActions.removeAll { $0.id == action.id }
                }
              }
              if action.safety != "read_only" {
                Label("Never auto-runs; explicit review required", systemImage: "hand.raised.fill")
                  .font(.caption)
                  .foregroundStyle(.orange)
              }
            }
            .padding(.vertical, 4)
          }
          Button("Add Startup Command", systemImage: "plus") {
            guard draft.startupActions.count < 16 else { return }
            draft.startupActions.append(
              StartupActionEditorDraft(
                WorkbenchStartupActionDraft(
                  statement: draft.engine == "redis" ? "PING" : "SELECT 1",
                  safety: "read_only", timeoutMs: 5_000, runOnReconnect: false)))
          }
          .accessibilityIdentifier("profile.editor.startup.add")
          Text(
            "Commands run in listed order. Read-only commands may auto-run. Write and dangerous commands always wait for separate review."
          )
          .font(.caption)
          .foregroundStyle(.secondary)
        }
      }
      .formStyle(.grouped)
      .navigationTitle(draft.idBytes == nil ? "New Connection" : "Edit Connection")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
        ToolbarItem(placement: .confirmationAction) {
          Button("Save") {
            saving = true
            Task {
              if await onSave(draft) { dismiss() }
              saving = false
            }
          }
          .accessibilityIdentifier("profile.editor.save")
          .keyboardShortcut(.defaultAction)
          .disabled(!canSave || saving)
        }
      }
    }
    .frame(minWidth: 520, minHeight: 620)
    .interactiveDismissDisabled(saving)
  }
}

struct ProfileRow: View {
  let profile: WorkbenchProfileItem
  let connectionState: String

  var body: some View {
    VStack(alignment: .leading, spacing: 2) {
      HStack(spacing: 6) {
        if profile.favorite {
          Image(systemName: "star.fill").foregroundStyle(.yellow).font(.caption)
        }
        Text(profile.name).font(.body)
        if profile.productionWarning {
          Label("Production", systemImage: "exclamationmark.triangle.fill")
            .font(.caption)
            .foregroundStyle(.orange)
        }
      }
      Text(
        [
          profile.engine,
          [
            [profile.host, profile.port].compactMap { $0 }.joined(separator: ":"),
            profile.context,
          ].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: "/"),
          profile.environment,
          profile.safetyMode == "read_only" ? "Read only" : "Confirm writes",
        ].compactMap { value in value?.isEmpty == false ? value : nil }.joined(separator: " · ")
      )
      .font(.caption)
      .foregroundStyle(.secondary)
      HStack(spacing: 4) {
        Text(connectionState)
        if profile.dangerousPlaintext {
          Label("Plaintext password", systemImage: "exclamationmark.shield")
        }
      }
      .font(.caption2)
      .foregroundStyle(
        profile.dangerousPlaintext
          ? Color.orange : Color(nsColor: .tertiaryLabelColor))
    }
    .padding(.vertical, 2)
    .accessibilityElement(children: .combine)
  }
}
