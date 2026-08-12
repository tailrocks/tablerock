import SwiftUI

struct LabSurfaceContent: View {
    let surface: LabSurface
    var compact = false

    @ViewBuilder
    var body: some View {
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

struct LabConnectionsSurface: View {
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
                Button("New Connection", systemImage: "plus") {}
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
                                LabConnectionCard(connection: connection)
                            }
                            LabNewConnectionCard()
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
                    Text("Edit")
                    Text("Duplicate")
                    Divider()
                    Text("Remove")
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
                Button("Test Connection", systemImage: "wave.3.right") {}
                Button("Save & Connect", systemImage: "arrow.right") {}
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
    var compact = false

    var body: some View {
        VStack(spacing: 0) {
            LabObjectHeader(title: "customers", detail: "analytics.public · PostgreSQL", symbol: "tablecells") {
                Button("Add Row", systemImage: "plus") {}
                Button("Export", systemImage: "square.and.arrow.up") {}
            }
            LabFilterRail()
            LabDataTable(compact: compact)
            LabStatusBar()
        }
        .background(Color(nsColor: .textBackgroundColor))
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
            Text("Sorted by monthly_revenue ↓")
                .font(.caption2)
                .labSecondaryForeground()
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
    var compact = false

    private var columns: ArraySlice<LabColumn> {
        LabFixtures.columns.prefix(compact ? 5 : 8)
    }

    var body: some View {
        ScrollView([.horizontal, .vertical]) {
            LazyVStack(spacing: 0, pinnedViews: [.sectionHeaders]) {
                Section {
                    ForEach(LabFixtures.rows) { row in
                        LabDataRow(row: row, columns: Array(columns))
                    }
                } header: {
                    LabDataHeader(columns: Array(columns))
                }
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Customer rows and columns")
        }
        .defaultScrollAnchor(.topLeading)
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Customer result grid, 12 preview rows")
    }
}

private struct LabDataHeader: View {
    @Environment(\.labAccessibilityPreview) private var preview

    let columns: [LabColumn]

    var body: some View {
        HStack(spacing: 0) {
            Text("#")
                .frame(width: 44, alignment: .trailing)
                .padding(.trailing, 8)
            ForEach(columns) { column in
                VStack(alignment: .leading, spacing: 1) {
                    HStack(spacing: 4) {
                        Text(column.title).fontWeight(.semibold)
                        if column.id == "mrr" { Image(systemName: "arrow.down").font(.caption2) }
                    }
                    Text(column.type)
                        .font(.caption2.weight(preview == .increaseContrast ? .semibold : .medium))
                        .labTertiaryForeground()
                }
                .frame(width: column.width, alignment: .leading)
                .padding(.horizontal, 8)
                .overlay(alignment: .trailing) { Divider() }
            }
        }
        .font(.caption)
        .frame(height: 36)
        .background(Color(nsColor: .controlBackgroundColor))
        .overlay(alignment: .bottom) { Divider() }
    }
}

private struct LabDataRow: View {
    @Environment(\.labAccessibilityPreview) private var preview
    @Environment(\.colorScheme) private var colorScheme

    let row: LabRow
    let columns: [LabColumn]

    var body: some View {
        HStack(spacing: 0) {
            Text(String(row.id))
                .font(.system(size: preview == .increaseContrast ? 13 : 10, weight: preview == .increaseContrast ? .bold : .regular, design: .monospaced))
                .labPrimaryForeground()
                .frame(width: 44, alignment: .trailing)
                .padding(.trailing, 8)
            ForEach(Array(columns.enumerated()), id: \.element.id) { index, column in
                Text(row.values[index])
                    .font(.caption.monospaced())
                    .fontWeight(preview == .increaseContrast ? .medium : .regular)
                    .foregroundStyle(
                        preview == .increaseContrast
                            ? (colorScheme == .dark ? Color.white : Color.black)
                            : (column.id == "active" ? Color.green : Color.primary)
                    )
                    .lineLimit(1)
                    .frame(width: column.width, alignment: column.id == "seats" || column.id == "mrr" ? .trailing : .leading)
                    .padding(.horizontal, 8)
                    .overlay(alignment: .trailing) { Divider().opacity(0.45) }
            }
        }
        .frame(height: 29)
        .background(row.id == 10482 ? Color.accentColor.opacity(0.13) : (row.id.isMultiple(of: 2) ? Color.primary.opacity(0.025) : .clear))
        .overlay(alignment: .bottom) { Divider().opacity(0.45) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Row \(row.id)")
    }
}

struct LabSQLResultsSurface: View {
    var compact = false

    var body: some View {
        VSplitView {
            VStack(spacing: 0) {
                LabObjectHeader(title: "Revenue by region", detail: "Query 2 · Northstar Analytics", symbol: "chevron.left.forwardslash.chevron.right") {
                    Button("Explain", systemImage: "chart.xyaxis.line") {}
                    Button("Run", systemImage: "play.fill") {}
                        .buttonStyle(.borderedProminent)
                        .tint(.green)
                        .keyboardShortcut(.return, modifiers: .command)
                }
                LabCodeEditor()
            }
            .frame(minHeight: compact ? 210 : 250)

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
                    Label("100 rows", systemImage: "tablecells")
                    Text("126 ms")
                    LabBadge(text: "SUCCESS", tint: .green, symbol: "checkmark.circle.fill")
                }
                .font(.caption)
                .padding(.horizontal, 10)
                .frame(height: 38)
                .background(Color(nsColor: .windowBackgroundColor))
                .overlay(alignment: .bottom) { Divider() }
                LabDataTable(compact: compact)
            }
            .frame(minHeight: 240)
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}

private struct LabCodeEditor: View {
    private let lines = LabFixtures.query.split(separator: "\n", omittingEmptySubsequences: false)

    var body: some View {
        ScrollView([.horizontal, .vertical]) {
            HStack(alignment: .top, spacing: 0) {
                VStack(alignment: .trailing, spacing: 0) {
                    ForEach(lines.indices, id: \.self) { index in
                        Text(String(index + 1))
                            .frame(height: 22)
                    }
                }
                .font(.caption.monospacedDigit())
                .foregroundStyle(.tertiary)
                .padding(.horizontal, 10)
                .padding(.vertical, 12)
                .background(Color(nsColor: .controlBackgroundColor).opacity(0.7))

                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(lines.enumerated()), id: \.offset) { _, line in
                        Text(String(line))
                            .font(.system(size: 13, design: .monospaced))
                            .foregroundStyle(sqlColor(for: String(line)))
                            .frame(height: 22, alignment: .leading)
                    }
                }
                .padding(12)
                Spacer(minLength: 200)
            }
        }
        .defaultScrollAnchor(.topLeading)
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityLabel("SQL editor preview")
    }

    private func sqlColor(for line: String) -> Color {
        let trimmed = line.trimmingCharacters(in: .whitespaces).uppercased()
        if ["SELECT", "FROM", "WHERE", "ORDER", "LIMIT"].contains(where: trimmed.hasPrefix) {
            return .purple
        }
        return .primary
    }
}

struct LabChangeReviewSurface: View {
    var compact = false

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Review Changes")
                        .font(.title2.weight(.semibold))
                    Text("Northstar Analytics · analytics.public · 4 staged operations")
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

            HSplitView {
                ScrollView {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text("STAGED OPERATIONS")
                                .font(.caption2.weight(.semibold))
                                .foregroundStyle(.secondary)
                            Spacer()
                            Text("2 updates · 1 insert · 1 delete")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        ForEach(LabFixtures.changes) { change in
                            LabReviewChangeRow(change: change)
                        }
                    }
                    .padding(16)
                }
                .frame(minWidth: 440)

                if !compact {
                    LabSQLPreview()
                        .frame(minWidth: 330, idealWidth: 420)
                }
            }

            Divider()
            HStack {
                Button("Back to Editing") {}
                Button("Discard All", role: .destructive) {}
                Spacer()
                VStack(alignment: .trailing, spacing: 1) {
                    Text("4 changes will run in one transaction")
                        .font(.caption.weight(.medium))
                    Text("Rollback on any failure")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
                Button("Apply Changes", systemImage: "checkmark") {}
                    .buttonStyle(.borderedProminent)
                    .tint(.orange)
            }
            .padding(.horizontal, 16)
            .frame(height: 58)
            .background(Color(nsColor: .windowBackgroundColor))
        }
        .background(Color(nsColor: .controlBackgroundColor))
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
            Text("""
            BEGIN;

            UPDATE analytics.customers
            SET plan = 'Scale', seats = 124
            WHERE customer_id = '…a91f';

            INSERT INTO analytics.customers
              (company_name, region, plan)
            VALUES ('Morrow Studio', 'APAC', 'Team');

            DELETE FROM analytics.customers
            WHERE customer_id = '…4de2';

            COMMIT;
            """)
            .font(.system(size: 12, design: .monospaced))
            .textSelection(.enabled)
            .padding(14)
            Spacer()
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}
