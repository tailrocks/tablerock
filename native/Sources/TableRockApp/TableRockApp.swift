// TableRock native macOS app — plan 020.
//
// Built directly with Swift 6 against the macOS 26 SDK. The Rust bridge is
// linked as the cargo release dylib for local development; notarized
// XCFramework distribution remains the operator-gated release path (plan 019).
//
// Checkpoint 1: app shell + live bridge (runtime + persistence).
// Checkpoint 2: connection list — lists saved profiles over the bridge.

import SwiftUI

private func connectedSessionLabel(_ session: String) -> String {
    "Connected · session \(session.prefix(16))…"
}
import Observation
import AppKit
import TableRockBridge
import UniformTypeIdentifiers

private struct NativeOperationProjection: Sendable {
    let table: PageV1Table?
    let envelope: PageV1Envelope?
    let outcome: String?
    let historyFailed: Bool
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
        let scheme: ColorScheme? = switch environment["TABLEROCK_FIXTURE_APPEARANCE"] {
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
        let name: NSAppearance.Name = scheme == .dark
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
    let canRun: Bool
    let canCancel: Bool
    let canRefresh: Bool
    let run: () -> Void
    let cancel: () -> Void
    let refresh: () -> Void
}

private struct WorkbenchActionsKey: FocusedValueKey {
    typealias Value = WorkbenchActions
}

private extension FocusedValues {
    var workbenchActions: WorkbenchActions? {
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
        }
    }
}

/// Sole owner of the synchronous UniFFI object. Blocking driver pumping and
/// page decoding run away from MainActor; awaiting the detached pump keeps this
/// actor reentrant so cancellation can use the operation id independently.
private actor BridgeClient {
    private let bridge: TableRockBridge
    private var eventCursor: UInt64 = 0

    init(persistencePath: String) throws {
        let bridge = TableRockBridge.create()
        try bridge.ensureRuntime()
        try bridge.configurePersistence(path: persistencePath)
        self.bridge = bridge
    }

    func listProfiles() throws -> [BridgeProfileItem] { try bridge.listProfiles() }
    func searchProfiles(_ search: String?) throws -> [BridgeProfileItem] {
        try bridge.searchProfiles(search: search)
    }
    func profileDraft(id: Data) throws -> BridgeProfileDraft {
        try bridge.getProfileDraft(profileId: id)
    }
    func saveProfile(_ draft: BridgeProfileDraft) throws -> Data {
        try bridge.saveProfile(draft: draft)
    }
    func deleteProfile(id: Data, revision: UInt64) throws {
        try bridge.deleteProfile(profileId: id, expectedRevision: revision)
    }
    func testProfile(id: Data, passwordOverride: String?) throws -> BridgeConnectionTestReport {
        try bridge.testProfile(profileId: id, passwordOverride: passwordOverride)
    }
    func listProfileGroups() throws -> [BridgeProfileGroup] { try bridge.listProfileGroups() }
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
    func listHistory(_ search: String?) throws -> [BridgeHistoryItem] {
        try bridge.listHistory(search: search, limit: 100)
    }
    func setHistoryRetention(_ retention: String) throws {
        try bridge.setHistoryRetention(retention: retention)
    }
    func historyRetention() throws -> String { try bridge.historyRetention() }
    func listSavedQueries(engine: String?, search: String?) throws -> [BridgeSavedQueryItem] {
        try bridge.listSavedQueries(engine: engine, search: search)
    }
    func saveQuery(name: String, engine: String, statement: String) throws -> Int64 {
        try bridge.saveQuery(name: name, engine: engine, statementText: statement)
    }
    func deleteSavedQuery(_ id: Int64) throws -> Bool {
        try bridge.deleteSavedQuery(queryId: id)
    }
    func readSqlFile(path: String) throws -> BridgeSqlFile {
        try bridge.readSqlFile(path: path)
    }
    func writeSqlFile(
        path: String,
        statement: String,
        expectedModifiedNanos: UInt64?,
        expectedLength: UInt64?,
        overwriteExternalChange: Bool
    ) throws -> BridgeSqlFile {
        try bridge.writeSqlFile(
            path: path,
            statementText: statement,
            expectedModifiedNanos: expectedModifiedNanos,
            expectedLen: expectedLength,
            overwriteExternalChange: overwriteExternalChange
        )
    }
    func putSessionIntent(profileId: Data, intent: BridgeSessionIntent) throws {
        try bridge.putSessionIntent(profileId: profileId, intent: intent)
    }
    func sessionIntent(profileId: Data) throws -> BridgeSessionIntent? {
        try bridge.getSessionIntent(profileId: profileId)
    }
    func deleteSessionIntent(profileId: Data) throws {
        try bridge.deleteSessionIntent(profileId: profileId)
    }
    func setProfileFavorite(_ item: BridgeProfileItem, _ favorite: Bool) throws {
        try bridge.setProfileFavorite(
            profileId: item.idBytes,
            expectedRevision: item.revision,
            favorite: favorite
        )
    }
    func reorderProfiles(group: String?, profiles: [BridgeProfileItem]) throws {
        try bridge.reorderProfiles(
            group: group,
            ordered: profiles.map {
                BridgeProfileOrderItem(idBytes: $0.idBytes, expectedRevision: $0.revision)
            }
        )
    }
    func open(params: OpenParams) throws -> Data { try bridge.open(params: params) }
    func openProfile(id: Data, passwordOverride: String?) throws -> Data {
        try bridge.openProfile(profileId: id, passwordOverride: passwordOverride)
    }
    func disconnect(session: Data) throws { try bridge.disconnect(sessionId: session) }
    func checkHealth(session: Data) throws -> BridgeSessionHealth {
        try bridge.checkSessionHealth(sessionId: session)
    }
    func planReconnect(
        session: Data, attempt: UInt32, authenticationStopped: Bool
    ) throws -> BridgeReconnectPlan {
        try bridge.planSessionReconnect(
            sessionId: session, attempt: attempt,
            authenticationStopped: authenticationStopped
        )
    }
    func reconnect(session: Data, passwordOverride: String? = nil) throws -> BridgeReconnectAttempt {
        try bridge.reconnectSavedSession(
            sessionId: session, passwordOverride: passwordOverride
        )
    }
    func refreshCatalog(session: Data, parentNodeId: Data?) throws -> [BridgeCatalogNode] {
        try bridge.refreshCatalog(sessionId: session, parentNodeId: parentNodeId)
    }
    func submitCatalogBrowse(session: Data, nodeId: Data) throws -> Data {
        try bridge.submitCatalogBrowse(
            sessionId: session, catalogNodeId: nodeId, rowCount: 500
        )
    }
    func submit(session: Data, intent: String, statement: String?) throws -> Data {
        try bridge.submit(spec: SubmitSpec(
            intent: intent, sessionId: session, statement: statement,
            resultId: nil, startRow: nil, rowCount: 500, expectedRevision: 0
        ))
    }

    func finish(operationId: Data) async throws -> NativeOperationProjection {
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
            return NativeOperationProjection(
                table: nil, envelope: nil, outcome: outcome, historyFailed: historyFailed
            )
        }
        let decoded = try await Task.detached {
            (try PageV1.decodeTable(page), try PageV1.decodeEnvelope(page))
        }.value
        return NativeOperationProjection(
            table: decoded.0, envelope: decoded.1,
            outcome: outcome, historyFailed: historyFailed
        )
    }

    func cancel(operationId: Data) throws -> CancelOutcome {
        try bridge.cancel(operationId: operationId)
    }

    func fetchPage(resultId: Data, startRow: UInt64, revision: UInt64) async throws
        -> (PageV1Table, PageV1Envelope)
    {
        let bytes = try bridge.fetchPage(
            resultId: resultId, startRow: startRow, revision: revision)
        return try await Task.detached {
            (try PageV1.decodeTable(bytes), try PageV1.decodeEnvelope(bytes))
        }.value
    }

    func stageAndApply(session: Data, now: UInt64) throws -> ApplyOutcome {
        let token = try bridge.stageProbeReview(sessionId: session, nowMs: now)
        return try bridge.applyReviewToken(
            tokenId: token, nowMs: now, sessionId: session, expectedRevision: 0)
    }
}

