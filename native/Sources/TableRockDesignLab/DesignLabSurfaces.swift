import SwiftUI

struct LabSurfaceContent: View {
    @EnvironmentObject private var session: LabSession

    let surface: LabSurface
    var compact = false

    @ViewBuilder
    var body: some View {
        Group {
            switch session.fixture {
            case .empty:
                LabScenarioStateView(
                    title: "No objects yet",
                    detail: "Choose a database context or create a connection.",
                    symbol: "tray"
                )
            case .loading:
                LabScenarioStateView(
                    title: "Loading \(session.engine.title) metadata",
                    detail: "The static scenario keeps content work outside rendering.",
                    symbol: "arrow.triangle.2.circlepath",
                    loading: true
                )
            case .connectionError:
                LabScenarioStateView(
                    title: "Connection unavailable",
                    detail: "TLS negotiation failed. Credentials and values remain redacted.",
                    symbol: "exclamationmark.icloud"
                )
            default:
                surfaceContent
            }
        }
    }

    @ViewBuilder
    private var surfaceContent: some View {
        switch surface {
        case .connections:
            LabConnectionsSurface(compact: compact)
        case .setup:
            LabConnectionSetupSurface(compact: compact)
        case .dataGrid:
            LabDataGridSurface(compact: compact)
        case .sqlResults:
            LabSQLResultsSurface(compact: compact)
        case .changeReview:
            LabChangeReviewSurface(compact: compact)
        }
    }
}

private struct LabScenarioStateView: View {
    let title: String
    let detail: String
    let symbol: String
    var loading = false

    var body: some View {
        ContentUnavailableView {
            Label(title, systemImage: symbol)
        } description: {
            Text(detail)
        } actions: {
            if loading { ProgressView().controlSize(.small) }
        }
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityIdentifier("design-lab-scenario-state")
    }
}

struct LabConnectionsSurface: View {
    @EnvironmentObject private var session: LabSession

