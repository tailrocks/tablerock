// TableRock native macOS app — plan 020.
//
// Built directly with Swift 6 against the macOS 26 SDK. The Rust bridge is
// linked as the cargo release dylib for local development; notarized
// XCFramework distribution remains the operator-gated release path (plan 019).
//
// Checkpoint 1: app shell + live bridge (runtime + persistence).
// Checkpoint 2: connection list — lists saved profiles over the bridge.

import SwiftUI
import Observation
import AppKit
import TableRockBridge

private struct NativeOperationProjection: Sendable {
    let table: PageV1Table?
    let envelope: PageV1Envelope?
    let outcome: String?
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
    func testProfile(id: Data) throws -> BridgeConnectionTestReport {
        try bridge.testProfile(profileId: id, passwordOverride: nil)
    }
    func listProfileGroups() throws -> [String] { try bridge.listProfileGroups() }
    func createProfileGroup(_ name: String) throws { try bridge.createProfileGroup(name: name) }
    func renameProfileGroup(_ oldName: String, _ newName: String) throws -> UInt32 {
        try bridge.renameProfileGroup(oldName: oldName, newName: newName)
    }
    func deleteProfileGroup(_ name: String) throws -> UInt32 {
        try bridge.deleteProfileGroup(name: name)
    }
    func open(params: OpenParams) throws -> Data { try bridge.open(params: params) }
    func openProfile(id: Data) throws -> Data {
        try bridge.openProfile(profileId: id, passwordOverride: nil)
    }
    func disconnect(session: Data) throws { try bridge.disconnect(sessionId: session) }
    func refreshCatalog(session: Data, parentNodeId: Data?) throws -> [BridgeCatalogNode] {
        try bridge.refreshCatalog(sessionId: session, parentNodeId: parentNodeId)
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
        for _ in 0..<64 {
            let batch = try bridge.nextEvents(cursor: eventCursor, maximum: 64)
            eventCursor = batch.nextCursor
            for event in batch.events where event.operationId == operationId {
                if event.kind == "page" { page = event.pageBytes }
                if event.kind == "terminal" { outcome = event.outcome ?? "ok" }
            }
            if outcome != nil || batch.events.isEmpty { break }
        }
        guard let page else {
            return NativeOperationProjection(table: nil, envelope: nil, outcome: outcome)
        }
        let decoded = try await Task.detached {
            (try PageV1.decodeTable(page), try PageV1.decodeEnvelope(page))
        }.value
        return NativeOperationProjection(table: decoded.0, envelope: decoded.1, outcome: outcome)
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
        "PROFILE_GROUP_PROOF_PASSED empty_groups=2 hosting_tree=true"
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
                }
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
}

struct ProfileGroupDialog: Identifiable {
    let id = UUID()
    let oldName: String?
    var name: String
    var title: String { oldName == nil ? "New Group" : "Rename Group" }
}

extension BridgeProfileDraft: @retroactive Identifiable {
    public var id: String {
        idBytes?.base64EncodedString() ?? "new-profile"
    }
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
final class BridgeModel {
    var status: String = "starting…"
    var bridgeError: String?
    var profiles: [BridgeProfileItem] = []
    var profileGroups: [String] = []
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
    var pendingGroupRemoval: String?
    var profileSections: [ProfileSection] {
        var order = profileGroups
        var grouped: [String: [BridgeProfileItem]] = [:]
        for profile in profiles {
            let group = profile.group ?? ""
            if !group.isEmpty && !order.contains(group) { order.append(group) }
            grouped[group, default: []].append(profile)
        }
        if grouped[""] != nil { order.append("") }
        if !profileSearch.isEmpty { order.removeAll { grouped[$0]?.isEmpty != false } }
        return order.map { group in
            ProfileSection(
                id: group.isEmpty ? "ungrouped" : group,
                title: group.isEmpty ? "Ungrouped" : group,
                profiles: grouped[group] ?? []
            )
        }
    }
    var sessionHex: String?
    var connectError: String?
    var connectingName: String?
    var catalogSummary: String?
    var catalogError: String?
    var catalogSnapshot: [BridgeCatalogNode]?
    private(set) var catalogRefreshState: CatalogRefreshState = .idle
    var isCatalogRefreshing: Bool {
        if case .loading = catalogRefreshState { true } else { false }
    }
    var resultTable: PageV1Table?
    var catalogSelection: String?
    var writeOutcome: String?
    var isRunning = false
    var cancelOutcome: String?
    // Pagination state for the current result (fetch_page).
    var resultIdData: Data?
    var resultRevision: UInt64 = 0
    var nextStartRow: UInt64?
    var connectedEngine: String = ""
    var queryText: String = "SELECT 1;"
    var reviewOutcome: String?
    var reviewError: String?
    // Direct-connect form (no saved profile required).
    var formEngine: String = "postgresql"
    var formHost: String = "127.0.0.1"
    var formPort: String = "5432"
    var formDatabase: String = "postgres"
    var formUser: String = "postgres"
    var formPassword: String = ""
    private var client: BridgeClient?
    private var activeOperationId: Data?
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
        installPerformanceFixtureIfRequested()
    }

