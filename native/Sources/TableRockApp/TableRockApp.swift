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
    var bridge: TableRockBridge?

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
}

struct ContentView: View {
    @EnvironmentObject private var model: BridgeModel

    var body: some View {
        NavigationSplitView {
            // Connection list (left sidebar).
            List(model.profiles, id: \.name) { profile in
                ProfileRow(profile: profile)
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
