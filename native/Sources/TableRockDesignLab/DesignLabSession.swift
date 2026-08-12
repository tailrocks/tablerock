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
        case .minimum: CGSize(width: 1_280, height: 760)
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
    @Published var fixture: LabFixtureScenario {
        didSet {
            selectedRowID = fixtureRows.first?.id
            objectMode = .data
            inspectorPresented = surface == .dataGrid && fixtureSupportsSelection
            stagedChanges = Self.initialChanges(for: fixture)
            reviewSheetPresented = fixture == .destructiveReview
        }
    }
    @Published var selectedRowID: Int?
    @Published var selectedCatalogItemID: String
    @Published var objectMode: LabObjectMode
    @Published var sortColumnID: String
    @Published var sortAscending: Bool
    @Published var inspectorPresented: Bool
    @Published var connectionSheetPresented = false
    @Published var editSheetPresented = false
    @Published var historySheetPresented = false
    @Published var reviewSheetPresented: Bool
    @Published var catalogPresented = true
    @Published var workbenchColumnVisibility = NavigationSplitViewVisibility.all
    @Published var searchText = ""
    @Published var queryRunCount = 0
    @Published var queryText: String
    @Published var queryTitle: String
    @Published var queryExecuted = true
    @Published var queryErrorText: String?
    @Published var queryHistory: [LabQueryHistoryEntry]
    @Published var stagedChanges: [LabChange]

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
        selectedCatalogItemID = "customers"
        objectMode = .data
        sortColumnID = "mrr"
        sortAscending = false
        captureMode = launch.captureMode
        windowSize = launch.windowSize
        inactiveCapture = launch.inactiveCapture
        queryText = LabFixtures.query
        queryTitle = "Revenue by region"
        queryHistory = LabFixtures.queryHistory
        stagedChanges = Self.initialChanges(for: launch.fixture)
        selectedRowID = launch.fixture == .largeResult
            ? LabFixtures.largeResultRows.first?.id
            : (launch.fixture == .empty ? nil : LabFixtures.rows.first?.id)
        inspectorPresented = launch.surface == .dataGrid
            && ![.empty, .loading, .connectionError].contains(launch.fixture)
        reviewSheetPresented = launch.fixture == .destructiveReview
        applyPresentationRoute(launch.presentation)
    }

    var fixtureRows: [LabRow] {
        fixture == .largeResult ? LabFixtures.largeResultRows : LabFixtures.rows
    }

    var displayedRows: [LabRow] {
        let index = LabFixtures.columns.firstIndex(where: { $0.id == sortColumnID }) ?? 0
        return fixtureRows.sorted { lhs, rhs in
            let order = compare(lhs.values[index], rhs.values[index], columnID: sortColumnID)
            if order == .orderedSame { return lhs.id < rhs.id }
            return sortAscending ? order == .orderedAscending : order == .orderedDescending
        }
    }

    var fixtureSupportsSelection: Bool {
        ![.empty, .loading, .connectionError].contains(fixture)
    }

    var pendingChangeCount: Int {
        stagedChanges.count
    }

    var hasDestructiveChanges: Bool {
        stagedChanges.contains { $0.kind == .delete }
    }

    var selectedCatalogItem: LabCatalogItem {
        LabFixtures.catalog.first(where: { $0.id == selectedCatalogItemID })
            ?? LabFixtures.catalog[0]
    }

    var sortDescription: String {
        let title = LabFixtures.columns.first(where: { $0.id == sortColumnID })?.title
            ?? sortColumnID
        let direction = sortAscending ? "ascending" : "descending"
        return title + " " + direction
    }

    var selectedRow: LabRow? {
        guard let selectedRowID else { return nil }
        return fixtureRows.first { $0.id == selectedRowID }
    }

    func show(_ surface: LabSurface) {
        self.surface = surface
        inspectorPresented = surface == .dataGrid
            && objectMode == .data
            && fixtureSupportsSelection
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

    func openObject(_ id: String) {
        guard LabFixtures.catalog.contains(where: { $0.id == id }) else { return }
        selectedCatalogItemID = id
        objectMode = .data
        show(.dataGrid)
    }

    func showStructure(for id: String? = nil) {
        if let id { selectedCatalogItemID = id }
        objectMode = .structure
        if surface != .dataGrid { surface = .dataGrid }
        if !inspectorPresented { inspectorPresented = true }
    }

    func setSort(columnID: String, ascending: Bool) {
        guard LabFixtures.columns.contains(where: { $0.id == columnID }) else { return }
        sortColumnID = columnID
        sortAscending = ascending
    }

    func stageSafeEdit(plan: String, seats: String) {
        guard let row = selectedRow else { return }
        let object = "\(selectedCatalogItem.name) · \(row.id)"
        var next = stagedChanges.filter { $0.object != object }
        if plan != row.values[3] {
            next.append(
                LabChange(
                    id: "safe-plan-\(row.id)",
                    kind: .update,
                    object: object,
                    field: "plan",
                    before: row.values[3],
                    after: plan
                )
            )
        }
        if seats != row.values[4] {
            next.append(
                LabChange(
                    id: "safe-seats-\(row.id)",
                    kind: .update,
                    object: object,
                    field: "seats",
                    before: row.values[4],
                    after: seats
                )
            )
        }
        stagedChanges = next
        editSheetPresented = false
    }

    func presentReview() {
        guard !stagedChanges.isEmpty else { return }
        reviewSheetPresented = true
    }

    func discardChanges() {
        stagedChanges = []
        reviewSheetPresented = false
        show(.dataGrid)
    }

    func applyChanges() {
        stagedChanges = []
        reviewSheetPresented = false
        show(.dataGrid)
    }

    func createQuery() {
        queryTitle = "Untitled Query"
        queryText = ""
        queryExecuted = false
        queryErrorText = nil
        show(.sqlResults)
    }

    func openSavedQuery() {
        queryTitle = "Revenue by region"
        queryText = LabFixtures.query
        queryExecuted = true
        queryErrorText = nil
        show(.sqlResults)
    }

    func runQuery() {
        guard !queryText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            queryExecuted = false
            queryErrorText = "Enter SQL before running."
            return
        }
        queryRunCount += 1
        queryExecuted = true
        queryErrorText = nil
        queryHistory.insert(
            LabQueryHistoryEntry(
                id: "history-run-\(queryRunCount)",
                title: queryTitle,
                statement: queryText,
                executedAt: "Just now",
                duration: "41 ms",
                rowCount: "12 rows",
                engine: engine
            ),
            at: 0
        )
        show(.sqlResults)
    }

    func openHistoryEntry(_ entry: LabQueryHistoryEntry) {
        queryTitle = entry.title
        queryText = entry.statement
        queryExecuted = true
        queryErrorText = nil
        engine = entry.engine
        historySheetPresented = false
        show(.sqlResults)
    }

    private static func initialChanges(for fixture: LabFixtureScenario) -> [LabChange] {
        switch fixture {
        case .pendingChange: Array(LabFixtures.changes.prefix(1))
        case .destructiveReview: LabFixtures.changes
        default: []
        }
    }

    private func applyPresentationRoute(_ route: LabPresentationRoute) {
        switch route {
        case .standard:
            break
        case .structure:
            surface = .dataGrid
            objectMode = .structure
            inspectorPresented = true
        case .connectionSheet:
            surface = .connections
            inspectorPresented = false
            connectionSheetPresented = true
        case .safeEdit:
            surface = .dataGrid
            objectMode = .data
            inspectorPresented = true
            editSheetPresented = true
        case .queryHistory:
            surface = .sqlResults
            inspectorPresented = false
            historySheetPresented = true
        case .safeReview:
            surface = .dataGrid
            objectMode = .data
            inspectorPresented = true
            stagedChanges = Array(LabFixtures.changes.prefix(2))
            reviewSheetPresented = true
        case .queryError:
            surface = .sqlResults
            inspectorPresented = false
            queryTitle = "Untitled Query"
            queryText = ""
            queryExecuted = false
            queryErrorText = "Enter SQL before running."
        }
    }

    private func compare(
        _ lhs: String,
        _ rhs: String,
        columnID: String
    ) -> ComparisonResult {
        if ["seats", "mrr"].contains(columnID),
           let left = numericValue(lhs),
           let right = numericValue(rhs) {
            if left < right { return .orderedAscending }
            if left > right { return .orderedDescending }
            return .orderedSame
        }
        return lhs.localizedStandardCompare(rhs)
    }

    private func numericValue(_ value: String) -> Double? {
        Double(value.filter { $0.isNumber || $0 == "." || $0 == "-" })
    }
}

