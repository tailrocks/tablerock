import SwiftUI

private enum LabForegroundLevel {
    case primary
    case secondary
    case tertiary

    var color: Color {
        switch self {
        case .primary: Color(nsColor: .labelColor)
        case .secondary: Color(nsColor: .secondaryLabelColor)
        case .tertiary: Color(nsColor: .tertiaryLabelColor)
        }
    }
}

private struct LabAdaptiveForeground: ViewModifier {
    @Environment(\.labAccessibilityPreview) private var preview
    @Environment(\.colorScheme) private var colorScheme

    let level: LabForegroundLevel

    func body(content: Content) -> some View {
        content.foregroundStyle(
            preview == .increaseContrast
                ? (colorScheme == .dark ? Color.white : Color.black)
                : level.color
        )
    }
}

extension View {
    func labPrimaryForeground() -> some View {
        modifier(LabAdaptiveForeground(level: .primary))
    }

    func labSecondaryForeground() -> some View {
        modifier(LabAdaptiveForeground(level: .secondary))
    }

    func labTertiaryForeground() -> some View {
        modifier(LabAdaptiveForeground(level: .tertiary))
    }
}

struct LabChromeBackground: View {
    @Environment(\.labAccessibilityPreview) private var preview

    var body: some View {
        if preview == .reduceTransparency || preview == .increaseContrast {
            Color(nsColor: .windowBackgroundColor)
        } else {
            Rectangle().fill(.bar)
        }
    }
}

struct LabGlassPanel<Content: View>: View {
    @Environment(\.accessibilityReduceTransparency) private var systemReduceTransparency
    @Environment(\.accessibilityDifferentiateWithoutColor) private var systemDifferentiate
    @Environment(\.colorSchemeContrast) private var systemContrast
    @Environment(\.labAccessibilityPreview) private var preview

    let radius: CGFloat
    let padding: CGFloat
    @ViewBuilder let content: Content

    init(
        radius: CGFloat = 14,
        padding: CGFloat = 10,
        @ViewBuilder content: () -> Content
    ) {
        self.radius = radius
        self.padding = padding
        self.content = content()
    }

    private var reduceTransparency: Bool {
        systemReduceTransparency || preview == .reduceTransparency
    }

    private var highContrast: Bool {
        systemDifferentiate || systemContrast == .increased || preview == .increaseContrast
    }

    var body: some View {
        if reduceTransparency {
            content
                .padding(padding)
                .background(Color(nsColor: .controlBackgroundColor), in: .rect(cornerRadius: radius))
                .overlay {
                    RoundedRectangle(cornerRadius: radius)
                        .stroke(Color(nsColor: highContrast ? .labelColor : .separatorColor), lineWidth: highContrast ? 2 : 1)
                }
                .shadow(color: .black.opacity(0.12), radius: 12, y: 5)
        } else {
            content
                .padding(padding)
                .glassEffect(.regular.interactive(), in: .rect(cornerRadius: radius))
                .overlay {
                    if highContrast {
                        RoundedRectangle(cornerRadius: radius)
                            .stroke(Color(nsColor: .labelColor), lineWidth: 2)
                    }
                }
        }
    }
}

struct LabIconButton: View {
    @Environment(\.accessibilityReduceTransparency) private var systemReduceTransparency
    @Environment(\.accessibilityDifferentiateWithoutColor) private var systemDifferentiate
    @Environment(\.colorSchemeContrast) private var systemContrast
    @Environment(\.labAccessibilityPreview) private var preview

    let title: String
    let symbol: String
    var selected = false
    var prominent = false

    var body: some View {
        Group {
            if reduceTransparency && prominent {
                button.buttonStyle(.borderedProminent)
            } else if reduceTransparency {
                button.buttonStyle(.bordered)
            } else if prominent {
                button.buttonStyle(.glassProminent)
            } else {
                button.buttonStyle(.glass)
            }
        }
        .tint(selected ? .accentColor : nil)
        .help(title)
        .accessibilityLabel(title)
        .overlay {
            if highContrast {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(Color(nsColor: .labelColor), lineWidth: 1.5)
                    .allowsHitTesting(false)
            }
        }
    }