    func initialize() async {
        if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_PROFILE_GROUPS"] == "1" {
            profileGroups = ["Empty", "Production"]
            status = "Profile group fixture"
            guard profileSections.map(\.title) == ["Empty", "Production"],
                  profileSections.allSatisfy({ $0.profiles.isEmpty })
            else {
                writePerformanceMetric("PROFILE_GROUP_PROOF_FAILED group projection mismatch")
                return
            }
            try? await Task.sleep(for: .milliseconds(500))
            runNativeProfileGroupAudit()
            return
        }
        do {
            client = try BridgeClient(persistencePath: Self.persistencePath())
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

    func testProfile(_ item: BridgeProfileItem) async {
        guard let client else { return }
        profileActionError = nil
        profileActionOutcome = "Testing \(item.name)…"
        do {
            let report = try await client.testProfile(id: item.idBytes)
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
        guard let client,
              let port = UInt16(formPort),
              !formHost.isEmpty
        else {
            connectError = "Invalid host or port"
            return
        }
        sessionHex = nil
        sessionData = nil
        connectError = nil
        catalogSummary = nil
        catalogSnapshot = nil
        catalogRefreshState = .idle
        resultTable = nil
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
            sessionData = session
            sessionHex = session.map { String(format: "%02x", $0) }.joined()
        } catch {
            connectError = "Connect failed: \(error)"
        }
    }

    /// Open a saved profile by id (password override nil — inline source only).
    func connect(_ item: BridgeProfileItem) async {
        guard let client else { return }
        connectingName = item.name
        sessionHex = nil
        sessionData = nil
        connectError = nil
        catalogSummary = nil
        catalogError = nil
        catalogSnapshot = nil
        catalogRefreshState = .idle
        resultTable = nil
        do {
            let session = try await client.openProfile(id: item.idBytes)
            connectedEngine = item.engine
            sessionData = session
            sessionHex = session.map { String(format: "%02x", $0) }.joined()
        } catch {
            connectError = "Connect failed: \(error)"
        }
        connectingName = nil
    }

    /// Submit a catalog refresh and poll events until the page arrives, then
    /// decode the v1 page envelope. Proves the operation/event/page flow.
    /// Submit an operation and poll events until the result page arrives.
    /// Returns the decoded table, or nil on terminal-without-page.
    private func fetchPage(intent: String, statement: String?) async throws -> PageV1Table? {
        guard let client, let session = sessionData else { return nil }
        let operationId = try await client.submit(
            session: session, intent: intent, statement: statement)
        activeOperationId = operationId
        isRunning = true
        cancelOutcome = nil
        defer { activeOperationId = nil; isRunning = false }
        let projection = try await client.finish(operationId: operationId)
        writeOutcome = projection.outcome
        if let env = projection.envelope {
            resultIdData = env.resultId
            resultRevision = env.revision
            nextStartRow = env.startRow + UInt64(env.rowCount)
        }
        return projection.table
    }

    func cancel() async {
        guard let client, let operationId = activeOperationId else { return }
        do {
            let outcome = try await client.cancel(operationId: operationId)
            cancelOutcome = String(describing: outcome)
        } catch {
            cancelOutcome = "Cancel failed: \(error)"
        }
    }

    /// Fetch the next page of the current result and append its rows.
    func loadMore() async {
        guard let client, let resultId = resultIdData, let start = nextStartRow else { return }
        do {
            let (more, env) = try await client.fetchPage(
                resultId: resultId, startRow: start, revision: resultRevision)
            if more.rows.isEmpty {
                nextStartRow = nil
                return
            }
            if var table = resultTable {
                table.rows.append(contentsOf: more.rows)
                resultTable = table
                catalogSummary =
                    "result · \(table.columns.count) columns · \(table.rows.count) rows loaded"
            }
            nextStartRow = env.startRow + UInt64(env.rowCount)
        } catch {
            catalogError = "Load more failed: \(error)"
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
        let sql = queryText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !sql.isEmpty else { return }
        catalogSummary = nil
        catalogError = nil
        resultTable = nil
        do {
            if let table = try await fetchPage(intent: "execute", statement: sql) {
                resultTable = table
                catalogSummary = "result · \(table.columns.count) columns · \(table.rows.count) rows"
            } else if let outcome = writeOutcome {
                catalogSummary = "write ok · \(outcome)"
            } else {
                catalogSummary = "query: no result"
            }
        } catch {
            catalogError = "Query failed: \(error)"
        }
    }

    /// Stage a probe mutation, authorize it, and apply it through the single-use
    /// review-token safety gate. Demonstrates the edit/review flow.
    func applyProbeEdit() async {
        guard let client, let session = sessionData else { return }
        reviewOutcome = nil
        reviewError = nil
        do {
            let now = UInt64(Date().timeIntervalSince1970 * 1000)
            let outcome = try await client.stageAndApply(session: session, now: now)
            reviewOutcome =
                "\(outcome.transaction) · \(outcome.appliedCount) applied · \(outcome.conflictCount) conflict · \(outcome.failedCount) failed"
        } catch {
            reviewError = "Review/apply failed: \(error)"
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
                                            connectionState: model.connectingName == profile.name
                                                ? "Connecting" : "Disconnected"
                                        )
                                    }
                                    .buttonStyle(.plain)
                                    Menu {
                                        Button("Connect") { Task { await model.connect(profile) } }
                                        Button("Edit…") { Task { await model.editProfile(profile) } }
                                        Button("Duplicate…") { Task { await model.duplicateProfile(profile) } }
                                        Button("Test") { Task { await model.testProfile(profile) } }
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
                                    Button("Edit…") { Task { await model.editProfile(profile) } }
                                    Button("Duplicate…") { Task { await model.duplicateProfile(profile) } }
                                    Button("Test") { Task { await model.testProfile(profile) } }
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
                        "Connected · session \(String(session.prefix(16)))…",
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
                        Text("SQL").font(.headline)
                        SqlTextEditor(text: $model.queryText)
                            .frame(minHeight: 56, maxHeight: 80)
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
                        if let cancelOutcome = model.cancelOutcome {
                            Text(cancelOutcome).foregroundStyle(.secondary).font(.callout)
                        }
                        if let reviewOutcome = model.reviewOutcome {
                            Text(reviewOutcome).foregroundStyle(.green).font(.callout)
                        }
                        if let reviewError = model.reviewError {
                            Text(reviewError).foregroundStyle(.red).font(.callout).textSelection(.enabled)
                        }
                    }
                }
                if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_GRID_ROWS"] == nil,
                   let table = model.resultTable
                {
                    CatalogGrid(table: table)
                        .frame(minHeight: 220)
                    if model.nextStartRow != nil {
                        Button("Load more rows") { Task { await model.loadMore() } }
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
        .alert(
            "Connection action failed",
            isPresented: Binding(
                get: { model.profileActionError != nil },
                set: { if !$0 { model.profileActionError = nil } }
            )
        ) { Button("OK") { model.profileActionError = nil } } message: {
            Text(model.profileActionError ?? "Unknown failure")
        }
        .task { await model.initialize() }
        .focusedValue(\.workbenchActions, WorkbenchActions(
            canRun: model.sessionHex != nil && !model.isRunning && !model.isCatalogRefreshing,
            canCancel: model.isRunning,
            canRefresh: model.sessionHex != nil && !model.isRunning && !model.isCatalogRefreshing,
            run: { Task { await model.runQuery() } },
            cancel: { Task { await model.cancel() } },
            refresh: { Task { await model.browse() } }
        ))
        .toolbar(id: "workbench") {
            ToolbarItem(id: "connection", placement: .automatic) {
                Label(
                    model.sessionHex == nil ? "Disconnected" : model.connectedEngine,
                    systemImage: model.sessionHex == nil ? "bolt.slash" : "bolt.horizontal"
                )
                .accessibilityLabel(model.sessionHex == nil
                    ? "No active connection" : "Connected to \(model.connectedEngine)")
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
                .disabled(model.sessionHex == nil || model.isRunning || model.isCatalogRefreshing)
            }
            ToolbarItem(id: "cancel", placement: .primaryAction) {
                Button { Task { await model.cancel() } } label: {
                    Label("Cancel Query", systemImage: "stop.fill")
                }
                .disabled(!model.isRunning)
            }
        }
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

    func makeCoordinator() -> Coordinator {
        Coordinator(
            table: table,
            selection: $selection,
            refreshState: refreshState,
            onExpand: onExpand
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
        weak var outline: NSOutlineView?
        private var suppressExpansionCallbacks = false

        init(
            table: [BridgeCatalogNode],
            selection: Binding<String?>,
            refreshState: CatalogRefreshState,
            onExpand: @escaping @MainActor (String) -> Void
        ) {
            self.selection = selection
            self.onExpand = onExpand
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