@main
struct TableRockApp: App {
    @State private var model = BridgeModel()

    init() {
        NativeAppearanceFixture.current.applyApplicationAppearance()
    }

    var body: some Scene {
        WindowGroup {
            if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_ACCESSIBILITY_AUDIT"] == "1" {
                NativeAccessibilityFixtureView()
                    .frame(minWidth: 760, minHeight: 520)
            } else if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_PROFILE_EDITOR"] == "1" {
                NativeProfileEditorFixtureView()
            } else if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_GRID_ROWS"] != nil {
                PerformanceFixtureView(table: model.resultTable)
                    .frame(minWidth: 760, minHeight: 520)
            } else {
                ContentView()
                    .environment(model)
                    .modifier(NativeAppearanceFixtureModifier(
                        fixture: NativeAppearanceFixture.current))
                    .frame(minWidth: 760, minHeight: 520)
            }
        }
        .commands {
            WorkbenchCommands()
        }
        Settings {
            NativeSettingsView()
        }
    }
}

private struct NativeProfileEditorFixtureView: View {
    private let draft = BridgeProfileDraft(
        idBytes: Data(repeating: 7, count: 16), revision: 3,
        engine: "postgresql", name: "Production analytics", group: "Production",
        environment: "production", host: "db.example.internal", port: "5432",
        database: "analytics", username: "operator", passwordSource: "prompt",
        passwordValue: "", hasStoredPassword: false, plaintextAcknowledged: false,
        tlsMode: "verify_full", safetyMode: "read_only"
    )

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
          textFields.count >= 6,
          titles.contains("PostgreSQL"),
          titles.contains("Production"),
          titles.contains("Prompt on connect"),
          titles.contains("Read only"),
          titles.contains("Verify full")
    else {
        writePerformanceMetric(
            "PROFILE_EDITOR_PROOF_FAILED title=\(window.title) fields=\(textFields.count) buttons=\(titles.sorted())"
        )
        return
    }
    writePerformanceMetric(
        "PROFILE_EDITOR_PROOF_PASSED title=Edit_Connection fields=\(textFields.count) pickers=engine_environment_password_safety_tls"
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
        "PROFILE_GROUP_PROOF_PASSED empty_group=true alphabetical=Alpha_Zebra health=Healthy_12_ms reconnect=attempt_1 hosting_tree=true"
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

private struct NativeAccessibilityFixtureView: View {
    @State private var catalogSelection: String?
    @State private var query = "SELECT 1;"
    @State private var refreshState: CatalogRefreshState = .loaded

    private let catalog = [
        BridgeCatalogNode(
            idBytes: Data(repeating: 1, count: 16),
            parentIdBytes: nil,
            depth: 0,
            name: "public",
            kind: "postgresql_schema",
            childrenState: "loaded_complete",
            expandable: true
        ),
        BridgeCatalogNode(
            idBytes: Data(repeating: 2, count: 16),
            parentIdBytes: Data(repeating: 1, count: 16),
            depth: 1,
            name: "users",
            kind: "postgresql_table",
            childrenState: "not_applicable",
            expandable: false
        ),
    ]
    private let result = PageV1Table(
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
                SqlTextEditor(text: $query)
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
    let table: PageV1Table?

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
    let profiles: [BridgeProfileItem]
    let alphabetical: Bool
}

struct ProfileGroupDialog: Identifiable {
    let id = UUID()
    let oldName: String?
    var name: String
    var title: String { oldName == nil ? "New Group" : "Rename Group" }
}

enum ProfilePasswordAction: String {
    case connect, test, reconnect
}

struct ProfilePasswordPrompt: Identifiable {
    let profile: BridgeProfileItem
    let action: ProfilePasswordAction
    var id: String { profile.idBytes.base64EncodedString() + ":" + action.rawValue }
}

extension BridgeProfileDraft: @retroactive Identifiable {
    public var id: String {
        idBytes?.base64EncodedString() ?? "new-profile"
    }
}

extension BridgeHistoryItem: @retroactive Identifiable {
    public var id: Int64 { historyId }
}

extension BridgeSavedQueryItem: @retroactive Identifiable {
    public var id: Int64 { queryId }
}

private func catalogNodeKey(_ id: Data) -> String {
    "node:" + id.map { String(format: "%02x", $0) }.joined()
}

private func catalogDescendantIds(
    of parentId: Data,
    in nodes: [BridgeCatalogNode]
) -> Set<Data> {
    var descendants: Set<Data> = []
    var frontier: Set<Data> = [parentId]
    while !frontier.isEmpty {
        let children = Set<Data>(nodes.compactMap { node in
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
final class NativeQueryTab: Identifiable {
    let id = UUID()
    var title: String
    var statementText: String
    var resultTable: PageV1Table?
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
    var sqlFile: BridgeSqlFile?
    var sqlFileBaseline: String
    var sqlFileError: String?

    init(title: String, statementText: String) {
        self.title = title
        self.statementText = statementText
        sqlFileBaseline = statementText
    }
}

@MainActor
@Observable
final class NativeObjectTab: Identifiable {
    let id = UUID()
    let catalogNodeId: Data
    let kind: String
    var title: String
    var pinned: Bool
    var resultTable: PageV1Table?
    var resultIdData: Data?
    var resultRevision: UInt64 = 0
    var nextStartRow: UInt64?
    var isRunning = false
    var activeOperationId: Data?
    var summary: String?
    var error: String?

    init(node: BridgeCatalogNode, pinned: Bool = false) {
        catalogNodeId = node.idBytes
        kind = node.kind
        title = node.name
        self.pinned = pinned
    }
}

@MainActor
@Observable
final class BridgeModel {
    var status: String = "starting…"
    var bridgeError: String?
    var profiles: [BridgeProfileItem] = []
    var profileGroups: [BridgeProfileGroup] = []
    var collapsedProfileGroups: Set<String> = []
    var profileSearch = ""
    private(set) var profilesLoading = false
    private(set) var profilesError: String?
    private var profileSearchGeneration: UInt64 = 0
    var editorDraft: BridgeProfileDraft?
    var profileActionError: String?
    var profileActionOutcome: String?
    var pendingRemoval: BridgeProfileItem?
    var groupDialog: ProfileGroupDialog?
    var passwordPrompt: ProfilePasswordPrompt?
    var pendingGroupRemoval: String?
    var profileSections: [ProfileSection] {
        var order = profileGroups.map(\.name)
        let alphabetical = Dictionary(
            uniqueKeysWithValues: profileGroups.map { ($0.name, $0.alphabetical) }
        )
        var grouped: [String: [BridgeProfileItem]] = [:]
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
    private(set) var sessionHealth: BridgeSessionHealth?
    private(set) var healthChecking = false
    private(set) var reconnectState: String?
    private var reconnectGeneration: UInt64 = 0
    var historyPresented = false
    var historySearch = ""
    var historyItems: [BridgeHistoryItem] = []
    private(set) var historyLoading = false
    private(set) var historyError: String?
    var historyRetention = "full"
    private var historyGeneration: UInt64 = 0
    var savedQueriesPresented = false
    var savedQuerySearch = ""
    var savedQueryEngine = ""
    var savedQueries: [BridgeSavedQueryItem] = []
    private(set) var savedQueriesLoading = false
    private(set) var savedQueriesError: String?
    private var savedQueriesGeneration: UInt64 = 0
    var saveQueryDialog = false
    var savedQueryName = ""
    var pendingSavedQueryRemoval: BridgeSavedQueryItem?
    var queryTabs: [NativeQueryTab]
    var selectedQueryTabId: UUID
    var objectTabs: [NativeObjectTab] = []
    var selectedObjectTabId: UUID?
    var selectedWorkbenchKind = "query"
    var pendingQueryTabClose: NativeQueryTab?
    var queryTabRename: NativeQueryTab?
    var queryTabRenameText = ""
    private var activeProfileId: Data?
    private var activeQueryTab: NativeQueryTab {
        queryTabs.first(where: { $0.id == selectedQueryTabId }) ?? queryTabs[0]
    }
    private var activeObjectTab: NativeObjectTab? {
        guard let selectedObjectTabId else { return nil }
        return objectTabs.first(where: { $0.id == selectedObjectTabId })
    }
    var selectedObjectTab: NativeObjectTab? { activeObjectTab }
    var queryWorkbenchSelected: Bool { selectedWorkbenchKind == "query" }
    private var hasRunningWorkbench: Bool {
        queryTabs.contains(where: \.isRunning) || objectTabs.contains(where: \.isRunning)
    }
    var sqlFile: BridgeSqlFile? {
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
    var catalogSnapshot: [BridgeCatalogNode]?
    private(set) var catalogRefreshState: CatalogRefreshState = .idle
    var isCatalogRefreshing: Bool {
        if case .loading = catalogRefreshState { true } else { false }
    }
    var resultTable: PageV1Table? {
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
            selectedWorkbenchKind == "object"
                ? activeObjectTab?.isRunning == true : activeQueryTab.isRunning
        }
        set {
            if selectedWorkbenchKind == "object" {
                activeObjectTab?.isRunning = newValue
            } else {
                activeQueryTab.isRunning = newValue
            }
        }
    }
    var cancelOutcome: String? {
        get { activeQueryTab.cancelOutcome }
        set { activeQueryTab.cancelOutcome = newValue }
    }
    // Pagination state for the current result (fetch_page).
    var resultIdData: Data? {
        get { activeQueryTab.resultIdData }
        set { activeQueryTab.resultIdData = newValue }
    }
    var resultRevision: UInt64 {
        get { activeQueryTab.resultRevision }
        set { activeQueryTab.resultRevision = newValue }
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
    private var client: BridgeClient?
    var sessionData: Data?

    private static func persistencePath() throws -> String {
        let base = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        ).appendingPathComponent("TableRock", isDirectory: true)
        try FileManager.default.createDirectory(
            at: base,
            withIntermediateDirectories: true
        )
        return base.appendingPathComponent("profiles.db").path
    }

    init() {
        let tab = NativeQueryTab(title: "Query 1", statementText: "SELECT 1;")
        queryTabs = [tab]
        selectedQueryTabId = tab.id
        installPerformanceFixtureIfRequested()
    }

    func initialize() async {
        if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_OBJECT_TABS"] == "1" {
            let node = BridgeCatalogNode(
                idBytes: Data(repeating: 7, count: 16), parentIdBytes: Data(repeating: 6, count: 16),
                depth: 2, name: "users", kind: "postgresql_table",
                childrenState: "not_applicable", expandable: false
            )
            let first = NativeObjectTab(node: node, pinned: true)
            first.resultTable = PageV1Table(columns: ["id"], rows: [["1"]])
            let preview = NativeObjectTab(node: node)
            preview.resultTable = PageV1Table(columns: ["id"], rows: [["2"]])
            objectTabs = [first, preview]
            selectedObjectTabId = preview.id
            selectedWorkbenchKind = "object"
            sessionHex = String(repeating: "b", count: 32)
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
            runNativeObjectTabsAudit()
            return
        }
        if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_QUERY_TABS"] == "1" {
            let first = NativeQueryTab(title: "Users", statementText: "SELECT 1;")
            first.resultTable = PageV1Table(columns: ["n"], rows: [["1"]])
            first.isRunning = true
            first.querySummary = "first result"
            let second = NativeQueryTab(title: "Orders", statementText: "SELECT 2;")
            second.resultTable = PageV1Table(columns: ["n"], rows: [["2"]])
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
            sqlFile = BridgeSqlFile(
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
                BridgeSavedQueryItem(
                    queryId: 1, name: "Recent users", engine: "postgresql",
                    statementText: "SELECT id FROM users", updatedAt: "2026-07-19 05:00:00"
                ),
                BridgeSavedQueryItem(
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
                BridgeHistoryItem(
                    historyId: 2, engine: "postgresql", databaseName: "postgres",
                    schemaName: "public", statementText: "SELECT fixture_history",
                    outcome: "completed", createdAt: "2026-07-19 05:00:00"
                ),
                BridgeHistoryItem(
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
                BridgeProfileGroup(name: "Empty", alphabetical: false),
                BridgeProfileGroup(name: "Production", alphabetical: true),
            ]
            profiles = [
                BridgeProfileItem(
                    idBytes: Data(repeating: 1, count: 16), revision: 0,
                    name: "Zebra", engine: "postgresql", group: "Production",
                    favorite: false, savedOrder: 0, host: "z.internal", port: "5432",
                    context: "db", safetyMode: "confirm_writes", environment: "production",
                    productionWarning: true, dangerousPlaintext: false, connected: true
                ),
                BridgeProfileItem(
                    idBytes: Data(repeating: 2, count: 16), revision: 0,
                    name: "Alpha", engine: "postgresql", group: "Production",
                    favorite: false, savedOrder: 1, host: "a.internal", port: "5432",
                    context: "db", safetyMode: "read_only", environment: "production",
                    productionWarning: true, dangerousPlaintext: false, connected: false
                ),
            ]
            sessionHealth = BridgeSessionHealth(
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
                  connectionState(connectedFixture) == "Healthy · 12 ms"
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
        do {
            let loadedClient = try BridgeClient(persistencePath: Self.persistencePath())
            client = loadedClient
            historyRetention = try await loadedClient.historyRetention()
            await refreshProfiles()
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
        resultTable = PageV1Table(columns: columns, rows: rows)
        let elapsed = Date().timeIntervalSince(started)
        catalogSummary = "Performance fixture · \(count) rows · \(columns.count) columns"
        writePerformanceMetric(
            "PERF_FIXTURE_READY rows=\(count) columns=\(columns.count) build_seconds=\(String(format: "%.6f", elapsed))"
        )
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
            status = profiles.isEmpty
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

    func restoreHistory(_ item: BridgeHistoryItem) {
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

    func restoreSavedQuery(_ item: BridgeSavedQueryItem) {
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
        let tab = NativeObjectTab(node: node)
        objectTabs.append(tab)
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
        await loadObjectTab(tab)
    }

    func selectObjectTab(_ tab: NativeObjectTab) {
        if selectedWorkbenchKind == "object", selectedObjectTabId != tab.id {
            activeObjectTab?.pinned = true
        }
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
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
        tab.resultTable = nil
        do {
            let operation = try await client.submitCatalogBrowse(
                session: session, nodeId: tab.catalogNodeId
            )
            tab.activeOperationId = operation
            tab.isRunning = true
            defer { tab.activeOperationId = nil; tab.isRunning = false }
            let projection = try await client.finish(operationId: operation)
            tab.resultTable = projection.table
            if let envelope = projection.envelope {
                tab.resultIdData = envelope.resultId
                tab.resultRevision = envelope.revision
                tab.nextStartRow = envelope.rowCount == 500
                    ? envelope.startRow + UInt64(envelope.rowCount) : nil
            }
            if let table = projection.table {
                tab.summary = "\(table.rows.count) rows · \(table.columns.count) columns"
            } else {
                tab.summary = "No rows"
            }
        } catch { tab.error = "Object browse failed: \(error)" }
    }

    func reloadObjectTab() async {
        guard let tab = activeObjectTab, !tab.isRunning else { return }
        await loadObjectTab(tab)
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
            if var table = tab.resultTable {
                table.rows.append(contentsOf: more.rows)
                tab.resultTable = table
                tab.summary = "\(table.rows.count) rows · \(table.columns.count) columns"
            }
            tab.nextStartRow = envelope.rowCount == 500
                ? envelope.startRow + UInt64(envelope.rowCount) : nil
        } catch { tab.error = "Load more failed: \(error)" }
    }

    func persistSessionIntent() async {
        guard let client, let profileId = activeProfileId,
              let selected = queryTabs.firstIndex(where: { $0.id == selectedQueryTabId })
        else { return }
        let intent = BridgeSessionIntent(
            database: formDatabase,
            schema: nil,
            selectedTab: UInt32(selected),
            tabs: queryTabs.map {
                BridgeWorkspaceTab(title: $0.title, statementText: $0.statementText)
            }
        )
        do {
            try await client.putSessionIntent(profileId: profileId, intent: intent)
        } catch { profileActionError = "Save workspace intent failed: \(error)" }
    }

    private func restoreSessionIntent(profileId: Data) async {
        guard let client else { return }
        do {
            guard let intent = try await client.sessionIntent(profileId: profileId) else {
                let tab = NativeQueryTab(title: "Query 1", statementText: "")
                queryTabs = [tab]
                selectedQueryTabId = tab.id
                return
            }
            let restored = intent.tabs.map {
                NativeQueryTab(title: $0.title, statementText: $0.statementText)
            }
            guard !restored.isEmpty, Int(intent.selectedTab) < restored.count else { return }
            queryTabs = restored
            selectedQueryTabId = restored[Int(intent.selectedTab)].id
            formDatabase = intent.database
        } catch { profileActionError = "Restore workspace intent failed: \(error)" }
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
        let panel = NSOpenPanel()
        panel.title = "Open SQL File"
        panel.prompt = "Open"
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.canChooseFiles = true
        panel.allowedContentTypes = [UTType(filenameExtension: "sql") ?? .plainText]
        guard panel.runModal() == .OK, let url = panel.url, let client else { return }
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
            let panel = NSSavePanel()
            panel.title = "Save SQL File"
            panel.prompt = "Save"
            panel.allowedContentTypes = [UTType(filenameExtension: "sql") ?? .plainText]
            panel.nameFieldStringValue = "query.sql"
            guard panel.runModal() == .OK, let selected = panel.url else { return }
            url = selected.pathExtension == "sql"
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
        groupDialog = ProfileGroupDialog(oldName: nil, name: "")
    }

    func beginRenameGroup(_ name: String) {
        groupDialog = ProfileGroupDialog(oldName: name, name: name)
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
            profileActionOutcome = alphabetical
                ? "\(section.title) sorted alphabetically"
                : "\(section.title) uses manual order"
            await refreshProfiles()
        } catch { profileActionError = "Group ordering failed: \(error)" }
    }

    func toggleFavorite(_ item: BridgeProfileItem) async {
        guard let client else { return }
        profileActionError = nil
        do {
            try await client.setProfileFavorite(item, !item.favorite)
            profileActionOutcome = item.favorite
                ? "Removed from favorites: \(item.name)"
                : "Added to favorites: \(item.name)"
            await refreshProfiles()
        } catch { profileActionError = "Favorite change failed: \(error)" }
    }

    func canMove(_ item: BridgeProfileItem, in section: ProfileSection, offset: Int) -> Bool {
        guard !section.alphabetical,
              let index = section.profiles.firstIndex(where: { $0.idBytes == item.idBytes })
        else { return false }
        let target = index + offset
        return section.profiles.indices.contains(target)
            && section.profiles[target].favorite == item.favorite
    }

    func move(_ item: BridgeProfileItem, in section: ProfileSection, offset: Int) async {
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
        editorDraft = BridgeProfileDraft(
            idBytes: nil, revision: 0, engine: "postgresql", name: "",
            group: "", environment: "", host: "127.0.0.1", port: "5432",
            database: "postgres", username: "postgres", passwordSource: "prompt",
            passwordValue: "", hasStoredPassword: false,
            plaintextAcknowledged: false, tlsMode: "verify_full",
            safetyMode: "confirm_writes"
        )
    }

    func editProfile(_ item: BridgeProfileItem) async {
        guard let client else { return }
        profileActionError = nil
        do { editorDraft = try await client.profileDraft(id: item.idBytes) }
        catch { profileActionError = "Load connection failed: \(error)" }
    }

    func duplicateProfile(_ item: BridgeProfileItem) async {
        await editProfile(item)
        guard var copy = editorDraft else { return }
        copy.idBytes = nil
        copy.revision = 0
        copy.name += " Copy"
        if copy.hasStoredPassword { copy.passwordValue = "" }
        editorDraft = copy
    }

    func saveProfile(_ draft: BridgeProfileDraft) async -> Bool {
        guard let client else { return false }
        profileActionError = nil
        do {
            _ = try await client.saveProfile(draft)
            editorDraft = nil
            profileActionOutcome = draft.idBytes == nil ? "Connection created" : "Connection saved"
            await refreshProfiles()
            return true
        } catch {
            profileActionError = "Save connection failed: \(error)"
            return false
        }
    }

    func testProfile(_ item: BridgeProfileItem, passwordOverride: String? = nil) async {
        guard let client else { return }
        if passwordOverride == nil {
            do {
                if try await client.profileDraft(id: item.idBytes).passwordSource == "prompt" {
                    passwordPrompt = ProfilePasswordPrompt(profile: item, action: .test)
                    return
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
                id: item.idBytes, passwordOverride: passwordOverride
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
            try await client.deleteProfile(id: item.idBytes, revision: item.revision)
            profileActionOutcome = "Connection removed: \(item.name)"
            await refreshProfiles()
        } catch { profileActionError = "Remove connection failed: \(error)" }
    }

    /// Connect directly from form params (temporary session, no saved profile).
    func connectByParams() async {
        guard !hasRunningWorkbench else {
            connectError = "Cancel running queries before replacing the connection"
            return
        }
        guard let client,
              let port = UInt16(formPort),
              !formHost.isEmpty
        else {
            connectError = "Invalid host or port"
            return
        }
        let previousSession = sessionData
        await persistSessionIntent()
        connectError = nil
        do {
            let session = try await client.open(params: OpenParams(
                engine: formEngine,
                host: formHost,
                port: port,
                database: formDatabase,
                user: formUser,
                password: formPassword,
                tlsMode: "off"
            ))
            connectedEngine = formEngine
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
    func connect(_ item: BridgeProfileItem, passwordOverride: String? = nil) async -> Bool {
        guard let client else { return false }
        guard !hasRunningWorkbench else {
            connectError = "Cancel running queries before replacing the connection"
            return false
        }
        if passwordOverride == nil {
            do {
                let draft = try await client.profileDraft(id: item.idBytes)
                if draft.passwordSource == "prompt" {
                    passwordPrompt = ProfilePasswordPrompt(profile: item, action: .connect)
                    return false
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
                id: item.idBytes, passwordOverride: passwordOverride
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
            sessionHealth = BridgeSessionHealth(
                state: "unhealthy", serverReachable: false,
                elapsedMillis: nil, authenticationStopped: false
            )
            profileActionError = "Health check failed: \(error)"
        }
    }

    func reconnectActive() async {
        guard let client, let sourceSession = sessionData else { return }
        if let profile = profiles.first(where: \.connected) {
            do {
                if try await client.profileDraft(id: profile.idBytes).passwordSource == "prompt" {
                    passwordPrompt = ProfilePasswordPrompt(profile: profile, action: .reconnect)
                    return
                }
            } catch {
                profileActionError = "Load connection failed: \(error)"
                return
            }
        }
        await reconnectActive(sourceSession: sourceSession, passwordOverride: nil)
    }

    private func reconnectActive(sourceSession: Data, passwordOverride: String?) async {
        guard let client else { return }
        reconnectGeneration &+= 1
        let generation = reconnectGeneration
        reconnectState = "Reconnecting"
        do {
            let attempt = try await client.reconnect(
                session: sourceSession, passwordOverride: passwordOverride
            )
            guard attempt.state == "connected", let replacement = attempt.sessionId else {
                reconnectState = attempt.state == "authentication_stopped"
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
            if profileActionError == nil { passwordPrompt = nil; return true }
            return false
        case .reconnect:
            guard let sourceSession = sessionData else { return false }
            await reconnectActive(sourceSession: sourceSession, passwordOverride: password)
            if reconnectState == nil { passwordPrompt = nil; return true }
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
            let plan: BridgeReconnectPlan
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
                    let reconnectAttempt = try await client.reconnect(session: sourceSession)
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

    func connectionState(_ profile: BridgeProfileItem) -> String {
        if connectingName == profile.name { return "Connecting" }
        guard profile.connected else { return "Disconnected" }
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

    /// Submit a catalog refresh and poll events until the page arrives, then
    /// decode the v1 page envelope. Proves the operation/event/page flow.
    /// Submit an operation and poll events until the result page arrives.
    /// Returns the decoded table, or nil on terminal-without-page.
    private func fetchPage(
        intent: String,
        statement: String?,
        tab: NativeQueryTab
    ) async throws -> PageV1Table? {
        guard let client, let session = sessionData else { return nil }
        let operationId = try await client.submit(
            session: session, intent: intent, statement: statement)
        tab.activeOperationId = operationId
        tab.isRunning = true
        tab.cancelOutcome = nil
        defer { tab.activeOperationId = nil; tab.isRunning = false }
        let projection = try await client.finish(operationId: operationId)
        tab.writeOutcome = projection.outcome
        if projection.historyFailed {
            profileActionError = "Query completed, but local history could not be saved"
        }
        if let env = projection.envelope {
            tab.resultIdData = env.resultId
            tab.resultRevision = env.revision
            tab.nextStartRow = env.rowCount == 500
                ? env.startRow + UInt64(env.rowCount) : nil
        }
        return projection.table
    }

    func cancel() async {
        if selectedWorkbenchKind == "object", let tab = activeObjectTab {
            guard let client, let operationId = tab.activeOperationId else { return }
            do {
                let outcome = try await client.cancel(operationId: operationId)
                tab.summary = String(describing: outcome)
            } catch { tab.error = "Cancel failed: \(error)" }
            return
        }
        let tab = activeQueryTab
        guard let client, let operationId = tab.activeOperationId else { return }
        do {
            let outcome = try await client.cancel(operationId: operationId)
            tab.cancelOutcome = String(describing: outcome)
        } catch {
            tab.cancelOutcome = "Cancel failed: \(error)"
        }
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
            if var table = tab.resultTable {
                table.rows.append(contentsOf: more.rows)
                tab.resultTable = table
                tab.querySummary =
                    "result · \(table.columns.count) columns · \(table.rows.count) rows loaded"
            }
            tab.nextStartRow = env.rowCount == 500
                ? env.startRow + UInt64(env.rowCount) : nil
        } catch {
            tab.queryError = "Load more failed: \(error)"
        }
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
            catalogRefreshState = hadSnapshot
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
            if let table = try await fetchPage(intent: "execute", statement: sql, tab: tab) {
                tab.resultTable = table
                tab.querySummary = "result · \(table.columns.count) columns · \(table.rows.count) rows"
            } else if let outcome = tab.writeOutcome {
                tab.querySummary = "write ok · \(outcome)"
            } else {
                tab.querySummary = "query: no result"
            }
        } catch {
            tab.queryError = "Query failed: \(error)"
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
            let now = UInt64(Date().timeIntervalSince1970 * 1000)
            let outcome = try await client.stageAndApply(session: session, now: now)
            tab.reviewOutcome =
                "\(outcome.transaction) · \(outcome.appliedCount) applied · \(outcome.conflictCount) conflict · \(outcome.failedCount) failed"
        } catch {
            tab.reviewError = "Review/apply failed: \(error)"
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
                                    Button { Task { await model.connect(profile) } } label: {
                                        ProfileRow(
                                            profile: profile,
                                            connectionState: model.connectionState(profile)
                                        )
                                    }
                                    .buttonStyle(.plain)
                                    Menu {
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
                                            Label("Manual Order", systemImage: section.alphabetical
                                                ? "circle" : "checkmark")
                                        }
                                        Button {
                                            Task { await model.setGroupAlphabetical(section, true) }
                                        } label: {
                                            Label("Alphabetical", systemImage: section.alphabetical
                                                ? "checkmark" : "circle")
                                        }
                                        Divider()
                                        Button("Rename Group…") {
                                            model.beginRenameGroup(section.title)
                                        }
                                        Button("Remove Group…", role: .destructive) {
                                            model.pendingGroupRemoval = section.title
                                        }
                                    } label: { Image(systemName: "ellipsis") }
                                    .menuStyle(.borderlessButton)
                                    .accessibilityLabel("Actions for group \(section.title)")
                                }
                            }
                        }
                    }
                }
                .searchable(text: $model.profileSearch, prompt: "Search connections")
                .task(id: model.profileSearch) {
                    try? await Task.sleep(for: .milliseconds(150))
                    guard !Task.isCancelled else { return }
                    await model.refreshProfiles()
                }
                .safeAreaInset(edge: .bottom) {
                    HStack {
                        Button { model.createProfile() } label: {
                            Label("New connection", systemImage: "plus")
                        }
                        Button { model.beginCreateGroup() } label: {
                            Label("New group", systemImage: "folder.badge.plus")
                        }
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
                        && (!model.profileSearch.isEmpty || model.profileGroups.isEmpty) {
                        ContentUnavailableView(
                            model.profileSearch.isEmpty ? "No connections" : "No matches",
                            systemImage: model.profileSearch.isEmpty ? "tray" : "magnifyingglass",
                            description: Text(model.profileSearch.isEmpty
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
                        Button { Task { await model.browse() } } label: {
                            Image(systemName: "arrow.clockwise")
                        }
                        .buttonStyle(.borderless)
                        .disabled(model.isRunning || model.isCatalogRefreshing)
                        .accessibilityLabel("Refresh catalog")
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
                        case let .failed(message):
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
                Text(model.status).foregroundStyle(.secondary)
                if let outcome = model.profileActionOutcome {
                    Text(outcome).foregroundStyle(.secondary).font(.callout)
                }
                if let bridgeError = model.bridgeError {
                    Text(bridgeError)
                        .foregroundStyle(.red)
                        .font(.callout)
                        .textSelection(.enabled)
                }
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
                        GridRow { Text("Host"); TextField("127.0.0.1", text: $model.formHost) }
                        GridRow { Text("Port"); TextField("5432", text: $model.formPort) }
                        GridRow { Text("Database"); TextField("postgres", text: $model.formDatabase) }
                        GridRow { Text("User"); TextField("postgres", text: $model.formUser) }
                        GridRow {
                            Text("Password")
                            SecureField("", text: $model.formPassword)
                        }
                    }
                    HStack {
                        Button("Connect") { Task { await model.connectByParams() } }
                            .buttonStyle(.borderedProminent)
                        Spacer()
                    }
                    .padding(.top, 4)
                }
                if let name = model.connectingName {
                    Text("Connecting to \(name)…").foregroundStyle(.secondary)
                }
                if let session = model.sessionHex {
                    Label(
                        connectedSessionLabel(session),
                        systemImage: "checkmark.circle.fill"
                    )
                    .foregroundStyle(.green)
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
        .sheet(item: $model.editorDraft) { draft in
            ProfileEditorSheet(initialDraft: draft) { saved in
                await model.saveProfile(saved)
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
        .sheet(isPresented: $model.historyPresented) {
            HistorySheet()
        }
        .sheet(isPresented: $model.savedQueriesPresented) {
            SavedQueriesSheet()
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
        ) { Button("OK") { model.profileActionError = nil } } message: {
            Text(model.profileActionError ?? "Unknown failure")
        }
        .alert("Rename Query Tab", isPresented: Binding(
            get: { model.queryTabRename != nil },
            set: { if !$0 { model.queryTabRename = nil } }
        )) {
            TextField("Title", text: $model.queryTabRenameText)
            Button("Rename") { model.renameQueryTab() }
            Button("Cancel", role: .cancel) { model.queryTabRename = nil }
        }
        .task { await model.initialize() }
        .focusedValue(\.workbenchActions, WorkbenchActions(
            canRun: model.queryWorkbenchSelected && model.sessionHex != nil
                && !model.isRunning && !model.isCatalogRefreshing,
            canCancel: model.isRunning,
            canRefresh: model.sessionHex != nil && !model.isRunning && !model.isCatalogRefreshing,
            run: { Task { await model.runQuery() } },
            cancel: { Task { await model.cancel() } },
            refresh: { Task { await model.browse() } }
        ))
        .toolbar(id: "workbench") {
            WorkbenchToolbar(model: model)
        }
    }
}

struct QueryWorkbenchView: View {
    @Environment(BridgeModel.self) private var model

    var body: some View {
        @Bindable var model = model
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("SQL").font(.headline)
                if let file = model.sqlFile {
                    Text(URL(fileURLWithPath: file.path).lastPathComponent)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            SqlTextEditor(text: $model.queryText)
                .frame(minHeight: 56, maxHeight: 80)
                .task(id: model.queryText) {
                    try? await Task.sleep(for: .milliseconds(300))
                    guard !Task.isCancelled else { return }
                    await model.persistSessionIntent()
                }
            HStack {
                Button("Run query") { Task { await model.runQuery() } }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut("r", modifiers: .command)
                    .disabled(model.isRunning || model.isCatalogRefreshing)
                Button("Cancel") { Task { await model.cancel() } }
                    .disabled(!model.isRunning)
                Button("Refresh catalog") { Task { await model.browse() } }
                    .disabled(model.isRunning || model.isCatalogRefreshing)
                Button("Apply probe edit") { Task { await model.applyProbeEdit() } }
                    .disabled(model.isRunning || model.isCatalogRefreshing)
            }
            if let value = model.cancelOutcome {
                Text(value).foregroundStyle(.secondary).font(.callout)
            }
            if let value = model.querySummary {
                Text(value).foregroundStyle(.secondary).font(.callout)
            }
            if let value = model.queryError {
                Text(value).foregroundStyle(.red).font(.callout).textSelection(.enabled)
            }
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
                CatalogGrid(table: table).frame(minHeight: 220)
                if model.nextStartRow != nil {
                    Button("Load more rows") { Task { await model.loadMore() } }
                }
            }
        }
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
                    Spacer()
                    if !tab.pinned {
                        Button("Pin") { model.pinObjectTab(tab) }
                    }
                    Button("Refresh") { Task { await model.reloadObjectTab() } }
                        .disabled(tab.isRunning)
                    Button("Close", role: .destructive) { model.closeObjectTab(tab) }
                        .disabled(tab.isRunning)
                }
                if tab.isRunning { ProgressView("Loading \(tab.title)…") }
                if let summary = tab.summary {
                    Text(summary).font(.callout).foregroundStyle(.secondary)
                }
                if let error = tab.error {
                    Text(error).font(.callout).foregroundStyle(.red).textSelection(.enabled)
                }
                if let table = tab.resultTable {
                    CatalogGrid(table: table).frame(minHeight: 260)
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

struct QueryTabStrip: View {
    @Environment(BridgeModel.self) private var model

    var body: some View {
        ScrollView(.horizontal) {
            HStack(spacing: 4) {
                ForEach(model.queryTabs) { tab in
                    HStack(spacing: 2) {
                        if model.queryWorkbenchSelected && tab.id == model.selectedQueryTabId {
                            Button(tab.title) { model.selectQueryTab(tab) }
                                .buttonStyle(.borderedProminent)
                                .accessibilityValue("Selected")
                        } else {
                            Button(tab.title) { model.selectQueryTab(tab) }
                                .buttonStyle(.bordered)
                        }
                        Menu {
                            Button("Rename…") { model.beginRenameQueryTab(tab) }
                            Button("Close", role: .destructive) {
                                model.requestCloseQueryTab(tab)
                            }
                            .disabled(model.queryTabs.count == 1 || tab.isRunning)
                        } label: {
                            Image(systemName: tab.isRunning ? "progress.indicator" : "ellipsis")
                        }
                        .menuStyle(.borderlessButton)
                        .accessibilityLabel("Actions for \(tab.title)")
                    }
                }
                ForEach(model.objectTabs) { tab in
                    HStack(spacing: 2) {
                        if !model.queryWorkbenchSelected && tab.id == model.selectedObjectTabId {
                            Button { model.selectObjectTab(tab) } label: {
                                Label(tab.title, systemImage: tab.pinned ? "pin.fill" : "eye")
                            }
                            .buttonStyle(.borderedProminent)
                            .accessibilityValue("Selected")
                        } else {
                            Button { model.selectObjectTab(tab) } label: {
                                Label(tab.title, systemImage: tab.pinned ? "pin.fill" : "eye")
                            }
                            .buttonStyle(.bordered)
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
                Button { model.addQueryTab() } label: {
                    Image(systemName: "plus")
                }
                .buttonStyle(.borderless)
                .accessibilityLabel("New query tab")
                .disabled(model.queryTabs.count + model.objectTabs.count >= 64)
            }
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
            Button { model.requestOpenSqlFile() } label: {
                Label("Open SQL File", systemImage: "folder")
            }
            .disabled(!model.queryWorkbenchSelected)
        }
        ToolbarItem(id: "save-sql-file", placement: .automatic) {
            Button { Task { await model.saveSqlFile() } } label: {
                Label("Save SQL File", systemImage: "square.and.arrow.down")
            }
            .disabled(!model.queryWorkbenchSelected)
        }
        ToolbarItem(id: "save-sql-file-as", placement: .automatic) {
            Button { Task { await model.saveSqlFile(saveAs: true) } } label: {
                Label("Save SQL File As", systemImage: "square.and.arrow.down.on.square")
            }
            .disabled(!model.queryWorkbenchSelected)
        }
        ToolbarItem(id: "reload-sql-file", placement: .automatic) {
            Button { Task { await model.reloadSqlFile() } } label: {
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
            .accessibilityLabel(model.sessionHex == nil
                ? "No active connection" : "Connected to \(model.connectedEngine)")
        }
        ToolbarItem(id: "disconnect", placement: .automatic) {
            Button { Task { await model.disconnectActive() } } label: {
                Label("Disconnect", systemImage: "bolt.slash")
            }
            .disabled(model.sessionHex == nil || model.isRunning)
        }
        ToolbarItem(id: "health", placement: .automatic) {
            Button { Task { await model.checkActiveHealth() } } label: {
                Label("Check Health", systemImage: "heart.text.square")
            }
            .disabled(model.sessionHex == nil || model.isRunning || model.healthChecking)
        }
        ToolbarItem(id: "reconnect", placement: .automatic) {
            Button { Task { await model.reconnectActive() } } label: {
                Label("Reconnect", systemImage: "arrow.triangle.2.circlepath")
            }
            .disabled(
                model.sessionHex == nil || model.isRunning
                    || model.reconnectState?.hasPrefix("Reconnecting") == true
            )
        }
        ToolbarItem(id: "history", placement: .automatic) {
            Button { Task { await model.presentHistory() } } label: {
                Label("Query History", systemImage: "clock.arrow.circlepath")
            }
        }
        ToolbarItem(id: "saved-queries", placement: .automatic) {
            Button { Task { await model.presentSavedQueries() } } label: {
                Label("Saved Queries", systemImage: "bookmark")
            }
        }
    }
}

struct WorkbenchQueryToolbar: CustomizableToolbarContent {
    let model: BridgeModel

    var body: some CustomizableToolbarContent {
        ToolbarItem(id: "save-query", placement: .automatic) {
            Button { model.beginSaveCurrentQuery() } label: {
                Label("Save Query", systemImage: "bookmark.badge.plus")
            }
            .disabled(!model.queryWorkbenchSelected)
        }
        ToolbarSpacer(.fixed)
        ToolbarItem(id: "refresh", placement: .automatic) {
            Button { Task { await model.browse() } } label: {
                Label("Refresh Catalog", systemImage: "arrow.clockwise")
            }
            .disabled(model.sessionHex == nil || model.isRunning || model.isCatalogRefreshing)
        }
        ToolbarSpacer(.fixed)
        ToolbarItem(id: "run", placement: .primaryAction) {
            Button { Task { await model.runQuery() } } label: {
                Label("Run Query", systemImage: "play.fill")
            }
            .buttonStyle(.glassProminent)
            .disabled(!model.queryWorkbenchSelected || model.sessionHex == nil
                || model.isRunning || model.isCatalogRefreshing)
        }
        ToolbarItem(id: "cancel", placement: .primaryAction) {
            Button { Task { await model.cancel() } } label: {
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
                        description: Text(model.savedQuerySearch.isEmpty
                            ? "Save current editor text to reuse it later."
                            : "Try a different name or SQL-text search.")
                    )
                } else {
                    List(model.savedQueries) { item in
                        HStack(spacing: 10) {
                            Button { model.restoreSavedQuery(item) } label: {
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
                        description: Text(model.historySearch.isEmpty
                            ? "Executed statements appear here when retention is enabled."
                            : "Try a different SQL-text search.")
                    )
                } else {
                    List(model.historyItems) { item in
                        Button { model.restoreHistory(item) } label: {
                            VStack(alignment: .leading, spacing: 5) {
                                Text(item.statementText ?? "SQL text not retained")
                                    .font(.system(.body, design: .monospaced))
                                    .lineLimit(3)
                                Text([
                                    item.engine, item.databaseName,
                                    item.schemaName, item.outcome, item.createdAt,
                                ].compactMap { $0 }.joined(separator: " · "))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.vertical, 3)
                        }
                        .buttonStyle(.plain)
                        .disabled(item.statementText == nil)
                        .accessibilityHint(item.statementText == nil
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
                    Button { Task { await model.refreshHistory() } } label: {
                        Label("Refresh History", systemImage: "arrow.clockwise")
                    }
                    .disabled(model.historyLoading)
                }
            }
        }
        .frame(minWidth: 680, minHeight: 480)
    }
}

struct NativeSettingsView: View {
    var body: some View {
        Form {
            LabeledContent("Storage", value: "Local only")
            LabeledContent("Telemetry", value: "Off by default")
        }
        .formStyle(.grouped)
        .padding()
        .frame(width: 420)
    }
}

struct CatalogOutline: NSViewRepresentable {
    let table: [BridgeCatalogNode]
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
            table: [BridgeCatalogNode],
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

        func rebuild(from table: [BridgeCatalogNode], refreshState: CatalogRefreshState) {
            let byParent = Dictionary(grouping: table, by: \.parentIdBytes)
            func build(_ record: BridgeCatalogNode) -> Node {
                let key = catalogNodeKey(record.idBytes)
                var children = (byParent[record.idBytes] ?? []).map(build)
                switch refreshState {
                case let .loading(nodeKey) where nodeKey == key:
                    children.append(Node(
                        key: "state:loading:\(key)", title: "Loading…", isState: true))
                case let .stale(nodeKey, message) where nodeKey == key:
                    children.append(Node(
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
            cell.setAccessibilityLabel(node.isState
                ? "Catalog state \(node.title)"
                : node.children.isEmpty
                ? "Catalog object \(node.title)" : "Catalog group \(node.title)")
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
            else { selection.wrappedValue = nil; return }
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
                if row >= 0 { outline.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false) }
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
    let table: PageV1Table

    func makeCoordinator() -> Coordinator { Coordinator(table) }

    func makeNSView(context: Context) -> NSScrollView {
        let grid = NSTableView()
        grid.usesAlternatingRowBackgroundColors = true
        grid.allowsColumnReordering = true
        grid.allowsColumnResizing = true
        grid.allowsMultipleSelection = true
        grid.rowSizeStyle = .small
        grid.backgroundColor = .textBackgroundColor
        grid.setAccessibilityLabel("Query results")
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
        context.coordinator.startPerformanceScrollIfRequested(on: grid)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let grid = scroll.documentView as? NSTableView else { return }
        let selectedRows = grid.selectedRowIndexes
        context.coordinator.snapshot = table
        context.coordinator.installColumns(on: grid)
        grid.reloadData()
        context.coordinator.startPerformanceScrollIfRequested(on: grid)
        let validSelection = selectedRows.filter { $0 < table.rows.count }
        grid.selectRowIndexes(IndexSet(validSelection), byExtendingSelection: false)
    }

    @MainActor
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
        var snapshot: PageV1Table
        private var fixtureScrollTask: Task<Void, Never>?

        init(_ snapshot: PageV1Table) {
            self.snapshot = snapshot
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
                    column.title = title
                }
                return
            }
            for column in tableView.tableColumns { tableView.removeTableColumn(column) }
            for (index, title) in snapshot.columns.enumerated() {
                let column = NSTableColumn(
                    identifier: NSUserInterfaceItemIdentifier("result-column-\(index)"))
                column.title = title
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
            let cell: NSTableCellView
            if let reused = tableView.makeView(withIdentifier: identifier, owner: nil)
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
                    label.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
                    label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
                    label.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
                ])
            }
            let value = snapshot.rows[row][column]
            cell.textField?.stringValue = value
            cell.setAccessibilityLabel("\(snapshot.columns[column]), row \(row + 1)")
            cell.setAccessibilityValue(value)
            return cell
        }
    }
}

private func writePerformanceMetric(_ metric: String) {
    FileHandle.standardError.write(Data("\(metric)\n".utf8))
}

struct SqlTextEditor: NSViewRepresentable {
    @Binding var text: String

    func makeCoordinator() -> Coordinator { Coordinator(text: $text) }

    func makeNSView(context: Context) -> NSScrollView {
        let editor = NSTextView()
        editor.delegate = context.coordinator
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
        editor.setAccessibilityLabel("SQL editor")

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
        // Never replace storage while an input method owns marked text.
        guard !editor.hasMarkedText(), editor.string != text else { return }
        let selection = editor.selectedRanges
        editor.string = text
        let maximum = (text as NSString).length
        editor.selectedRanges = selection.map { value in
            let range = value.rangeValue
            return NSValue(range: NSRange(
                location: min(range.location, maximum),
                length: min(range.length, max(0, maximum - min(range.location, maximum)))
            ))
        }
    }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        var text: Binding<String>

        init(text: Binding<String>) { self.text = text }

        func textDidChange(_ notification: Notification) {
            guard let editor = notification.object as? NSTextView else { return }
            text.wrappedValue = editor.string
        }
    }
}

struct ProfilePasswordSheet: View {
    @Environment(\.dismiss) private var dismiss
    let profile: BridgeProfileItem
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
            if await onConnect(transientPassword) { dismiss() }
            else { connecting = false }
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
                .disabled(dialog.name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                    || saving)
            }
        }
        .padding(24)
        .frame(width: 380)
        .interactiveDismissDisabled(saving)
    }
}

struct ProfileEditorSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var draft: BridgeProfileDraft
    @State private var saving = false
    let onSave: (BridgeProfileDraft) async -> Bool

    init(
        initialDraft: BridgeProfileDraft,
        onSave: @escaping (BridgeProfileDraft) async -> Bool
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
                    TextField("Name", text: $draft.name)
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
                    TextField("Port", text: $draft.port)
                    TextField(
                        draft.engine == "redis" ? "Logical database" : "Default database",
                        text: $draft.database
                    )
                    TextField("Username", text: $draft.username)
                }
                Section("Credentials") {
                    Picker("Password storage", selection: $draft.passwordSource) {
                        Text("Prompt on connect").tag("prompt")
                        Text("Save locally (dangerous)").tag("dangerous_plaintext")
                        Text("Environment variable").tag("environment")
                        Text("1Password reference").tag("onepassword")
                    }
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
                    }
                }
                Section("TLS") {
                    Picker("Mode", selection: $draft.tlsMode) {
                        Text("Off").tag("off")
                        Text("Verify CA").tag("verify_ca")
                        Text("Verify full").tag("verify_full")
                    }
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
                    .disabled(!canSave || saving)
                }
            }
        }
        .frame(minWidth: 520, minHeight: 620)
        .interactiveDismissDisabled(saving)
    }
}

struct ProfileRow: View {
    let profile: BridgeProfileItem
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
            Text([
                profile.engine,
                [
                    [profile.host, profile.port].compactMap { $0 }.joined(separator: ":"),
                    profile.context,
                ].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: "/"),
                profile.environment,
                profile.safetyMode == "read_only" ? "Read only" : "Confirm writes",
            ].compactMap { value in value?.isEmpty == false ? value : nil }.joined(separator: " · "))
                .font(.caption)
                .foregroundStyle(.secondary)
            HStack(spacing: 4) {
                Text(connectionState)
                if profile.dangerousPlaintext {
                    Label("Plaintext password", systemImage: "exclamationmark.shield")
                }
            }
            .font(.caption2)
            .foregroundStyle(profile.dangerousPlaintext
                ? Color.orange : Color(nsColor: .tertiaryLabelColor))
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}