    var compact = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .center, spacing: 12) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Connections")
                        .font(.title2.weight(.semibold))
                    Text("Open a workspace or configure a data source")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button("New Connection", systemImage: "plus") {
                    session.connectionSheetPresented = true
                }
                    .buttonStyle(.borderedProminent)
            }
            .padding(.horizontal, compact ? 16 : 24)
            .frame(height: 76)

            Divider()

            ScrollView {
                VStack(spacing: 18) {
                    HStack(spacing: 8) {
                        Image(systemName: "magnifyingglass")
                            .foregroundStyle(.secondary)
                        Text("Search connections")
                            .foregroundStyle(.secondary)
                        Spacer()
                        Text("⌘K")
                            .font(.caption)
                            .foregroundStyle(.tertiary)
                    }
                    .padding(.horizontal, 12)
                    .frame(height: 36)
                    .background(Color(nsColor: .controlBackgroundColor), in: .rect(cornerRadius: 9))
                    .overlay { RoundedRectangle(cornerRadius: 9).stroke(Color(nsColor: .separatorColor), lineWidth: 0.5) }

                    VStack(alignment: .leading, spacing: 10) {
                        Text("FAVORITES")
                            .font(.caption2.weight(.semibold))
                            .foregroundStyle(.secondary)

                        LazyVGrid(
                            columns: [GridItem(.adaptive(minimum: compact ? 230 : 280), spacing: 12)],
                            spacing: 12
                        ) {
                            ForEach(LabFixtures.connections) { connection in
                                Button {
                                    session.openConnection(connection)
                                } label: {
                                    LabConnectionCard(connection: connection)
                                }
                                .buttonStyle(.plain)
                            }
                            Button {
                                session.connectionSheetPresented = true
                            } label: {
                                LabNewConnectionCard()
                            }
                            .buttonStyle(.plain)
                        }
                    }

                    HStack {
                        Label("Connection values are local to this preview", systemImage: "lock.shield")
                        Spacer()
                        Button("Import…") {}
                        Button("Manage Groups…") {}
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .padding(compact ? 16 : 24)
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

private struct LabConnectionCard: View {
    let connection: LabConnection

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                Image(systemName: connection.symbol)
                    .font(.title2)
                    .foregroundStyle(connection.id == "northstar" ? Color.blue : .secondary)
                    .frame(width: 38, height: 38)
                    .background(Color.accentColor.opacity(0.09), in: .rect(cornerRadius: 9))
                VStack(alignment: .leading, spacing: 3) {
                    Text(connection.name)
                        .font(.headline)
                    Text(connection.engine)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Menu("More", systemImage: "ellipsis") {
                    Button("Edit") {}
                    Button("Duplicate") {}
                    Divider()
                    Button("Remove", role: .destructive) {}
                }
                .menuStyle(.borderlessButton)
                .labelStyle(.iconOnly)
            }

            Text(connection.detail)
                .font(.caption.monospaced())
                .foregroundStyle(.secondary)

            HStack {
                LabBadge(
                    text: connection.environment,
                    tint: connection.environment == "PRODUCTION" ? .orange : .secondary,
                    symbol: connection.environment == "PRODUCTION" ? "exclamationmark.triangle.fill" : nil
                )
                Spacer()
                Label(connection.status, systemImage: connection.status == "Connected" ? "circle.fill" : "clock")
                    .foregroundStyle(connection.status == "Connected" ? Color.green : .secondary)
                    .font(.caption)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, minHeight: 146, alignment: .topLeading)
        .background(Color(nsColor: .controlBackgroundColor), in: .rect(cornerRadius: 12))
        .overlay { RoundedRectangle(cornerRadius: 12).stroke(Color(nsColor: .separatorColor), lineWidth: 0.5) }
        .contentShape(.rect)
        .accessibilityElement(children: .combine)
    }
}

private struct LabNewConnectionCard: View {
    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: "plus.circle")
                .font(.title2)
            Text("New Connection")
                .font(.headline)
            Text("PostgreSQL, ClickHouse, or Redis")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .foregroundStyle(.secondary)
        .frame(maxWidth: .infinity, minHeight: 176)
        .background(Color(nsColor: .controlBackgroundColor).opacity(0.45), in: .rect(cornerRadius: 12))
        .overlay {
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color(nsColor: .separatorColor), style: StrokeStyle(lineWidth: 1, dash: [5]))
        }
    }
}

struct LabConnectionSetupSurface: View {
    @EnvironmentObject private var session: LabSession

    var compact = false

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("New PostgreSQL Connection")
                        .font(.title2.weight(.semibold))
                    Text("Configure identity, endpoint, and connection safety")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                LabBadge(text: "NOT SAVED", tint: .secondary, symbol: "circle.dashed")
            }
            .padding(.horizontal, compact ? 16 : 24)
            .frame(height: 76)
            Divider()

            HSplitView {
                ScrollView {
                    VStack(alignment: .leading, spacing: 22) {
                        LabFormSection(title: "GENERAL", subtitle: "Name and engine") {
                            LabFormField(label: "Name", value: "Northstar Analytics")
                            LabSegmentedField(label: "Engine", values: ["PostgreSQL", "ClickHouse", "Redis"], selection: "PostgreSQL")
                            LabFormField(label: "Color", value: "Ocean Blue", symbol: "circle.fill", tint: .blue)
                        }

                        LabFormSection(title: "SERVER", subtitle: "Direct endpoint") {
                            LabFormField(label: "Host", value: "analytics.internal")
                            HStack(spacing: 10) {
                                LabFormField(label: "Port", value: "5432")
                                LabFormField(label: "Database", value: "analytics")
                            }
                            LabFormField(label: "User", value: "table_operator")
                            LabFormField(label: "Secret", value: "••••••••••••", symbol: "key.fill")
                        }

                        LabFormSection(title: "SECURITY", subtitle: "Transport and write protection") {
                            LabToggleField(label: "Require TLS", detail: "Verify server certificate", enabled: true)
                            LabToggleField(label: "Use SSH tunnel", detail: "Connect through a bastion", enabled: false)
                            LabToggleField(label: "Safe mode", detail: "Review writes before execution", enabled: true)
                        }
                    }
                    .padding(compact ? 16 : 24)
                }
                .frame(minWidth: 480)

                if !compact {
                    LabConnectionSummary()
                        .frame(minWidth: 260, idealWidth: 300, maxWidth: 340)
                }
            }

            Divider()
            HStack {
                Button("Cancel") {}
                Spacer()
                Button("Test Connection", systemImage: "wave.3.right") {
                    session.connectionSheetPresented = true
                }
                Button("Save & Connect", systemImage: "arrow.right") {
                    session.show(.dataGrid)
                }
                    .buttonStyle(.borderedProminent)
            }
            .padding(.horizontal, 18)
            .frame(height: 52)
            .background(Color(nsColor: .windowBackgroundColor))
        }
        .background(Color(nsColor: .controlBackgroundColor))
    }
}