    private var reduceTransparency: Bool {
        systemReduceTransparency || preview == .reduceTransparency
    }

    private var highContrast: Bool {
        systemDifferentiate || systemContrast == .increased || preview == .increaseContrast
    }

    private var button: some View {
        Button(action: {}) {
            Image(systemName: symbol)
                .font(.system(size: 13, weight: .medium))
                .frame(width: 28, height: 26)
                .contentShape(.rect)
        }
    }
}

struct LabBadge: View {
    @Environment(\.labAccessibilityPreview) private var preview
    @Environment(\.colorScheme) private var colorScheme

    let text: String
    var tint: Color = .secondary
    var symbol: String?

    var body: some View {
        HStack(spacing: 4) {
            if let symbol {
                Image(systemName: symbol)
            }
            Text(text)
        }
        .font(.caption2.weight(.semibold))
        .foregroundStyle(foregroundColor)
        .padding(.horizontal, 7)
        .padding(.vertical, 3)
        .background(backgroundColor, in: .capsule)
        .overlay {
            if preview == .increaseContrast {
                Capsule().stroke(foregroundColor, lineWidth: 1.5)
            }
        }
        .accessibilityElement(children: .combine)
    }

    private var foregroundColor: Color {
        preview == .increaseContrast
            ? (colorScheme == .dark ? .white : .black)
            : tint
    }

    private var backgroundColor: Color {
        preview == .increaseContrast
            ? Color(nsColor: .controlBackgroundColor)
            : tint.opacity(0.11)
    }
}

struct LabContextToolbar: View {
    var compact = false

    var body: some View {
        HStack(spacing: 8) {
            if !compact {
                LabIconButton(title: "Toggle navigation", symbol: "sidebar.left")
                Divider().frame(height: 20)
            }

            Menu {
                Button("Northstar Analytics") {}
                Button("Atlas Events") {}
                Divider()
                Button("Manage Connections…") {}
            } label: {
                HStack(spacing: 7) {
                    Image(systemName: "cylinder.split.1x2")
                        .foregroundStyle(.blue)
                    VStack(alignment: .leading, spacing: 0) {
                        Text("Northstar Analytics")
                            .font(.caption.weight(.semibold))
                        Text("analytics · public")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    if !compact {
                        LabBadge(text: "PRODUCTION", tint: .orange, symbol: "exclamationmark.triangle.fill")
                    }
                }
                .padding(.horizontal, 8)
                .frame(height: 32)
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.visible)
            .accessibilityLabel("Database context, Northstar Analytics, production")

            Spacer(minLength: 8)

            LabIconButton(title: "Refresh", symbol: "arrow.clockwise")
            LabIconButton(title: "Search", symbol: "magnifyingglass")
            LabIconButton(title: "New query", symbol: "plus.rectangle.on.rectangle")

            if !compact {
                Divider().frame(height: 20)
                LabBadge(text: "SAFE MODE", tint: .green, symbol: "lock.shield.fill")
            }
        }
        .padding(.horizontal, 10)
        .frame(height: 48)
    }
}

struct LabTabStrip: View {
    var sqlSelected = false

    var body: some View {
        HStack(spacing: 1) {
            LabDocumentTab(title: "customers", symbol: "tablecells", selected: !sqlSelected, dirty: true)
            LabDocumentTab(title: "Revenue by region", symbol: "chevron.left.forwardslash.chevron.right", selected: sqlSelected)
            LabDocumentTab(title: "orders", symbol: "tablecells")
            Button(action: {}) {
                Image(systemName: "plus")
                    .frame(width: 28, height: 28)
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
            .help("New tab")
            Spacer()
        }
        .padding(.horizontal, 8)
        .frame(height: 34)
        .background(Color(nsColor: .windowBackgroundColor))
        .overlay(alignment: .bottom) { Divider() }
    }
}

private struct LabDocumentTab: View {
    @Environment(\.labAccessibilityPreview) private var preview

