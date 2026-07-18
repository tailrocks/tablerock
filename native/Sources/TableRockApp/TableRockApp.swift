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
    func open(params: OpenParams) throws -> Data { try bridge.open(params: params) }
    func openProfile(id: Data) throws -> Data {
        try bridge.openProfile(profileId: id, passwordOverride: nil)
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

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
                .frame(minWidth: 760, minHeight: 520)
        }
        .commands {
            WorkbenchCommands()
        }
        Settings {
            NativeSettingsView()
        }
    }
}

/// Owns the live TableRockBridge + the profile list for the window's lifetime.
@MainActor
@Observable
final class BridgeModel {
    var status: String = "starting…"
    var bridgeError: String?
    var profiles: [BridgeProfileItem] = []
    var sessionHex: String?
    var connectError: String?
    var connectingName: String?
    var catalogSummary: String?
    var catalogError: String?
    var catalogSnapshot: PageV1Table?
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

    private static let persistenceDirectory: URL = {
        let base = FileManager.default.temporaryDirectory
            .appendingPathComponent("tablerock-native", isDirectory: true)
        try? FileManager.default.createDirectory(
            at: base,
            withIntermediateDirectories: true
        )
        return base
    }()

    func initialize() async {
        do {
            client = try BridgeClient(persistencePath: Self.persistenceDirectory
                .appendingPathComponent("profiles.db").path)
            await refreshProfiles()
        } catch {
            bridgeError = "Bridge init failed: \(error)"
            status = "error"
        }
    }