private struct LabFormSection<Content: View>: View {
    let title: String
    let subtitle: String
    @ViewBuilder let content: Content

    init(title: String, subtitle: String, @ViewBuilder content: () -> Content) {
        self.title = title
        self.subtitle = subtitle
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Text(title).font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
                Text(subtitle).font(.caption2).foregroundStyle(.tertiary)
            }
            VStack(spacing: 10) { content }
                .padding(14)
                .background(Color(nsColor: .windowBackgroundColor), in: .rect(cornerRadius: 10))
                .overlay { RoundedRectangle(cornerRadius: 10).stroke(Color(nsColor: .separatorColor), lineWidth: 0.5) }
        }
    }
}

private struct LabFormField: View {
    let label: String
    let value: String
    var symbol: String?
    var tint: Color = .secondary

    var body: some View {
        HStack(spacing: 12) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 76, alignment: .trailing)
            HStack {
                if let symbol {
                    Image(systemName: symbol).foregroundStyle(tint)
                }
                Text(value)
                    .font(.callout)
                Spacer()
            }
            .padding(.horizontal, 9)
            .frame(height: 30)
            .background(Color(nsColor: .textBackgroundColor), in: .rect(cornerRadius: 6))
            .overlay { RoundedRectangle(cornerRadius: 6).stroke(Color(nsColor: .separatorColor), lineWidth: 0.5) }
        }
    }
}

private struct LabSegmentedField: View {
    let label: String
    let values: [String]
    let selection: String

    var body: some View {
        HStack(spacing: 12) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 76, alignment: .trailing)
            Picker(label, selection: .constant(selection)) {
                ForEach(values, id: \.self) { Text($0).tag($0) }
            }
            .labelsHidden()
            .pickerStyle(.segmented)
        }
    }
}

private struct LabToggleField: View {
    let label: String
    let detail: String
    let enabled: Bool

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(label).font(.callout)
                Text(detail).font(.caption2).labSecondaryForeground()
            }
            Spacer()
            Toggle(label, isOn: .constant(enabled))
                .labelsHidden()
                .toggleStyle(.switch)
        }
    }
}

private struct LabConnectionSummary: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Image(systemName: "cylinder.split.1x2")
                .font(.largeTitle)
                .foregroundStyle(.blue)
                .frame(width: 58, height: 58)
                .background(.blue.opacity(0.1), in: .rect(cornerRadius: 14))
            VStack(alignment: .leading, spacing: 4) {
                Text("Northstar Analytics").font(.headline)
                Text("PostgreSQL 18").foregroundStyle(.secondary)
            }
            Divider()
            LabeledContent("Endpoint", value: "analytics.internal:5432")
            LabeledContent("Database", value: "analytics")
            LabeledContent("TLS", value: "Required")
            LabeledContent("Safe mode", value: "On")
            Divider()
            Label("No connection test has run", systemImage: "info.circle")
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
        }
        .font(.caption)
        .padding(22)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