    let title: String
    let symbol: String
    var selected = false
    var dirty = false

    var body: some View {
        Button(action: {}) {
            HStack(spacing: 6) {
                Image(systemName: symbol)
                    .font(.caption)
                    .foregroundStyle(selected ? Color.accentColor : .secondary)
                Text(title)
                    .font(.caption)
                    .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                    .labPrimaryForeground()
                if dirty {
                    Circle()
                        .fill(.orange)
                        .frame(width: 6, height: 6)
                        .accessibilityHidden(true)
                }
                Image(systemName: "xmark")
                    .font(.system(size: 8, weight: .bold))
                    .foregroundStyle(.tertiary)
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(title)
        .accessibilityValue(dirty ? "Pending changes" : "No pending changes")
        .padding(.horizontal, 10)
        .frame(height: 28)
        .background(selected ? Color(nsColor: .controlBackgroundColor) : .clear, in: .rect(cornerRadius: 7))
        .overlay {
            if selected {
                RoundedRectangle(cornerRadius: 7)
                    .stroke(Color(nsColor: .separatorColor), lineWidth: 0.5)
            }
        }
    }
}

struct LabCatalogSidebar: View {
    var compact = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                Text("Search objects")
                Spacer()
                Text("⌘K")
                    .font(.caption2)
            }
            .font(.caption)
            .padding(.horizontal, 10)
            .frame(height: 30)
            .background(Color(nsColor: .controlBackgroundColor).opacity(0.75), in: .rect(cornerRadius: 7))
            .padding(10)

            HStack {
                Text("analytics")
                    .font(.caption.weight(.semibold))
                Spacer()
                Text("public")
                    .font(.caption2)
                    .labSecondaryForeground()
            }
            .padding(.horizontal, 12)
            .padding(.bottom, 6)

            ScrollView {
                VStack(alignment: .leading, spacing: 2) {
                    LabTreeRow(title: "Tables", symbol: "folder", depth: 0, expanded: true)
                    ForEach(LabFixtures.catalog.prefix(compact ? 4 : 6)) { item in
                        LabTreeRow(
                            title: item.name,
                            detail: compact ? nil : item.detail,
                            symbol: item.symbol,
                            depth: 1,
                            selected: item.id == "customers"
                        )
                    }
                    LabTreeRow(title: "Functions", detail: "18", symbol: "function", depth: 0)
                    LabTreeRow(title: "Types", detail: "7", symbol: "curlybraces", depth: 0)
                }
                .padding(.horizontal, 6)
            }

            Divider()
            HStack {
                Label {
                    Text("Connected").labPrimaryForeground()
                } icon: {
                    Image(systemName: "circle.fill").foregroundStyle(.green)
                }
                Spacer()
                Text("18 ms")
                    .labSecondaryForeground()
            }
            .font(.caption2)
            .padding(.horizontal, 12)
            .frame(height: 32)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Database catalog")
    }
}

struct LabTreeRow: View {
    @Environment(\.labAccessibilityPreview) private var preview

    let title: String
    var detail: String?
    let symbol: String
    var depth = 0
    var expanded = false
    var selected = false

