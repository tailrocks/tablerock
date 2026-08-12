import Foundation

enum LabConcept: String, CaseIterable, Identifiable, Sendable {
    case nativeWorkbench = "native-workbench"
    case queryStudio = "query-studio"
    case columnObservatory = "column-observatory"
    case gridCanvas = "grid-canvas"
    case changeDesk = "change-desk"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .nativeWorkbench: "Native Workbench"
        case .queryStudio: "Query Studio"
        case .columnObservatory: "Column Observatory"
        case .gridCanvas: "Grid Canvas"
        case .changeDesk: "Change Desk"
        }
    }

    var shortTitle: String {
        switch self {
        case .nativeWorkbench: "Workbench"
        case .queryStudio: "Studio"
        case .columnObservatory: "Columns"
        case .gridCanvas: "Canvas"
        case .changeDesk: "Changes"
        }
    }

    var symbol: String {
        switch self {
        case .nativeWorkbench: "macwindow"
        case .queryStudio: "text.cursor"
        case .columnObservatory: "rectangle.split.3x1"
        case .gridCanvas: "tablecells"
        case .changeDesk: "checklist"
        }
    }
}

enum LabSurface: String, CaseIterable, Identifiable, Sendable {
    case connections
    case setup
    case dataGrid = "data-grid"
    case sqlResults = "sql-results"
    case changeReview = "change-review"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .connections: "Connections"
        case .setup: "Connection Setup"
        case .dataGrid: "Data Grid"
        case .sqlResults: "SQL + Results"
        case .changeReview: "Change Review"
        }
    }

    var symbol: String {
        switch self {
        case .connections: "externaldrive.connected.to.line.below"
        case .setup: "slider.horizontal.3"
        case .dataGrid: "tablecells"
        case .sqlResults: "chevron.left.forwardslash.chevron.right"
        case .changeReview: "checkmark.seal"
        }
    }
}

enum LabAppearance: String, CaseIterable, Identifiable, Sendable {
    case system
    case light
    case dark

    var id: String { rawValue }
    var title: String { rawValue.capitalized }
}

enum LabAccessibilityMode: String, CaseIterable, Identifiable, Sendable {
    case system
    case reduceTransparency = "reduce-transparency"
    case increaseContrast = "increase-contrast"
    case reduceMotion = "reduce-motion"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .system: "System"
        case .reduceTransparency: "Reduce Transparency"
        case .increaseContrast: "Increase Contrast"
        case .reduceMotion: "Reduce Motion"
        }
    }
}

struct LabLaunchConfiguration: Equatable, Sendable {
    var concept: LabConcept = .nativeWorkbench
    var surface: LabSurface = .dataGrid
    var appearance: LabAppearance = .system
    var accessibility: LabAccessibilityMode = .system
    var captureMode = false

    static func parse(_ arguments: [String]) -> LabLaunchConfiguration {
        func value(after flag: String) -> String? {
            guard let index = arguments.firstIndex(of: flag),
                  arguments.indices.contains(index + 1)
            else { return nil }
            return arguments[index + 1]
        }

        var configuration = LabLaunchConfiguration()
        if let value = value(after: "--concept"),
           let concept = LabConcept(rawValue: value) {
            configuration.concept = concept
        }
        if let value = value(after: "--surface"),
           let surface = LabSurface(rawValue: value) {
            configuration.surface = surface
        }
        if let value = value(after: "--appearance"),
           let appearance = LabAppearance(rawValue: value) {
            configuration.appearance = appearance
        }
        if let value = value(after: "--accessibility"),
           let accessibility = LabAccessibilityMode(rawValue: value) {
            configuration.accessibility = accessibility
        }
        configuration.captureMode = arguments.contains("--capture")
        return configuration
    }
}

struct LabConnection: Identifiable, Equatable, Sendable {
    let id: String
    let name: String
    let detail: String
    let engine: String
    let environment: String
    let status: String
    let symbol: String
}

struct LabCatalogItem: Identifiable, Equatable, Sendable {
    let id: String
    let name: String
    let kind: String
    let detail: String
    let symbol: String
}

struct LabColumn: Identifiable, Equatable, Sendable {
    let id: String
    let title: String
    let type: String
    let width: CGFloat
}

struct LabRow: Identifiable, Equatable, Sendable {
    let id: Int
    let values: [String]
}

struct LabChange: Identifiable, Equatable, Sendable {
    enum Kind: String, Sendable {
        case update = "UPDATE"
        case insert = "INSERT"
        case delete = "DELETE"
    }

    let id: String
    let kind: Kind
    let object: String
    let field: String
    let before: String
    let after: String
}