struct LabDataGridSurface: View {
    @EnvironmentObject private var session: LabSession

    var compact = false

    var body: some View {
        VStack(spacing: 0) {
            LabObjectHeader(title: gridTitle, detail: gridDetail, symbol: "tablecells") {
                if session.objectMode == .data {
                    Button("Edit Selected", systemImage: "pencil") {
                        session.editSheetPresented = true
                    }
                    .disabled(session.selectedRow == nil)
                    .accessibilityIdentifier("design-lab-edit-selected")
                } else {
                    Button("Open Data", systemImage: "tablecells") {
                        session.objectMode = .data
                    }
                }
                Button("Export", systemImage: "square.and.arrow.up") {}
            }
            if session.objectMode == .data {
                LabFilterRail()
                LabDataTable(compact: compact)
            } else {
                LabStructureSurface(compact: compact)
            }
            LabStatusBar()
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    private var gridTitle: String {
        if session.fixture == .longIdentifiers {
            "enterprise_customer_subscription_entitlements_archive"
        } else if session.engine == .redis {
            "customer:session:*"
        } else {
            session.selectedCatalogItem.name
        }
    }

    private var gridDetail: String {
        switch session.engine {
        case .postgresql: "analytics.public · PostgreSQL"
        case .clickHouse: "analytics.events · ClickHouse"
        case .redis: "database 0 · Redis"
        }
    }
}

private struct LabObjectHeader<Actions: View>: View {
    @Environment(\.labAccessibilityPreview) private var preview

    let title: String
    let detail: String
    let symbol: String
    @ViewBuilder let actions: Actions

    init(title: String, detail: String, symbol: String, @ViewBuilder actions: () -> Actions) {
        self.title = title
        self.detail = detail
        self.symbol = symbol
        self.actions = actions()
    }

    var body: some View {
        HStack(spacing: 9) {
            Image(systemName: symbol)
                .foregroundStyle(.blue)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(.headline)
                Text(detail)
                    .font(.caption2)
                    .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                    .labSecondaryForeground()
            }
            Spacer()
            actions
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .frame(height: 48)
        .background(Color(nsColor: .windowBackgroundColor))
        .overlay(alignment: .bottom) { Divider() }
    }
}

private struct LabFilterRail: View {
    @EnvironmentObject private var session: LabSession

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "line.3.horizontal.decrease")
                .labSecondaryForeground()
            LabFilterToken(text: "active", relation: "is", value: "true")
            LabFilterToken(text: "region", relation: "is not", value: "LATAM")
            Button("Add filter", systemImage: "plus") {}
                .buttonStyle(.plain)
                .labSecondaryForeground()
            Spacer()
            Menu {
                Button("Company Name, A to Z") {
                    session.setSort(columnID: "name", ascending: true)
                }
                Button("Company Name, Z to A") {
                    session.setSort(columnID: "name", ascending: false)
                }
                Divider()
                Button("Monthly Revenue, High to Low") {
                    session.setSort(columnID: "mrr", ascending: false)
                }
                Button("Monthly Revenue, Low to High") {
                    session.setSort(columnID: "mrr", ascending: true)
                }
            } label: {
                Label("Sorted by \(session.sortDescription)", systemImage: "arrow.up.arrow.down")
                    .font(.caption2)
            }
            .menuStyle(.borderlessButton)
            .accessibilityIdentifier("design-lab-sort-menu")
            Button(action: {}) { Image(systemName: "xmark.circle") }
                .buttonStyle(.plain)
                .help("Clear filters")
        }
        .font(.caption)
        .padding(.horizontal, 10)
        .frame(height: 38)
        .background(Color(nsColor: .controlBackgroundColor))
        .overlay(alignment: .bottom) { Divider() }
    }
}