struct LabCommands: Commands {
    @ObservedObject var session: LabSession

    var body: some Commands {
        CommandGroup(after: .newItem) {
            Button("New Query") { session.createQuery() }
                .keyboardShortcut("t", modifiers: [.command])
            Button("New Connection…") { session.connectionSheetPresented = true }
                .keyboardShortcut("n", modifiers: [.command, .shift])
        }

        CommandMenu("Navigate") {
            ForEach(LabSurface.allCases) { surface in
                Button(surface.title) { session.show(surface) }
                    .keyboardShortcut(KeyEquivalent(Character(String(surfaceIndex(surface)))), modifiers: [.command, .option])
            }
            Divider()
            Button("Object Structure") { session.showStructure() }
                .keyboardShortcut("s", modifiers: [.command, .option])
            Button("Query History…") { session.historySheetPresented = true }
                .keyboardShortcut("h", modifiers: [.command, .option])
        }

        CommandMenu("Database") {
            Picker("Engine", selection: $session.engine) {
                ForEach(LabEngine.allCases) { engine in
                    Label(engine.title, systemImage: engine.symbol).tag(engine)
                }
            }
            Button("Run Query") { session.runQuery() }
                .keyboardShortcut(.return, modifiers: [.command])
            Button("Review Pending Changes…") { session.presentReview() }
                .keyboardShortcut("r", modifiers: [.command, .shift])
                .disabled(session.pendingChangeCount == 0)
        }

        CommandMenu("Design Lab") {
            Menu("Window Size") {
                ForEach(LabWindowSize.allCases) { size in
                    Button(size.title) { LabWindowSizing.resize(to: size) }
                }
            }
        }
    }

    private func surfaceIndex(_ surface: LabSurface) -> Int {
        (LabSurface.allCases.firstIndex(of: surface) ?? 0) + 1
    }
}