    var body: some View {
        HStack(spacing: 6) {
            if depth == 0 {
                Image(systemName: expanded ? "chevron.down" : "chevron.right")
                    .font(.system(size: 8, weight: .bold))
                    .frame(width: 10)
                    .labSecondaryForeground()
            } else {
                Spacer().frame(width: 10)
            }
            Image(systemName: symbol)
                .font(.caption)
                .foregroundStyle(selected ? Color.accentColor : .secondary)
                .frame(width: 14)
            Text(title)
                .font(.caption)
                .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                .labPrimaryForeground()
                .lineLimit(1)
            Spacer(minLength: 4)
            if let detail {
                Text(detail)
                    .font(
                        .system(
                            size: preview == .increaseContrast ? 13 : 10,
                            weight: preview == .increaseContrast ? .bold : .regular
                        )
                    )
                    .labTertiaryForeground()
                    .lineLimit(1)
            }
        }
        .padding(.leading, CGFloat(depth) * 8)
        .padding(.horizontal, 7)
        .frame(height: 26)
        .background(selected ? Color.accentColor.opacity(0.14) : .clear, in: .rect(cornerRadius: 6))
        .contentShape(.rect)
    }
}

struct LabModeRail: View {
    let surface: LabSurface

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "square.grid.2x2.fill")
                .font(.title3)
                .foregroundStyle(.blue)
                .padding(.bottom, 8)

            ForEach(LabSurface.allCases) { item in
                Button(action: {}) {
                    Image(systemName: item.symbol)
                        .font(.system(size: 14, weight: .medium))
                        .frame(width: 30, height: 30)
                        .background(item == surface ? Color.accentColor.opacity(0.16) : .clear, in: .rect(cornerRadius: 8))
                }
                .buttonStyle(.plain)
                .foregroundStyle(item == surface ? Color.accentColor : .secondary)
                .help(item.title)
                .accessibilityLabel(item.title)
            }
            Spacer()
            LabIconButton(title: "Settings", symbol: "gearshape")
        }
        .padding(.vertical, 14)
        .frame(width: 54)
    }
}

struct LabSourceColumn: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("SOURCES")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)
                .padding(.top, 12)

            ForEach(LabFixtures.connections) { connection in
                HStack(spacing: 8) {
                    Image(systemName: connection.symbol)
                        .foregroundStyle(connection.id == "northstar" ? Color.blue : .secondary)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(connection.name)
                            .font(.caption.weight(.medium))
                        Text(connection.environment)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                }
                .padding(.horizontal, 9)
                .frame(height: 38)
                .background(connection.id == "northstar" ? Color.accentColor.opacity(0.13) : .clear, in: .rect(cornerRadius: 7))
                .padding(.horizontal, 5)
            }

            Spacer()
            Button("New Connection", systemImage: "plus") {}
                .buttonStyle(.plain)
                .font(.caption)
                .padding(12)
        }
    }
}

struct LabObjectColumn: View {
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("OBJECTS")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Image(systemName: "line.3.horizontal.decrease")
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, 12)
            .frame(height: 38)

            Divider()
            ScrollView {
                VStack(spacing: 2) {
                    ForEach(LabFixtures.catalog) { item in
                        HStack(spacing: 8) {
                            Image(systemName: item.symbol)
                                .foregroundStyle(item.id == "customers" ? Color.accentColor : .secondary)
                            VStack(alignment: .leading, spacing: 1) {
                                Text(item.name).font(.caption)
                                Text(item.detail).font(.caption2).foregroundStyle(.secondary)
                            }
                            Spacer()
                        }
                        .padding(.horizontal, 9)
                        .frame(height: 38)
                        .background(item.id == "customers" ? Color.accentColor.opacity(0.13) : .clear, in: .rect(cornerRadius: 7))
                    }
                }
                .padding(6)
            }
        }
    }
}

struct LabConnectionGroupsColumn: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("GROUPS")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)
                .padding(.top, 12)

            LabGroupRow(title: "All Connections", count: 3, symbol: "tray.full", selected: true)
            LabGroupRow(title: "Favorites", count: 3, symbol: "star")
            LabGroupRow(title: "Production", count: 1, symbol: "exclamationmark.triangle")
            LabGroupRow(title: "Staging", count: 1, symbol: "shippingbox")
            LabGroupRow(title: "Local", count: 1, symbol: "desktopcomputer")
            Spacer()
            Button("New Group", systemImage: "folder.badge.plus") {}
                .buttonStyle(.plain)
                .font(.caption)
                .padding(12)
        }
    }
}