private struct LabFilterToken: View {
    @Environment(\.labAccessibilityPreview) private var preview

    let text: String
    let relation: String
    let value: String

    var body: some View {
        HStack(spacing: 4) {
            Text(text).fontWeight(.semibold).labPrimaryForeground()
            Text(relation).labSecondaryForeground()
            Text(value).labPrimaryForeground()
            Image(systemName: "xmark").font(.system(size: 7, weight: .bold)).labTertiaryForeground()
        }
        .fontWeight(preview == .increaseContrast ? .semibold : .regular)
        .padding(.horizontal, 7)
        .frame(height: 24)
        .background(Color.accentColor.opacity(0.1), in: .rect(cornerRadius: 6))
    }
}

struct LabDataTable: View {
    @EnvironmentObject private var session: LabSession

    var compact = false

    private var columns: ArraySlice<LabColumn> {
        LabFixtures.columns.prefix(compact ? 5 : 8)
    }

    var body: some View {
        LabNativeDataTable(
            rows: session.displayedRows,
            columns: Array(columns),
            selectedRowID: $session.selectedRowID,
            sortColumnID: $session.sortColumnID,
            sortAscending: $session.sortAscending,
            openInspector: { session.inspectorPresented = true },
            openQuery: { session.openSavedQuery() },
            reviewChanges: { session.presentReview() }
        )
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Customer result grid, \(session.displayedRows.count) preview rows")
    }
}

private struct LabStructureSurface: View {
    @EnvironmentObject private var session: LabSession

    var compact = false

    var body: some View {
        HStack(spacing: 0) {
            Table(LabFixtures.columns) {
                TableColumn("Column") { column in
                    HStack(spacing: 7) {
                        Image(systemName: column.id == "id" ? "key.fill" : "textformat")
                            .foregroundStyle(column.id == "id" ? Color.orange : .secondary)
                        Text(column.title)
                            .font(.body.monospaced())
                    }
                }
                TableColumn("Type") { column in
                    Text(column.type).font(.body.monospaced())
                }
                TableColumn("Nullable") { column in
                    Text(column.id == "updated" ? "YES" : "NO")
                }
                TableColumn("Default") { column in
                    Text(column.id == "updated" ? "now()" : "—")
                        .font(.body.monospaced())
                }
            }
            .accessibilityLabel("\(session.selectedCatalogItem.name) columns")
            .accessibilityIdentifier("design-lab-structure-table")
            .frame(minWidth: 500)

            if !compact {
                Divider()
                List {
                    Section("Indexes") {
                        Label("customers_pkey", systemImage: "key.fill")
                        Label("customers_region_idx", systemImage: "list.bullet.rectangle")
                    }
                    Section("Constraints") {
                        Label("PRIMARY KEY · customer_id", systemImage: "checkmark.seal")
                        Label("FOREIGN KEY · region_id", systemImage: "link")
                    }
                    Section("Relations") {
                        Label("orders.customer_id", systemImage: "arrow.right")
                        Label("invoices.customer_id", systemImage: "arrow.right")
                    }
                }
                .listStyle(.sidebar)
                .frame(width: 300)
                .accessibilityLabel("Structure details")
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("design-lab-structure")
    }
}

struct LabSQLResultsSurface: View {
    @EnvironmentObject private var session: LabSession

    var compact = false

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 0) {
                LabObjectHeader(title: session.queryTitle, detail: "Query · \(session.engine.connectionName)", symbol: "chevron.left.forwardslash.chevron.right") {
                    Button("Explain", systemImage: "chart.xyaxis.line") {}
                    Button("Run", systemImage: "play.fill") { session.runQuery() }
                        .buttonStyle(.borderedProminent)
                        .tint(.green)
                        .keyboardShortcut(.return, modifiers: .command)
                }
                LabCodeEditor()
            }
            .frame(minHeight: compact ? 210 : 250, maxHeight: .infinity)

            Divider()

            VStack(spacing: 0) {
                HStack(spacing: 12) {
                    Picker("Result", selection: .constant("Results")) {
                        Text("Results").tag("Results")
                        Text("Messages").tag("Messages")
                        Text("Plan").tag("Plan")
                    }
                    .labelsHidden()
                    .pickerStyle(.segmented)
                    .frame(width: 210)
                    Spacer()
                    if let queryErrorText = session.queryErrorText {
                        Text(queryErrorText)
                            .foregroundStyle(.red)
                        LabBadge(text: "NOT RUN", tint: .red, symbol: "exclamationmark.circle")
                    } else if session.queryExecuted {
                        Label(session.queryRunCount == 0 ? "100 rows" : "12 rows", systemImage: "tablecells")
                        Text(session.queryRunCount == 0 ? "126 ms" : "41 ms")
                        LabBadge(text: "SUCCESS", tint: .green, symbol: "checkmark.circle.fill")
                            .accessibilityIdentifier("design-lab-query-success")
                    } else {
                        Text("Enter SQL and press ⌘↩")
                            .foregroundStyle(.secondary)
                        LabBadge(text: "NOT RUN", tint: .secondary, symbol: "circle.dashed")
                    }
                }
                .font(.caption)
                .padding(.horizontal, 10)
                .frame(height: 38)
                .background(Color(nsColor: .windowBackgroundColor))
                .overlay(alignment: .bottom) { Divider() }
                LabDataTable(compact: compact)
            }
            .frame(minHeight: 240, maxHeight: .infinity)
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}

private struct LabCodeEditor: View {
    @EnvironmentObject private var session: LabSession
    @FocusState private var editorFocused: Bool

