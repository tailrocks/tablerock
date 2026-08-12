import Combine
import SwiftUI

enum LabEngine: String, CaseIterable, Identifiable, Sendable {
    case postgresql
    case clickHouse = "clickhouse"
    case redis

    var id: String { rawValue }

    var title: String {
        switch self {
        case .postgresql: "PostgreSQL"
        case .clickHouse: "ClickHouse"
        case .redis: "Redis"
        }
    }

    var connectionName: String {
        switch self {
        case .postgresql: "Northstar Analytics"
        case .clickHouse: "Atlas Events"
        case .redis: "Arbor Cache"
        }
    }

    var symbol: String {
        switch self {
        case .postgresql: "cylinder.split.1x2"
        case .clickHouse: "bolt.horizontal.circle"
        case .redis: "square.stack.3d.up"
        }
    }
}

enum LabFixtureScenario: String, CaseIterable, Identifiable, Sendable {
    case populated
    case empty
    case loading
    case connectionError = "connection-error"
    case largeResult = "large-result"
    case longIdentifiers = "long-identifiers"
    case selectedCell = "selected-cell"
    case pendingChange = "pending-change"
    case destructiveReview = "destructive-review"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .populated: "Populated"
        case .empty: "Empty"
        case .loading: "Loading"
        case .connectionError: "Connection Error"
        case .largeResult: "Large Result"
        case .longIdentifiers: "Long Identifiers"
        case .selectedCell: "Selected Cell"
        case .pendingChange: "Pending Change"
        case .destructiveReview: "Destructive Review"
        }
    }
}

enum LabWindowSize: String, CaseIterable, Identifiable, Sendable {
    case minimum
    case typical
    case expanded

    var id: String { rawValue }

    var title: String { rawValue.capitalized }

    var dimensions: CGSize {
        switch self {
        case .minimum: CGSize(width: 980, height: 680)
        case .typical: CGSize(width: 1_440, height: 900)
        case .expanded: CGSize(width: 1_720, height: 1_040)
        }
    }
}

@MainActor
final class LabSession: ObservableObject {
    @Published var concept: LabConcept
    @Published var surface: LabSurface
    @Published var appearance: LabAppearance
    @Published var accessibility: LabAccessibilityMode
    @Published var engine: LabEngine
    @Published var fixture: LabFixtureScenario
    @Published var selectedRowID: Int?
    @Published var inspectorPresented: Bool
    @Published var connectionSheetPresented = false
    @Published var reviewSheetPresented: Bool
    @Published var catalogPresented = true
    @Published var searchText = ""
    @Published var queryRunCount = 0

    let captureMode: Bool
    let windowSize: LabWindowSize
    let inactiveCapture: Bool

    init(launch: LabLaunchConfiguration) {
        concept = launch.concept
        surface = launch.surface
        appearance = launch.appearance
        accessibility = launch.accessibility
        engine = launch.engine
        fixture = launch.fixture
        captureMode = launch.captureMode
        windowSize = launch.windowSize
        inactiveCapture = launch.inactiveCapture
        selectedRowID = launch.fixture == .empty ? nil : LabFixtures.rows.first?.id
        inspectorPresented = launch.surface == .dataGrid
        reviewSheetPresented = launch.fixture == .destructiveReview
    }

    var selectedRow: LabRow? {
        guard let selectedRowID else { return nil }
        return LabFixtures.rows.first { $0.id == selectedRowID }
    }

    func show(_ surface: LabSurface) {
        self.surface = surface
        inspectorPresented = surface == .dataGrid
    }

    func openConnection(_ connection: LabConnection) {
        switch connection.id {
        case "atlas": engine = .clickHouse
        case "arbor": engine = .redis
        default: engine = .postgresql
        }
        show(.dataGrid)
    }

    func selectRow(_ id: Int) {
        selectedRowID = id
        inspectorPresented = true
    }

    func runQuery() {
        queryRunCount += 1
        show(.sqlResults)
    }
}

struct LabCommands: Commands {
    @ObservedObject var session: LabSession

    var body: some Commands {
        CommandGroup(after: .newItem) {
            Button("New Query") { session.show(.sqlResults) }
                .keyboardShortcut("t", modifiers: [.command])
            Button("New Connection…") { session.connectionSheetPresented = true }
                .keyboardShortcut("n", modifiers: [.command, .shift])
        }

        CommandMenu("Navigate") {
            ForEach(LabSurface.allCases) { surface in
                Button(surface.title) { session.show(surface) }
                    .keyboardShortcut(KeyEquivalent(Character(String(surfaceIndex(surface)))), modifiers: [.command, .option])
            }
        }

        CommandMenu("Database") {
            Picker("Engine", selection: $session.engine) {
                ForEach(LabEngine.allCases) { engine in
                    Label(engine.title, systemImage: engine.symbol).tag(engine)
                }
            }
            Button("Run Query") { session.runQuery() }
                .keyboardShortcut(.return, modifiers: [.command])
            Button("Review Pending Changes…") { session.reviewSheetPresented = true }
                .keyboardShortcut("r", modifiers: [.command, .shift])
        }
    }

    private func surfaceIndex(_ surface: LabSurface) -> Int {
        (LabSurface.allCases.firstIndex(of: surface) ?? 0) + 1
    }
}