private struct LabGroupRow: View {
    let title: String
    let count: Int
    let symbol: String
    var selected = false

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: symbol)
                .foregroundStyle(selected ? Color.accentColor : .secondary)
                .frame(width: 16)
            Text(title)
                .font(.caption)
            Spacer()
            Text(String(count))
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 9)
        .frame(height: 30)
        .background(selected ? Color.accentColor.opacity(0.13) : .clear, in: .rect(cornerRadius: 7))
        .padding(.horizontal, 5)
    }
}

struct LabConnectionProfilesColumn: View {
    let setupSelected: Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("CONNECTIONS")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Image(systemName: "line.3.horizontal.decrease")
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, 12)
            .frame(height: 38)
            Divider()

            VStack(spacing: 2) {
                HStack(spacing: 8) {
                    Image(systemName: "plus.circle")
                        .foregroundStyle(setupSelected ? Color.accentColor : .secondary)
                    VStack(alignment: .leading, spacing: 1) {
                        Text("New Connection").font(.caption.weight(.medium))
                        Text("PostgreSQL").font(.caption2).foregroundStyle(.secondary)
                    }
                    Spacer()
                }
                .padding(.horizontal, 9)
                .frame(height: 42)
                .background(setupSelected ? Color.accentColor.opacity(0.13) : .clear, in: .rect(cornerRadius: 7))

                ForEach(LabFixtures.connections) { connection in
                    HStack(spacing: 8) {
                        Image(systemName: connection.symbol)
                            .foregroundStyle(!setupSelected && connection.id == "northstar" ? Color.accentColor : .secondary)
                        VStack(alignment: .leading, spacing: 1) {
                            Text(connection.name).font(.caption)
                            Text(connection.environment).font(.caption2).foregroundStyle(.secondary)
                        }
                        Spacer()
                    }
                    .padding(.horizontal, 9)
                    .frame(height: 42)
                    .background(
                        !setupSelected && connection.id == "northstar"
                            ? Color.accentColor.opacity(0.13)
                            : .clear,
                        in: .rect(cornerRadius: 7)
                    )
                }
            }
            .padding(6)
            Spacer()
        }
    }
}

struct LabStatusBar: View {
    @Environment(\.labAccessibilityPreview) private var preview

    var reviewEmphasis = false

    var body: some View {
        HStack(spacing: 12) {
            Picker("View", selection: .constant("Data")) {
                Text("Data").tag("Data")
                Text("Structure").tag("Structure")
            }
            .labelsHidden()
            .pickerStyle(.segmented)
            .frame(width: 136)

            Divider().frame(height: 16)
            Text("12 of 48,224 rows")
                .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                .labPrimaryForeground()
            Text("41 ms")
                .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                .labSecondaryForeground()
            Spacer()
            Label("2 filters", systemImage: "line.3.horizontal.decrease")
                .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                .labPrimaryForeground()
            Label("8 columns", systemImage: "rectangle.split.3x1")
                .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                .labPrimaryForeground()
            HStack(spacing: 2) {
                Button(action: {}) { Image(systemName: "chevron.left") }
                Text("1 / 483")
                    .monospacedDigit()
                    .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                    .labPrimaryForeground()
                Button(action: {}) { Image(systemName: "chevron.right") }
            }
            .buttonStyle(.plain)

            if reviewEmphasis {
                Button("Review 4 Changes", systemImage: "checklist") {}
                    .buttonStyle(.borderedProminent)
                    .tint(.orange)
            } else {
                LabBadge(text: "4 CHANGES", tint: .orange, symbol: "circle.fill")
            }
        }
        .font(.caption)
        .padding(.horizontal, 12)
        .frame(height: 38)
        .background(Color(nsColor: .windowBackgroundColor))
        .overlay(alignment: .top) { Divider() }
    }
}