    private var lineCount: Int {
        max(session.queryText.split(separator: "\n", omittingEmptySubsequences: false).count, 1)
    }

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            ScrollView(.vertical) {
                VStack(alignment: .trailing, spacing: 0) {
                    ForEach(0..<lineCount, id: \.self) { index in
                        Text(String(index + 1)).frame(height: 22)
                    }
                }
                .font(.caption.monospacedDigit())
                .foregroundStyle(.tertiary)
                .padding(.horizontal, 10)
                .padding(.vertical, 12)
            }
            .scrollDisabled(true)
            .frame(width: 48)
            .background(Color(nsColor: .controlBackgroundColor).opacity(0.7))

            TextEditor(text: $session.queryText)
                .focused($editorFocused)
                .font(.system(size: 13, design: .monospaced))
                .scrollContentBackground(.hidden)
                .padding(6)
                .accessibilityLabel("SQL query editor")
                .accessibilityIdentifier("design-lab-query-editor")
        }
        .background(Color(nsColor: .textBackgroundColor))
        .task {
            guard session.queryTitle == "Untitled Query" else { return }
            editorFocused = true
        }
    }
}

struct LabChangeReviewSurface: View {
    @EnvironmentObject private var session: LabSession

    var compact = false

    private var changes: [LabChange] {
        session.stagedChanges
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Review Changes")
                        .font(.title2.weight(.semibold))
                    Text("Northstar Analytics · analytics.public · \(changes.count) staged operation\(changes.count == 1 ? "" : "s")")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                LabBadge(text: "PRODUCTION", tint: .orange, symbol: "exclamationmark.triangle.fill")
                LabBadge(text: "SAFE MODE", tint: .green, symbol: "lock.shield.fill")
            }
            .padding(.horizontal, compact ? 16 : 22)
            .frame(height: 70)
            .background(Color(nsColor: .windowBackgroundColor))
            .overlay(alignment: .bottom) { Divider() }

