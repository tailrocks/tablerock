// TableRock native macOS app — plan 020.
//
// Built with swiftc via Command Line Tools (no full Xcode required): SwiftUI +
// AppKit ship with the CLT macOS SDK, and the Rust bridge is linked as the cargo
// release dylib. Notarized XCFramework distribution remains the operator-gated
// release path (plan 019). See scripts/build-native-app.sh (direct swiftc,
// license-free).
//
// Checkpoint 1: app shell + live bridge (runtime + persistence).
// Checkpoint 2: connection list — lists saved profiles over the bridge.

import SwiftUI
import Observation
import TableRockBridge

@main
struct TableRockApp: App {
    @State private var model = BridgeModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
                .frame(minWidth: 760, minHeight: 520)
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
    var catalogTable: PageV1Table?
    var writeOutcome: String?
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
    var bridge: TableRockBridge?
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

    func initialize() {
        do {
            let bridge = TableRockBridge.create()
            try bridge.ensureRuntime()
            try bridge.configurePersistence(
                path: Self.persistenceDirectory
                    .appendingPathComponent("profiles.db")
                    .path
            )
            self.bridge = bridge
            refreshProfiles()
        } catch {
            bridgeError = "Bridge init failed: \(error)"
            status = "error"
        }
    }

    func refreshProfiles() {
        guard let bridge else { return }
        do {
            profiles = try bridge.listProfiles()
            status = profiles.isEmpty
                ? "Bridge ready · no saved profiles"
                : "Bridge ready · \(profiles.count) profile\(profiles.count == 1 ? "" : "s")"
        } catch {
            bridgeError = "List profiles failed: \(error)"
            status = "error"
        }
    }