struct LabValueInspector: View {
    @Environment(\.labAccessibilityPreview) private var preview

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("INSPECTOR")
                    .font(.system(size: preview == .increaseContrast ? 13 : 10, weight: .bold))
                    .labSecondaryForeground()
                Spacer()
                Button(action: {}) { Image(systemName: "xmark") }
                    .buttonStyle(.plain)
            }
            .padding(.horizontal, 12)
            .frame(height: 40)
            Divider()

            VStack(alignment: .leading, spacing: 14) {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("company_name").font(.caption.weight(.semibold))
                        Text("TEXT · NOT NULL")
                            .font(.caption)
                            .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                            .labSecondaryForeground()
                    }
                    Spacer()
                    Button(action: {}) { Image(systemName: "doc.on.doc") }
                        .buttonStyle(.plain)
                        .help("Copy value")
                }
                Text("Aster Works")
                    .font(.body.monospaced())
                    .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                    .labPrimaryForeground()
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .overlay {
                        RoundedRectangle(cornerRadius: 7)
                            .stroke(Color(nsColor: .separatorColor))
                    }

                Text("ROW DETAILS")
                    .font(.system(size: preview == .increaseContrast ? 13 : 10, weight: .bold))
                    .labSecondaryForeground()
                LabInspectorField(name: "customer_id", value: "…a91f")
                LabInspectorField(name: "region", value: "APAC")
                LabInspectorField(name: "plan", value: "Scale")
                LabInspectorField(name: "active", value: "true")
                Spacer()
            }
            .padding(12)
        }
        .background(Color(nsColor: .controlBackgroundColor))
        .labPrimaryForeground()
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Selected value inspector")
    }
}

private struct LabInspectorField: View {
    @Environment(\.labAccessibilityPreview) private var preview

    let name: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(name)
                .font(.caption)
                .fontWeight(preview == .increaseContrast ? .semibold : .regular)
                .labSecondaryForeground()
            Text(value).font(.caption.monospaced())
        }
    }
}

struct LabChangeLedger: View {
    var compact = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Label("Pending Changes", systemImage: "checklist")
                    .font(.headline)
                Spacer()
                Text("4")
                    .font(.caption.weight(.bold))
                    .padding(.horizontal, 7)
                    .padding(.vertical, 2)
                    .background(.orange.opacity(0.16), in: .capsule)
            }
            .padding(.horizontal, 12)
            .frame(height: 46)
            Divider()

            ScrollView {
                VStack(spacing: 8) {
                    ForEach(LabFixtures.changes) { change in
                        VStack(alignment: .leading, spacing: 5) {
                            HStack {
                                LabBadge(text: change.kind.rawValue, tint: change.kind == .delete ? .red : .orange)
                                Text(change.object)
                                    .font(.caption.weight(.medium))
                                    .lineLimit(1)
                            }
                            Text(change.field)
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                            if !compact {
                                HStack(spacing: 5) {
                                    Text(change.before)
                                        .strikethrough(change.kind != .insert)
                                        .foregroundStyle(.secondary)
                                    Image(systemName: "arrow.right")
                                        .font(.caption2)
                                    Text(change.after)
                                }
                                .font(.caption.monospaced())
                            }
                        }
                        .padding(9)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color(nsColor: .controlBackgroundColor), in: .rect(cornerRadius: 8))
                    }
                }
                .padding(10)
            }

            Divider()
            VStack(spacing: 8) {
                HStack {
                    Label("PRODUCTION", systemImage: "exclamationmark.triangle.fill")
                        .foregroundStyle(.orange)
                    Spacer()
                    Text("Safe mode on")
                        .foregroundStyle(.secondary)
                }
                .font(.caption.weight(.semibold))
                HStack {
                    Button("Discard") {}
                    Spacer()
                    Button("Review & Apply") {}
                        .buttonStyle(.borderedProminent)
                        .tint(.orange)
                }
            }
            .padding(12)
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