            if changes.isEmpty {
                ContentUnavailableView(
                    "No Pending Changes",
                    systemImage: "checkmark.circle",
                    description: Text("Edit a row to stage a safe change for review.")
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                HStack(spacing: 0) {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 12) {
                            HStack {
                                Text("STAGED OPERATIONS")
                                    .font(.caption2.weight(.semibold))
                                    .foregroundStyle(.secondary)
                                Spacer()
                                Text(changeSummary)
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                            }
                            ForEach(changes) { change in
                                LabReviewChangeRow(change: change)
                            }
                        }
                        .padding(16)
                    }
                    .frame(minWidth: 440, maxWidth: .infinity)

                    if !compact {
                        Divider()
                        LabSQLPreview(changes: changes)
                            .frame(width: 400)
                    }
                }
            }

            Divider()
            HStack {
                Button("Back to Editing") { session.show(.dataGrid) }
                Button("Discard All", role: .destructive) { session.discardChanges() }
                    .disabled(changes.isEmpty)
                Spacer()
                VStack(alignment: .trailing, spacing: 1) {
                    Text("\(changes.count) change\(changes.count == 1 ? "" : "s") will run in one transaction")
                        .font(.caption.weight(.medium))
                    Text("Rollback on any failure")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
                Button("Review & Apply", systemImage: "checkmark") {
                    session.presentReview()
                }
                    .buttonStyle(.borderedProminent)
                    .tint(.orange)
                    .disabled(changes.isEmpty)
            }
            .padding(.horizontal, 16)
            .frame(height: 58)
            .background(Color(nsColor: .windowBackgroundColor))
        }
        .background(Color(nsColor: .controlBackgroundColor))
    }

    private var changeSummary: String {
        let updates = changes.count { $0.kind == .update }
        let inserts = changes.count { $0.kind == .insert }
        let deletes = changes.count { $0.kind == .delete }
        return "\(updates) update\(updates == 1 ? "" : "s") · \(inserts) insert\(inserts == 1 ? "" : "s") · \(deletes) delete\(deletes == 1 ? "" : "s")"
    }
}

private struct LabReviewChangeRow: View {
    let change: LabChange

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            HStack {
                LabBadge(text: change.kind.rawValue, tint: change.kind == .delete ? .red : .orange)
                Text(change.object).font(.headline)
                Spacer()
                Button(action: {}) { Image(systemName: "trash") }
                    .buttonStyle(.plain)
                    .help("Discard this change")
            }
            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 5) {
                GridRow {
                    Text("Field").foregroundStyle(.secondary)
                    Text("Before").foregroundStyle(.secondary)
                    Text("After").foregroundStyle(.secondary)
                }
                GridRow {
                    Text(change.field).fontWeight(.medium)
                    Text(change.before).strikethrough(change.kind != .insert).foregroundStyle(.secondary)
                    Text(change.after).foregroundStyle(change.kind == .delete ? .secondary : .primary)
                }
            }
            .font(.caption.monospaced())
        }
        .padding(13)
        .background(Color(nsColor: .windowBackgroundColor), in: .rect(cornerRadius: 10))
        .overlay { RoundedRectangle(cornerRadius: 10).stroke(Color(nsColor: .separatorColor), lineWidth: 0.5) }
    }
}

private struct LabSQLPreview: View {
    let changes: [LabChange]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("SQL PREVIEW")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Copy", systemImage: "doc.on.doc") {}
                    .buttonStyle(.plain)
            }
            .padding(.horizontal, 12)
            .frame(height: 42)
            Divider()
            Text(sqlPreview)
            .font(.system(size: 12, design: .monospaced))
            .textSelection(.enabled)
            .padding(14)
            Spacer()
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    private var sqlPreview: String {
        let statements = changes.map { change in
            switch change.kind {
            case .update:
                "UPDATE analytics.customers\nSET \(change.field) = '\(change.after)'\nWHERE customer_id = $1;"
            case .insert:
                "INSERT INTO analytics.customers\n  (\(change.field)) VALUES ('\(change.after)');"
            case .delete:
                "DELETE FROM analytics.customers\nWHERE customer_id = $1;"
            }
        }
        return (["BEGIN;"] + statements + ["COMMIT;"]).joined(separator: "\n\n")
    }
}