    func refreshProfiles() async {
        guard let client else { return }
        do {
            profiles = try await client.listProfiles()
            status = profiles.isEmpty
                ? "Bridge ready · no saved profiles"
                : "Bridge ready · \(profiles.count) profile\(profiles.count == 1 ? "" : "s")"
        } catch {
            bridgeError = "List profiles failed: \(error)"
            status = "error"
        }
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
        resultTable = nil
        do {
            let session = try await client.open(params: OpenParams(
                engine: formEngine,
                host: formHost,
                port: port,
                database: formDatabase,
                user: formUser,
                password: formPassword
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

    func browse() async {
        catalogSummary = nil
        catalogError = nil
        catalogSnapshot = nil
        do {
            if let table = try await fetchPage(intent: "catalog", statement: nil) {
                catalogSnapshot = table
                catalogSummary = connectedEngine == "redis"
                    ? "keys · \(table.rows.count)"
                    : "tables · \(table.rows.count)"
            } else {
                catalogSummary = "catalog: no result page"
            }
        } catch {
            catalogError = "Browse failed: \(error)"
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
                List(model.profiles, id: \.name) { profile in
                    Button { Task { await model.connect(profile) } } label: {
                        ProfileRow(profile: profile)
                    }
                    .buttonStyle(.plain)
                }
                .overlay {
                    if model.profiles.isEmpty && model.sessionHex == nil {
                        ContentUnavailableView(
                            model.bridgeError == nil ? "No profiles" : "Error",
                            systemImage: model.bridgeError == nil ? "tray" : "exclamationmark.triangle",
                            description: Text(model.bridgeError ?? "Use the connection form to begin.")
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
                        .disabled(model.isRunning)
                        .accessibilityLabel("Refresh catalog")
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    if let snapshot = model.catalogSnapshot {
                        CatalogOutline(
                            table: snapshot,
                            selection: $model.catalogSelection
                        )
                        .frame(minHeight: 160)
                    } else {
                        ContentUnavailableView(
                            "Catalog not loaded",
                            systemImage: "sidebar.left",
                            description: Text("Refresh to list database objects.")
                        )
                        .frame(minHeight: 160)
                    }
                }
            }
            .navigationTitle("Connections")
        } detail: {
            VStack(alignment: .leading, spacing: 12) {
                Text("TableRock").font(.largeTitle).bold()
                Text(model.status).foregroundStyle(.secondary)
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
                                .disabled(model.isRunning)
                            Button("Cancel") { Task { await model.cancel() } }
                                .disabled(!model.isRunning)
                            Button("Refresh catalog") { Task { await model.browse() } }
                                .disabled(model.isRunning)
                            Button("Apply probe edit") { Task { await model.applyProbeEdit() } }
                                .disabled(model.isRunning)
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
                if let table = model.resultTable {
                    CatalogGrid(table: table)
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
        .task { await model.initialize() }
        .focusedValue(\.workbenchActions, WorkbenchActions(
            canRun: model.sessionHex != nil && !model.isRunning,
            canCancel: model.isRunning,
            canRefresh: model.sessionHex != nil && !model.isRunning,
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
            ToolbarItem(id: "refresh", placement: .automatic) {
                Button { Task { await model.browse() } } label: {
                    Label("Refresh Catalog", systemImage: "arrow.clockwise")
                }
                .disabled(model.sessionHex == nil || model.isRunning)
            }
            ToolbarItem(id: "run", placement: .primaryAction) {
                Button { Task { await model.runQuery() } } label: {
                    Label("Run Query", systemImage: "play.fill")
                }
                .buttonStyle(.borderedProminent)
                .disabled(model.sessionHex == nil || model.isRunning)
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
    let table: PageV1Table
    @Binding var selection: String?

    func makeCoordinator() -> Coordinator {
        Coordinator(table: table, selection: $selection)
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
        context.coordinator.rebuild(from: table)
        outline.reloadData()
        context.coordinator.restore(expanded: expanded, selected: selected)
    }

    @MainActor
    final class Node: NSObject {
        let key: String
        let title: String
        let children: [Node]

        init(key: String, title: String, children: [Node] = []) {
            self.key = key
            self.title = title
            self.children = children
        }
    }

    @MainActor
    final class Coordinator: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
        private(set) var roots: [Node] = []
        private var nodesByKey: [String: Node] = [:]
        var selection: Binding<String?>
        weak var outline: NSOutlineView?

        init(table: PageV1Table, selection: Binding<String?>) {
            self.selection = selection
            super.init()
            rebuild(from: table)
        }

        func rebuild(from table: PageV1Table) {
            if table.columns.count >= 2 {
                var order: [String] = []
                var grouped: [String: [Node]] = [:]
                for row in table.rows where row.count >= 2 {
                    let group = row[0]
                    if grouped[group] == nil { order.append(group) }
                    let title = row.dropFirst().joined(separator: " · ")
                    grouped[group, default: []].append(Node(
                        key: "item:\(group)\u{1f}\(title)", title: title))
                }
                roots = order.map { group in
                    Node(key: "group:\(group)", title: group, children: grouped[group] ?? [])
                }
            } else {
                roots = table.rows.enumerated().compactMap { index, row in
                    guard let title = row.first else { return nil }
                    return Node(key: "item:\(index)\u{1f}\(title)", title: title)
                }
            }
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
            return !node.children.isEmpty
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
            cell.setAccessibilityLabel(node.children.isEmpty
                ? "Catalog object \(node.title)" : "Catalog group \(node.title)")
            return cell
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
            roots.filter { !$0.children.isEmpty }.forEach { outline.expandItem($0) }
        }
    }
}

struct CatalogGrid: NSViewRepresentable {
    let table: PageV1Table

    func makeCoordinator() -> Coordinator { Coordinator(table) }

    func makeNSView(context: Context) -> NSScrollView {
        let grid = NSTableView()
        grid.delegate = context.coordinator
        grid.dataSource = context.coordinator
        grid.usesAlternatingRowBackgroundColors = true
        grid.allowsColumnReordering = true
        grid.allowsColumnResizing = true
        grid.allowsMultipleSelection = true
        grid.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        grid.rowSizeStyle = .small
        grid.setAccessibilityLabel("Query results")
        context.coordinator.installColumns(on: grid)

        let scroll = NSScrollView()
        scroll.documentView = grid
        scroll.hasVerticalScroller = true
        scroll.hasHorizontalScroller = true
        scroll.autohidesScrollers = true
        scroll.borderType = .bezelBorder
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let grid = scroll.documentView as? NSTableView else { return }
        let selectedRows = grid.selectedRowIndexes
        context.coordinator.snapshot = table
        context.coordinator.installColumns(on: grid)
        grid.reloadData()
        let validSelection = selectedRows.filter { $0 < table.rows.count }
        grid.selectRowIndexes(IndexSet(validSelection), byExtendingSelection: false)
    }

    @MainActor
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
        var snapshot: PageV1Table

        init(_ snapshot: PageV1Table) {
            self.snapshot = snapshot
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
        editor.string = text
        editor.setAccessibilityLabel("SQL editor")

        let scroll = NSScrollView()
        scroll.documentView = editor
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

struct ProfileRow: View {
    let profile: BridgeProfileItem

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 6) {
                if profile.favorite {
                    Image(systemName: "star.fill").foregroundStyle(.yellow).font(.caption)
                }
                Text(profile.name).font(.body)
            }
            Text([profile.engine, profile.group].compactMap { $0 }.joined(separator: " · "))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.vertical, 2)
    }
}
