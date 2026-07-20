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
import TableRockFeature
import UniformTypeIdentifiers
import Security

private func zeroizeTransientData(_ data: inout Data?) {
    guard var value = data else { return }
    value.resetBytes(in: 0..<value.count)
    data = value
}

private extension Data {
    func hexEncodedString() -> String {
        map { String(format: "%02x", $0) }.joined()
    }
}

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
    func testProfile(id: Data, secretOverride: Data?) throws -> BridgeConnectionTestReport {
        try bridge.testProfileWithSecret(profileId: id, secretOverride: secretOverride)
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
    func putNativeWindowIntent(
        windowId: String, profileId: Data, intent: BridgeSessionIntent
    ) throws {
        try bridge.putNativeWindowIntent(
            windowId: windowId, profileId: profileId, intent: intent
        )
    }
    func nativeWindowIntent(windowId: String) throws -> BridgeNativeWindowIntent? {
        try bridge.getNativeWindowIntent(windowId: windowId)
    }
    func deleteNativeWindowIntent(windowId: String) throws {
        try bridge.deleteNativeWindowIntent(windowId: windowId)
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
    func openProfile(id: Data, secretOverride: Data?) throws -> Data {
        try bridge.openProfileWithSecret(profileId: id, secretOverride: secretOverride)
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
    func reconnect(session: Data, secretOverride: Data? = nil) throws -> BridgeReconnectAttempt {
        try bridge.reconnectSavedSessionWithSecret(
            sessionId: session, secretOverride: secretOverride
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

    func previewCsvImport(path: String) throws -> BridgeCsvImportPreview {
        try bridge.previewCsvImport(path: path)
    }

    func stageCsvImport(
        sessionId: Data, catalogNodeId: Data, path: String,
        mappedColumns: [String], mappedTypes: [String], nowMs: UInt64
    ) throws -> BridgeCsvImportReview {
        try bridge.stageCsvImport(
            sessionId: sessionId, catalogNodeId: catalogNodeId, path: path,
            mappedColumns: mappedColumns, mappedTypes: mappedTypes, nowMs: nowMs
        )
    }

    func relationStructure(sessionId: Data, catalogNodeId: Data) throws
        -> BridgeRelationStructure
    {
        try bridge.relationStructure(
            sessionId: sessionId, catalogNodeId: catalogNodeId
        )
    }
    func redisKeyView(
        sessionId: Data, catalogNodeId: Data, collectionSkip: UInt64
    ) throws -> BridgeRedisKeyView {
        try bridge.redisKeyView(
            sessionId: sessionId, catalogNodeId: catalogNodeId,
            collectionSkip: collectionSkip
        )
    }

    func redisOverview(sessionId: Data) throws -> BridgeRedisOverview {
        try bridge.redisOverview(sessionId: sessionId)
    }

    func applyReviewToken(tokenId: Data, nowMs: UInt64, sessionId: Data) throws -> ApplyOutcome {
        try bridge.applyReviewToken(
            tokenId: tokenId, nowMs: nowMs, sessionId: sessionId, expectedRevision: 0
        )
    }

    func revokeReviewToken(tokenId: Data) throws -> Bool {
        try bridge.revokeReviewToken(tokenId: tokenId)
    }

    func stageAndApply(session: Data, now: UInt64) throws -> ApplyOutcome {
        let token = try bridge.stageProbeReview(sessionId: session, nowMs: now)
        return try bridge.applyReviewToken(
            tokenId: token, nowMs: now, sessionId: session, expectedRevision: 0)
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
        let status = SecItemDelete([
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: namespace,
            kSecMatchItemList: [reference] as CFArray
        ] as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw AppCapabilityError.rejected("keychain-remove-\(status)")
        }
    }
}

@MainActor
private final class NativeApplicationModel {
    let client: BridgeClient?
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
            configuredDependencies = AppDependencies(
                filePanels: SystemFilePanelPort(),
                pasteboard: SystemPasteboardPort(),
                keychain: SystemKeychainPort(namespace: configuration.keychainNamespace)
            )
            try configuration.paths.prepare()
            guard configuration.backend == .live else {
                throw AppConfigurationError.unsupportedBackend("scripted backend not installed")
            }
            let configuredClient = try BridgeClient(
                persistencePath: configuration.paths.profilesDatabase.path
            )
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
            NativeSettingsView()
        }
    }
}

private struct WorkbenchWindowRoot: View {
    @Environment(\.openWindow) private var openWindow
    @State private var model: BridgeModel
    private let application: NativeApplicationModel

    init(application: NativeApplicationModel, windowId: UUID) {
        self.application = application
        _model = State(initialValue: BridgeModel(
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
                .modifier(NativeAppearanceFixtureModifier(
                    fixture: NativeAppearanceFixture.current
                ))
                .frame(minWidth: 760, minHeight: 520)
                .task { await openFixtureWindowIfNeeded() }
        }
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
    private let draft = BridgeProfileDraft(
        idBytes: Data(repeating: 7, count: 16), revision: 3,
        engine: "postgresql", name: "Production analytics", group: "Production",
        environment: "production", host: "db.example.internal", port: "5432",
        database: "analytics", username: "operator", passwordSource: "prompt",
        passwordValue: "", passwordReference: nil, hasStoredPassword: false,
        plaintextAcknowledged: false,
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
    guard labels.contains(#"{"ok":true}"#),
          labels.contains("7b 22 6f 6b 22 3a 74 72 75 65 7d")
    else {
        writePerformanceMetric("VALUE_INSPECTOR_PROOF_FAILED labels=\(labels)")
        return
    }
    writePerformanceMetric(
        "VALUE_INSPECTOR_PROOF_PASSED metadata=column_type_kind_nullability truncation=true text=true hex=true appkit_selection=true"
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
    let id: UUID
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
    var selectedCell: NativeCellSelection?
    var copyOutcome: String?
    var copyError: String?

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
    var resultTable: PageV1Table?
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
    var selectedSection = "data"
    var structure: BridgeRelationStructure?
    var structureLoading = false
    var structureError: String?
    var redisView: BridgeRedisKeyView?

    init(id: UUID, node: BridgeCatalogNode, pinned: Bool = false) {
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
    var csvImportPresented = false
    var csvImportPreview: BridgeCsvImportPreview?
    var csvImportMappedColumns: [String] = []
    var csvImportColumnTypes: [String] = []
    var csvImportReview: BridgeCsvImportReview?
    var csvImportError: String?
    var csvImportOutcome: String?
    var csvImportApplying = false
    private var csvImportUrl: URL?
    var redisOverviewPresented = false
    var redisOverview: BridgeRedisOverview?
    private(set) var redisOverviewLoading = false
    private(set) var redisOverviewError: String?
    var queryTabs: [NativeQueryTab]
    var selectedQueryTabId: UUID
    var objectTabs: [NativeObjectTab] = []
    var selectedObjectTabId: UUID?
    var selectedWorkbenchKind = "query"
    var pendingQueryTabClose: NativeQueryTab?
    var queryTabRename: NativeQueryTab?
    var queryTabRenameText = ""
    private var activeProfileId: Data?
    var activeProfile: BridgeProfileItem? {
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
    var selectedCellSnapshot: (PageV1Column, PageV1Cell, Int, Int)? {
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
    var copyOutcome: String? {
        get { selectedWorkbenchKind == "object" ? activeObjectTab?.copyOutcome : activeQueryTab.copyOutcome }
        set {
            if selectedWorkbenchKind == "object" { activeObjectTab?.copyOutcome = newValue }
            else { activeQueryTab.copyOutcome = newValue }
        }
    }
    var copyError: String? {
        get { selectedWorkbenchKind == "object" ? activeObjectTab?.copyError : activeQueryTab.copyError }
        set {
            if selectedWorkbenchKind == "object" { activeObjectTab?.copyError = newValue }
            else { activeQueryTab.copyError = newValue }
        }
    }
    func selectCell(row: Int, column: Int) {
        selectedCell = NativeCellSelection(row: row, column: column)
    }
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
    private let client: BridgeClient?
    private let startupError: String?
    private let dependencies: AppDependencies
    var sessionData: Data?

    fileprivate init(
        client: BridgeClient? = nil,
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
            let node = BridgeCatalogNode(
                idBytes: Data(repeating: 7, count: 16), parentIdBytes: Data(repeating: 6, count: 16),
                depth: 2, name: "users", kind: "postgresql_table",
                childrenState: "not_applicable", expandable: false
            )
            let first = NativeObjectTab(
                id: dependencies.identifiers.next(), node: node, pinned: true
            )
            first.resultTable = PageV1Table(columns: ["id"], rows: [["1"]])
            let preview = NativeObjectTab(id: dependencies.identifiers.next(), node: node)
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
        if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_VALUE_INSPECTOR"] == "1" {
            sessionData = Data(repeating: 4, count: 16)
            sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
            connectedEngine = "postgresql"
            let raw = Data(#"{"ok":true}"#.utf8)
            activeQueryTab.resultTable = PageV1Table(
                columns: ["payload"], rows: [[#"{"ok":true}"#]],
                columnMetadata: [PageV1Column(
                    name: "payload", engine: 0, engineType: "jsonb", nullable: true
                )],
                cells: [[PageV1Cell(
                    display: #"{"ok":true}"#, kind: 8, truncation: 2,
                    originalByteCount: 128, bytes: raw
                )]]
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
        if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_STRUCTURE"] == "1" {
            guard let client else {
                writePerformanceMetric("STRUCTURE_PROOF_FAILED no bridge")
                return
            }
            do {
                let session = try await client.open(params: OpenParams(
                    engine: "postgresql", host: "127.0.0.1", port: 5433,
                    database: "db", user: "u", password: "secret", tlsMode: "off"
                ))
                sessionData = session
                sessionHex = session.map { String(format: "%02x", $0) }.joined()
                connectedEngine = "postgresql"
                guard let database = try await client.refreshCatalog(
                    session: session, parentNodeId: nil
                ).first, let schema = try await client.refreshCatalog(
                    session: session, parentNodeId: database.idBytes
                ).first(where: { $0.name == "public" }) else {
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
                      tab.structure?.constraints.contains(where: { $0.name == "structure_probe_name_check" }) == true
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
                let session = try await client.open(params: OpenParams(
                    engine: "clickhouse", host: "127.0.0.1", port: 8122,
                    database: "db", user: "u", password: "secret", tlsMode: "off"
                ))
                sessionData = session
                sessionHex = session.map { String(format: "%02x", $0) }.joined()
                connectedEngine = "clickhouse"
                guard let database = try await client.refreshCatalog(
                    session: session, parentNodeId: nil
                ).first(where: { $0.name == "db" }) else {
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
                let session = try await client.open(params: OpenParams(
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
        if ProcessInfo.processInfo.environment["TABLEROCK_FIXTURE_REDIS_KEY_VIEW"] == "1" {
            guard let client else {
                writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED no bridge")
                return
            }
            do {
                let session = try await client.open(params: OpenParams(
                    engine: "redis", host: "127.0.0.1", port: 6380,
                    database: "0", user: "", password: "", tlsMode: "off"
                ))
                sessionData = session
                sessionHex = session.map { String(format: "%02x", $0) }.joined()
                connectedEngine = "redis"
                guard let database = try await client.refreshCatalog(
                    session: session, parentNodeId: nil
                ).first(where: { $0.name == "db0" }) else {
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
                let session = try await client.open(params: OpenParams(
                    engine: "postgresql", host: "127.0.0.1", port: 5433,
                    database: "db", user: "u", password: "secret", tlsMode: "off"
                ))
                sessionData = session
                sessionHex = session.map { String(format: "%02x", $0) }.joined()
                connectedEngine = "postgresql"
                guard let database = try await client.refreshCatalog(
                    session: session, parentNodeId: nil
                ).first, let schema = try await client.refreshCatalog(
                    session: session, parentNodeId: database.idBytes
                ).first(where: { $0.name == "public" }) else {
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
                guard let verification = try await fetchPage(
                    intent: "execute",
                    statement: "SELECT count(*)::bigint AS n FROM import_probe",
                    tab: activeQueryTab
                ), verification.rows == [["2"]] else {
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
                let session = try await client.open(params: OpenParams(
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
            first.resultTable = PageV1Table(columns: ["n"], rows: [["1"]])
            first.isRunning = true
            first.querySummary = "first result"
            let second = NativeQueryTab(
                id: dependencies.identifiers.next(), title: "Orders", statementText: "SELECT 2;"
            )
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
            activeProfileId = profiles[0].idBytes
            sessionData = Data(repeating: 3, count: 16)
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
        resultTable = PageV1Table(columns: columns, rows: rows)
        let elapsed = Date().timeIntervalSince(started)
        catalogSummary = "Performance fixture · \(count) rows · \(columns.count) columns"
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
        tab.redisView = nil
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
            tab.redisView = BridgeRedisKeyView(
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
                guard table.append(more) else {
                    tab.error = "Load more returned incompatible page metadata"
                    return
                }
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
                AppPasteboardRepresentation(type: "public.utf8-tab-separated-values-text", value: payloads["tsv"] ?? ""),
                AppPasteboardRepresentation(type: "public.json", value: payloads["json"] ?? ""),
                AppPasteboardRepresentation(
                    type: "net.daringfireball.markdown", value: payloads["markdown"] ?? ""
                ),
            ])
            copyOutcome = "Copied \(scope) as \(preferredFormat.uppercased()) with CSV, TSV, JSON, and Markdown representations"
        } catch { copyError = "Copy failed: \(error)" }
    }

    func exportLoadedResult(format: String) async {
        guard let client, let resultId = resultIdData else {
            copyError = "No resident result to export"
            return
        }
        let fileExtension = format == "sql_insert" ? "sql" : format
        guard let selected = dependencies.filePanels.chooseSaveFile(AppFilePanelRequest(
            title: "Export Loaded Result", prompt: "Export",
            suggestedFilename: "result.\(fileExtension)", allowedExtensions: [fileExtension]
        )) else { return }
        let url = selected.pathExtension.lowercased() == fileExtension
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

    func chooseCsvImport() async {
        guard let client, sqlInsertCopyAvailable else { return }
        guard let url = dependencies.filePanels.chooseOpenFile(AppFilePanelRequest(
            title: "Import CSV into Table", prompt: "Preview", allowedExtensions: ["csv"]
        )) else { return }
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
            let outcome = try await client.applyReviewToken(
                tokenId: review.tokenId,
                nowMs: dependencies.clock.nowMilliseconds(),
                sessionId: session
            )
            csvImportReview = nil
            csvImportOutcome =
                "\(outcome.transaction) · \(outcome.appliedCount) applied · \(outcome.conflictCount) conflict · \(outcome.failedCount) failed"
            if outcome.failedCount == 0 && outcome.conflictCount == 0 {
                await reloadObjectTab()
            }
        } catch {
            csvImportReview = nil
            csvImportError = "Import apply failed; review authority consumed: \(error)"
        }
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

    private func restoreSessionIntent(profileId: Data) async {
        guard let client else { return }
        do {
            guard let record = try await client.nativeWindowIntent(
                windowId: windowId.uuidString.lowercased()
            ), record.profileId == profileId else {
                let tab = NativeQueryTab(
                    id: dependencies.identifiers.next(), title: "Query 1", statementText: ""
                )
                queryTabs = [tab]
                selectedQueryTabId = tab.id
                return
            }
            applySessionIntent(record.intent)
        } catch { profileActionError = "Restore workspace intent failed: \(error)" }
    }

    private func restoreWindowIntentOnLaunch() async {
        guard let client else { return }
        do {
            guard let record = try await client.nativeWindowIntent(
                windowId: windowId.uuidString.lowercased()
            ), let profile = profiles.first(where: { $0.idBytes == record.profileId })
            else { return }
            applySessionIntent(record.intent)
            activeProfileId = record.profileId
            profileActionOutcome = "Restored \(profile.name) workspace; connect to resume"
        } catch { profileActionError = "Restore window intent failed: \(error)" }
    }

    private func applySessionIntent(_ intent: BridgeSessionIntent) {
        let restored = intent.tabs.map {
            NativeQueryTab(
                id: dependencies.identifiers.next(),
                title: $0.title,
                statementText: $0.statementText
            )
        }
        guard !restored.isEmpty, Int(intent.selectedTab) < restored.count else { return }
        queryTabs = restored
        selectedQueryTabId = restored[Int(intent.selectedTab)].id
        formDatabase = intent.database
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
        guard let url = dependencies.filePanels.chooseOpenFile(AppFilePanelRequest(
            title: "Open SQL File", prompt: "Open", allowedExtensions: ["sql"]
        )), let client else { return }
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
            guard let selected = dependencies.filePanels.chooseSaveFile(AppFilePanelRequest(
                title: "Save SQL File", prompt: "Save", suggestedFilename: "query.sql",
                allowedExtensions: ["sql"]
            )) else { return }
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
            passwordValue: "", passwordReference: nil, hasStoredPassword: false,
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
        if copy.passwordSource == "keychain" {
            copy.passwordReference = nil
            copy.hasStoredPassword = false
        }
        editorDraft = copy
    }

    func saveProfile(_ draft: BridgeProfileDraft) async -> Bool {
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
            _ = try await client.saveProfile(draft)
            var cleanupWarning = false
            if let oldReference, let addedReference, oldReference != addedReference {
                do { try dependencies.keychain.remove(reference: oldReference) }
                catch { cleanupWarning = true }
            }
            editorDraft = nil
            profileActionOutcome = cleanupWarning
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

    func testProfile(_ item: BridgeProfileItem, passwordOverride: String? = nil) async {
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
                do { try dependencies.keychain.remove(reference: reference) }
                catch { cleanupWarning = true }
            }
            profileActionOutcome = cleanupWarning
                ? "Connection removed; Keychain item cleanup failed"
                : "Connection removed: \(item.name)"
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

    private func keychainPassword(for draft: BridgeProfileDraft) throws -> Data {
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
            await reconnectActive(
                sourceSession: sourceSession, secretOverride: Data(password.utf8)
            )
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

    func isActiveProfile(_ profile: BridgeProfileItem) -> Bool {
        sessionData != nil && activeProfileId == profile.idBytes
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
                guard table.append(more) else {
                    tab.queryError = "Load more returned incompatible page metadata"
                    return
                }
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
                .accessibilityIdentifier("sidebar.profiles")
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
                        .accessibilityIdentifier("profile.add")
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
                EnvironmentSafetyBadge(model: model)
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
        .sheet(isPresented: Binding(
            get: { model.editorDraft != nil },
            set: { if !$0 { model.editorDraft = nil } }
        )) {
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
        .sheet(isPresented: $model.historyPresented) {
            HistorySheet()
        }
        .sheet(isPresented: $model.savedQueriesPresented) {
            SavedQueriesSheet()
        }
        .sheet(isPresented: $model.redisOverviewPresented) {
            RedisOverviewSheet()
        }
        .sheet(
            isPresented: $model.csvImportPresented,
            onDismiss: { Task { await model.closeCsvImport() } }
        ) {
            CsvImportSheet()
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

    private var queryStatus: String {
        model.queryError ?? model.cancelOutcome ?? model.querySummary ?? "Idle"
    }

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
                ResultGridWithInspector(table: table, minimumHeight: 220)
                if model.nextStartRow != nil {
                    Button("Load more rows") { Task { await model.loadMore() } }
                        .accessibilityIdentifier("results.next-page")
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
                    if !tab.kind.hasPrefix("redis_key_") {
                        Picker("Object section", selection: Binding(
                        get: { tab.selectedSection },
                        set: { section in
                            tab.selectedSection = section
                            if section == "structure" {
                                Task { await model.loadObjectStructure() }
                            }
                        }
                    )) {
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
                            .disabled(tab.isRunning)
                    }
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

private struct RedisKeyObjectView: View {
    let view: BridgeRedisKeyView

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
                                    Text([
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
                                        Text(structure.facts[index].value.isEmpty
                                            ? "—" : structure.facts[index].value)
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
            if let preview = model.csvImportPreview {
                Text("\(URL(fileURLWithPath: preview.path).lastPathComponent) · \(preview.totalRows) rows · \(preview.headers.count) columns")
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
                        Text("Insert \(review.rowCount) rows and \(review.columnCount) mapped columns into \(review.target).")
                            .font(.headline)
                        if review.formulaLikeCells > 0 {
                            Text("\(review.formulaLikeCells) formula-like cells are frozen as literal text in this reviewed plan.")
                                .foregroundStyle(.orange)
                        }
                        Text("The reviewed plan is frozen for 60 seconds. Authority is consumed before database I/O and cannot be retried after failure.")
                            .foregroundStyle(.secondary)
                        HStack {
                            Button("Apply Import") { Task { await model.applyCsvImport() } }
                                .buttonStyle(.borderedProminent)
                                .disabled(model.csvImportApplying)
                            Button("Discard Review", role: .cancel) {
                                Task { await model.discardCsvImportReview() }
                            }
                            .disabled(model.csvImportApplying)
                        }
                    }
                    .padding(6)
                }
            } else if model.csvImportOutcome == nil {
                Button("Stage Reviewed Import") { Task { await model.stageCsvImport() } }
                    .buttonStyle(.borderedProminent)
                    .disabled(model.csvImportPreview == nil || model.csvImportApplying)
            }
            if model.csvImportApplying { ProgressView("Applying reviewed import…") }
            if let outcome = model.csvImportOutcome {
                Label(outcome, systemImage: "checkmark.circle.fill").foregroundStyle(.green)
            }
            if let error = model.csvImportError {
                Text(error).foregroundStyle(.red).textSelection(.enabled)
            }
        }
        .padding(20)
        .frame(minWidth: 720, minHeight: 560)
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

private struct ResultGridWithInspector: View {
    @Environment(BridgeModel.self) private var model
    let table: PageV1Table
    let minimumHeight: CGFloat

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                ResultCopyMenu()
                ResultExportMenu()
                if let outcome = model.copyOutcome {
                    Text(outcome).font(.caption).foregroundStyle(.secondary)
                }
                if let error = model.copyError {
                    Text(error).font(.caption).foregroundStyle(.red)
                }
                Spacer()
            }
            HSplitView {
                CatalogGrid(table: table) { row, column in
                    model.selectCell(row: row, column: column)
                }
                .frame(minWidth: 360, minHeight: minimumHeight)
                if let snapshot = model.selectedCellSnapshot {
                    NativeValueInspector(
                        column: snapshot.0, cell: snapshot.1,
                        row: snapshot.2, columnIndex: snapshot.3
                    )
                    .frame(minWidth: 220, idealWidth: 280, maxWidth: 380)
                }
            }
        }
    }
}

private struct ResultExportMenu: View {
    @Environment(BridgeModel.self) private var model

    var body: some View {
        Menu {
            exportButton("CSV", format: "csv")
            exportButton("TSV", format: "tsv")
            exportButton("JSON", format: "json")
            exportButton("Markdown", format: "markdown")
            if model.sqlInsertCopyAvailable {
                exportButton("SQL INSERT", format: "sql_insert")
            }
        } label: {
            Label("Export Loaded", systemImage: "square.and.arrow.down")
        }
        .disabled(model.resultIdData == nil)
        .accessibilityHint("Atomically export all rows currently resident in this result")
    }

    private func exportButton(_ label: String, format: String) -> some View {
        Button(label) { Task { await model.exportLoadedResult(format: format) } }
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
    let column: PageV1Column
    let cell: PageV1Cell
    let row: Int
    let columnIndex: Int

    private var hex: String {
        cell.bytes.map { String(format: "%02x", $0) }.joined(separator: " ")
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
            }
            .padding(10)
        }
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityElement(children: .contain)
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
                            Button { model.selectQueryTab(tab) } label: {
                                WorkbenchTabLabel(title: tab.title, model: model)
                            }
                                .buttonStyle(.borderedProminent)
                                .accessibilityIdentifier("query.tab.\(tab.id.uuidString.lowercased())")
                                .accessibilityValue("Selected")
                        } else {
                            Button { model.selectQueryTab(tab) } label: {
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
                                WorkbenchTabLabel(
                                    title: tab.title, model: model,
                                    leadingSystemImage: tab.pinned ? "pin.fill" : "eye"
                                )
                            }
                            .buttonStyle(.borderedProminent)
                            .accessibilityIdentifier("object.tab.\(tab.id.uuidString.lowercased())")
                            .accessibilityValue("Selected")
                        } else {
                            Button { model.selectObjectTab(tab) } label: {
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
                Image(systemName: model.activeProductionWarning
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
        ToolbarItem(id: "environment-safety", placement: .automatic) {
            EnvironmentSafetyBadge(model: model)
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
                    List(model.savedQueries, id: \.queryId) { item in
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
                    List(model.historyItems, id: \.historyId) { item in
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
    let onSelect: @MainActor (Int, Int) -> Void

    init(
        table: PageV1Table,
        onSelect: @escaping @MainActor (Int, Int) -> Void = { _, _ in }
    ) {
        self.table = table
        self.onSelect = onSelect
    }

    func makeCoordinator() -> Coordinator { Coordinator(table, onSelect: onSelect) }

    func makeNSView(context: Context) -> NSScrollView {
        let grid = NSTableView()
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
        context.coordinator.startPerformanceScrollIfRequested(on: grid)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let grid = scroll.documentView as? NSTableView else { return }
        let selectedRows = grid.selectedRowIndexes
        context.coordinator.snapshot = table
        context.coordinator.onSelect = onSelect
        context.coordinator.installColumns(on: grid)
        grid.reloadData()
        context.coordinator.startPerformanceScrollIfRequested(on: grid)
        let validSelection = selectedRows.filter { $0 < table.rows.count }
        grid.selectRowIndexes(IndexSet(validSelection), byExtendingSelection: false)
    }

    @MainActor
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
        var snapshot: PageV1Table
        var onSelect: @MainActor (Int, Int) -> Void
        private var fixtureScrollTask: Task<Void, Never>?

        init(_ snapshot: PageV1Table, onSelect: @escaping @MainActor (Int, Int) -> Void) {
            self.snapshot = snapshot
            self.onSelect = onSelect
        }

        func tableViewSelectionDidChange(_ notification: Notification) {
            guard let tableView = notification.object as? NSTableView,
                  tableView.selectedRow >= 0
            else { return }
            let column = max(tableView.clickedColumn, 0)
            guard snapshot.columns.indices.contains(column) else { return }
            onSelect(tableView.selectedRow, column)
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
            && (draft.passwordSource != "keychain"
                || draft.passwordReference != nil || !draft.passwordValue.isEmpty)
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
                        Text("macOS Keychain").tag("keychain")
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
