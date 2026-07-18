// TableRock native macOS app — plan 020 checkpoint 1 (workable vertical slice).
//
// Built with swiftc via Command Line Tools (no full Xcode required): SwiftUI +
// AppKit ship with the CLT macOS SDK, and the Rust bridge is linked as the cargo
// release dylib through the TableRockBridge SwiftPM target. Notarized XCFramework
// distribution remains the operator-gated release path (plan 019).
//
// This checkpoint proves a workable macOS application: it launches a window,
// owns a live TableRockBridge (Tokio runtime + local persistence), and reports
// bridge state. Later checkpoints add the connection list, catalog, editor,
// grid, and result surfaces over the same coarse operation/event facade.

import SwiftUI
import TableRockBridge

@main
struct TableRockApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 720, minHeight: 480)
        }
    }
}

/// Owns the live TableRockBridge for the window's lifetime.
@MainActor
final class BridgeModel: ObservableObject {
    @Published var status: String = "starting…"
    @Published var bridgeError: String?
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
            status = "Bridge ready · runtime + persistence initialized"
        } catch {
            bridgeError = "Bridge init failed: \(error)"
            status = "error"
        }
    }
}

struct ContentView: View {
    @StateObject private var model = BridgeModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("TableRock")
                .font(.largeTitle)
                .bold()
            Text(model.status)
                .foregroundStyle(.secondary)
            if let bridgeError = model.bridgeError {
                Text(bridgeError)
                    .foregroundStyle(.red)
                    .font(.callout)
                    .textSelection(.enabled)
            }
            Spacer()
            HStack {
                Text("PostgreSQL · ClickHouse · Redis")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                Spacer()
                Text("native vertical slice · checkpoint 1")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(24)
        .task { model.initialize() }
    }
}
