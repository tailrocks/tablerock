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
import TableRockBridge

@main
struct TableRockApp: App {
    @StateObject private var model = BridgeModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(model)
                .frame(minWidth: 760, minHeight: 520)
        }
    }
}

/// Owns the live TableRockBridge + the profile list for the window's lifetime.
@MainActor
final class BridgeModel: ObservableObject {
    @Published var status: String = "starting…"
    @Published var bridgeError: String?
    @Published var profiles: [BridgeProfileItem] = []
    @Published var sessionHex: String?
    @Published var connectError: String?
    @Published var connectingName: String?
    @Published var catalogSummary: String?
    @Published var catalogError: String?
    @Published var catalogTable: PageV1Table?
    @Published var queryText: String = "SELECT 1;"
    @Published var reviewOutcome: String?
    @Published var reviewError: String?
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
                    return try PageV1.decodeTable(page)
                }
                if event.kind == "terminal" { return nil }
            }
            cursor = batch.nextCursor
        }
        return nil
    }

    func browse() {
        catalogSummary = nil
        catalogError = nil
        catalogTable = nil
        do {
            if let table = try fetchPage(intent: "refresh_catalog", statement: nil) {
                catalogTable = table
                catalogSummary = "catalog · \(table.columns.count) columns · \(table.rows.count) rows"
            } else {
                catalogSummary = "catalog: no page"
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
            } else {
                catalogSummary = "query: no result page (terminal)"
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
            _ = try bridge.authorizeReviewToken(
                tokenId: token, nowMs: now, sessionId: session, expectedRevision: 0
            )
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
    @EnvironmentObject private var model: BridgeModel

    var body: some View {
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