enum LabFixtures {
    static let connections = [
        LabConnection(
            id: "northstar",
            name: "Northstar Analytics",
            detail: "analytics.internal:5432",
            engine: "PostgreSQL 18",
            environment: "PRODUCTION",
            status: "Connected",
            symbol: "cylinder.split.1x2"
        ),
        LabConnection(
            id: "atlas",
            name: "Atlas Events",
            detail: "events.cluster:9440",
            engine: "ClickHouse 26",
            environment: "STAGING",
            status: "12 min ago",
            symbol: "bolt.horizontal.circle"
        ),
        LabConnection(
            id: "arbor",
            name: "Arbor Cache",
            detail: "127.0.0.1:6379",
            engine: "Redis 8",
            environment: "LOCAL",
            status: "Yesterday",
            symbol: "square.stack.3d.up"
        ),
    ]

    static let catalog = [
        LabCatalogItem(id: "customers", name: "customers", kind: "Table", detail: "48.2K rows", symbol: "tablecells"),
        LabCatalogItem(id: "invoices", name: "invoices", kind: "Table", detail: "128K rows", symbol: "tablecells"),
        LabCatalogItem(id: "orders", name: "orders", kind: "Table", detail: "284K rows", symbol: "tablecells"),
        LabCatalogItem(id: "products", name: "products", kind: "Table", detail: "9.4K rows", symbol: "tablecells"),
        LabCatalogItem(id: "regions", name: "regions", kind: "View", detail: "12 rows", symbol: "eye"),
        LabCatalogItem(id: "revenue", name: "revenue_summary", kind: "View", detail: "Materialized", symbol: "eye"),
    ]

    static let columns = [
        LabColumn(id: "id", title: "customer_id", type: "UUID", width: 176),
        LabColumn(id: "name", title: "company_name", type: "TEXT", width: 176),
        LabColumn(id: "region", title: "region", type: "TEXT", width: 104),
        LabColumn(id: "plan", title: "plan", type: "TEXT", width: 104),
        LabColumn(id: "seats", title: "seats", type: "INT4", width: 80),
        LabColumn(id: "mrr", title: "monthly_revenue", type: "NUMERIC", width: 142),
        LabColumn(id: "active", title: "active", type: "BOOL", width: 78),
        LabColumn(id: "updated", title: "updated_at", type: "TIMESTAMPTZ", width: 184),
    ]

    static let rows = [
        LabRow(id: 10482, values: ["…a91f", "Aster Works", "APAC", "Scale", "124", "$18,600", "true", "2026-08-12 09:42"]),
        LabRow(id: 10481, values: ["…28cb", "Beacon & Co.", "EMEA", "Team", "38", "$4,750", "true", "2026-08-12 09:38"]),
        LabRow(id: 10480, values: ["…c773", "Cedar Systems", "AMER", "Scale", "92", "$13,800", "true", "2026-08-12 09:21"]),
        LabRow(id: 10479, values: ["…4de2", "Driftline", "APAC", "Starter", "12", "$540", "false", "2026-08-12 08:57"]),
        LabRow(id: 10478, values: ["…a602", "Ember Labs", "EMEA", "Team", "46", "$5,750", "true", "2026-08-12 08:44"]),
        LabRow(id: 10477, values: ["…8f31", "Fieldstone", "AMER", "Scale", "208", "$31,200", "true", "2026-08-12 08:30"]),
        LabRow(id: 10476, values: ["…1a0d", "Grove Digital", "APAC", "Team", "27", "$3,375", "true", "2026-08-12 08:14"]),
        LabRow(id: 10475, values: ["…b8e4", "Harbor North", "EMEA", "Starter", "8", "$360", "true", "2026-08-12 07:52"]),
        LabRow(id: 10474, values: ["…77c0", "Ion Foundry", "AMER", "Scale", "156", "$23,400", "true", "2026-08-12 07:31"]),
        LabRow(id: 10473, values: ["…09d6", "Juniper Cloud", "APAC", "Team", "64", "$8,000", "true", "2026-08-12 07:18"]),
        LabRow(id: 10472, values: ["…ef24", "Kitehouse", "EMEA", "Starter", "16", "$720", "false", "2026-08-12 06:55"]),
        LabRow(id: 10471, values: ["…339a", "Lumen River", "AMER", "Team", "51", "$6,375", "true", "2026-08-12 06:40"]),
    ]

    static let changes = [
        LabChange(id: "change-1", kind: .update, object: "customers · 10482", field: "plan", before: "Team", after: "Scale"),
        LabChange(id: "change-2", kind: .update, object: "customers · 10482", field: "seats", before: "82", after: "124"),
        LabChange(id: "change-3", kind: .insert, object: "customers · new", field: "company_name", before: "—", after: "Morrow Studio"),
        LabChange(id: "change-4", kind: .delete, object: "customers · 10479", field: "row", before: "Driftline", after: "—"),
    ]

    static let query = """
    SELECT
      company_name,
      region,
      plan,
      monthly_revenue
    FROM analytics.customers
    WHERE active = true
      AND monthly_revenue >= 5_000
    ORDER BY monthly_revenue DESC
    LIMIT 100;
    """
}