    /// Connect directly from form params (temporary session, no saved profile).
    func connectByParams() {
        guard let bridge,
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
        catalogTable = nil
        do {
            let session = try bridge.open(params: OpenParams(
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
    func connect(_ item: BridgeProfileItem) {
        guard let bridge else { return }
        connectingName = item.name
        sessionHex = nil
        sessionData = nil
        connectError = nil
        catalogSummary = nil
        catalogError = nil
        do {
            let session = try bridge.openProfile(profileId: item.idBytes, passwordOverride: nil)
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
    private func fetchPage(intent: String, statement: String?) throws -> PageV1Table? {
        guard let bridge, let session = sessionData else { return nil }
        let spec = SubmitSpec(
            intent: intent,
            sessionId: session,
            statement: statement,
            resultId: nil,
            startRow: nil,
            rowCount: 500,
            expectedRevision: 0
        )
        let operationId = try bridge.submit(spec: spec)
        try bridge.pump(operationId: operationId)
        var cursor: UInt64 = 0
        for _ in 0..<64 {
            let batch = try bridge.nextEvents(cursor: cursor, maximum: 64)
            if batch.events.isEmpty { break }
            for event in batch.events {
                if event.kind == "page", let page = event.pageBytes {
                    let env = try PageV1.decodeEnvelope(page)
                    resultIdData = env.resultId
                    resultRevision = env.revision
                    nextStartRow = env.startRow + UInt64(env.rowCount)
                    writeOutcome = nil
                    return try PageV1.decodeTable(page)
                }
                if event.kind == "terminal" {
                    writeOutcome = event.outcome ?? "ok"
                    return nil
                }
            }
            cursor = batch.nextCursor
        }
        return nil
    }

    /// Fetch the next page of the current result and append its rows.
    func loadMore() {
        guard let bridge, let resultId = resultIdData, let start = nextStartRow else { return }
        do {
            let pageBytes = try bridge.fetchPage(
                resultId: resultId, startRow: start, revision: resultRevision
            )
            let env = try PageV1.decodeEnvelope(pageBytes)
            let more = try PageV1.decodeTable(pageBytes)
            if more.rows.isEmpty {
                nextStartRow = nil
                return
            }
            if var table = catalogTable {
                table.rows.append(contentsOf: more.rows)
                catalogTable = table
                catalogSummary =
                    "result · \(table.columns.count) columns · \(table.rows.count) rows loaded"
            }
            nextStartRow = env.startRow + UInt64(env.rowCount)
        } catch {
            catalogError = "Load more failed: \(error)"
        }
    }

    func browse() {
        catalogSummary = nil
        catalogError = nil
        catalogTable = nil
        // The bridge supports execute/probe/fetch_page (not a refresh_catalog
        // intent), so the catalog browse runs an engine-appropriate listing
        // query through execute.
        let catalogSQL: String
        switch connectedEngine {
        case "postgresql":
            catalogSQL = "SELECT schemaname AS schema, tablename AS table FROM pg_tables ORDER BY 1, 2"
        case "clickhouse":
            catalogSQL = "SELECT database AS schema, name AS table FROM system.tables ORDER BY 1, 2"
        case "redis":
            // Redis execute performs a key scan (DriverPageRequest::RedisKeyScan).
            catalogSQL = ""
        default:
            catalogSummary = "catalog: no listing for \(connectedEngine)"
            return
        }
        do {
            let stmt = catalogSQL.isEmpty ? nil : catalogSQL
            if let table = try fetchPage(intent: "execute", statement: stmt) {
                catalogTable = table
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

    func runQuery() {
        let sql = queryText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !sql.isEmpty else { return }
        catalogSummary = nil
        catalogError = nil
        catalogTable = nil
        do {
            if let table = try fetchPage(intent: "execute", statement: sql) {
                catalogTable = table
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
    func applyProbeEdit() {
        guard let bridge, let session = sessionData else { return }
        reviewOutcome = nil
        reviewError = nil
        do {
            let now = UInt64(Date().timeIntervalSince1970 * 1000)
            let token = try bridge.stageProbeReview(sessionId: session, nowMs: now)
            // applyReviewToken does the authorize internally (consume-once);
            // calling authorizeReviewToken separately consumes the token first.
            let outcome = try bridge.applyReviewToken(
                tokenId: token, nowMs: now, sessionId: session, expectedRevision: 0
            )
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
            // Connection list (left sidebar).
            List(model.profiles, id: \.name) { profile in
                Button { model.connect(profile) } label: {
                    ProfileRow(profile: profile)
                }
                .buttonStyle(.plain)
            }
            .navigationTitle("Connections")
            .overlay {
                if model.profiles.isEmpty {
                    ContentUnavailableView(
                        model.bridgeError == nil ? "No profiles" : "Error",
                        systemImage: model.bridgeError == nil ? "tray" : "exclamationmark.triangle",
                        description: Text(model.bridgeError ?? "Save a profile from the TUI, then refresh.")
                    )
                }
            }
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
                        Button("Connect") { model.connectByParams() }
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
                    Button("Browse catalog") { model.browse() }
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
                        TextEditor(text: $model.queryText)
                            .font(.system(.body, design: .monospaced))
                            .frame(minHeight: 56, maxHeight: 80)
                        HStack {
                            Button("Run query") { model.runQuery() }
                                .buttonStyle(.borderedProminent)
                                .keyboardShortcut("r", modifiers: .command)
                            Button("Refresh catalog") { model.browse() }
                            Button("Apply probe edit") { model.applyProbeEdit() }
                        }
                        if let reviewOutcome = model.reviewOutcome {
                            Text(reviewOutcome).foregroundStyle(.green).font(.callout)
                        }
                        if let reviewError = model.reviewError {
                            Text(reviewError).foregroundStyle(.red).font(.callout).textSelection(.enabled)
                        }
                    }
                }
                if let table = model.catalogTable {
                    CatalogGrid(table: table)
                    if model.nextStartRow != nil {
                        Button("Load more rows") { model.loadMore() }
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
        .task { model.initialize() }
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

struct CatalogGrid: View {
    let table: PageV1Table

    var body: some View {
        ScrollView([.horizontal, .vertical]) {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 16) {
                    ForEach(table.columns.indices, id: \.self) { i in
                        Text(table.columns[i]).bold()
                            .frame(minWidth: 60, alignment: .leading)
                    }
                }
                .padding(6)
                Divider()
                ForEach(table.rows.indices, id: \.self) { r in
                    HStack(spacing: 16) {
                        ForEach(table.rows[r].indices, id: \.self) { c in
                            Text(table.rows[r][c])
                                .frame(minWidth: 60, alignment: .leading)
                        }
                    }
                    .padding(6)
                }
            }
        }
        .background(.quaternary.opacity(0.3))
        .cornerRadius(6)
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
