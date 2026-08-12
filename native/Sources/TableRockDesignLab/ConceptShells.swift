import SwiftUI

struct LabConceptHost: View {
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    @Environment(\.labAccessibilityPreview) private var preview

    let concept: LabConcept
    let surface: LabSurface

    @ViewBuilder
    var body: some View {
        Group {
            switch concept {
            case .nativeWorkbench:
                LabNativeWorkbench(surface: surface)
            case .queryStudio:
                LabQueryStudio(surface: surface)
            case .columnObservatory:
                LabColumnObservatory(surface: surface)
            case .gridCanvas:
                LabGridCanvas(surface: surface)
            case .changeDesk:
                LabChangeDesk(surface: surface)
            }
        }
        .animation(reduceMotion ? nil : .snappy(duration: 0.28), value: concept)
        .animation(reduceMotion ? nil : .snappy(duration: 0.22), value: surface)
    }

    private var reduceMotion: Bool {
        systemReduceMotion || preview == .reduceMotion
    }
}

// Lead concept: faithful clean-room adaptation of the operator-preferred
// native workbench composition visible in public TablePro materials.
private struct LabNativeWorkbench: View {
    let surface: LabSurface

    private var showsCatalog: Bool {
        surface == .dataGrid || surface == .sqlResults || surface == .changeReview
    }

    var body: some View {
        HSplitView {
            if showsCatalog {
                LabCatalogSidebar()
                    .frame(minWidth: 210, idealWidth: 232, maxWidth: 280)
                    .background(.bar)
            } else {
                LabConnectionNavigator(surface: surface)
                    .frame(minWidth: 200, idealWidth: 220, maxWidth: 250)
                    .background(.bar)
            }

            VStack(spacing: 0) {
                LabContextToolbar()
                    .background(.bar)
                    .overlay(alignment: .bottom) { Divider() }

                if showsCatalog {
                    LabTabStrip(sqlSelected: surface == .sqlResults)
                }

                HSplitView {
                    LabSurfaceContent(surface: surface)
                        .frame(minWidth: 590)

                    if surface == .dataGrid {
                        LabValueInspector()
                            .frame(minWidth: 220, idealWidth: 248, maxWidth: 300)
                    }
                }
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

private struct LabConnectionNavigator: View {
    let surface: LabSurface

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("TableRock")
                    .font(.headline)
                Spacer()
                Button(action: {}) { Image(systemName: "plus") }
                    .buttonStyle(.plain)
                    .help("New connection")
            }
            .padding(12)

            HStack(spacing: 7) {
                Image(systemName: "magnifyingglass")
                Text("Search")
                Spacer()
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 9)
            .frame(height: 29)
            .background(Color(nsColor: .controlBackgroundColor).opacity(0.7), in: .rect(cornerRadius: 7))
            .padding(.horizontal, 8)

            Text("WORKSPACE")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)
                .padding(.top, 10)

            ForEach([LabSurface.connections, .setup]) { item in
                Label(item.title, systemImage: item.symbol)
                    .font(.caption)
                    .padding(.horizontal, 9)
                    .frame(maxWidth: .infinity, minHeight: 30, alignment: .leading)
                    .background(item == surface ? Color.accentColor.opacity(0.14) : .clear, in: .rect(cornerRadius: 7))
                    .padding(.horizontal, 6)
            }

            Text("RECENT")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)
                .padding(.top, 14)
            ForEach(LabFixtures.connections) { connection in
                HStack(spacing: 7) {
                    Image(systemName: connection.symbol).foregroundStyle(.secondary)
                    Text(connection.name).lineLimit(1)
                }
                .font(.caption)
                .padding(.horizontal, 12)
                .frame(height: 29)
            }
            Spacer()
            Divider()
            Label("Settings", systemImage: "gearshape")
                .font(.caption)
                .padding(12)
        }
    }
}

private struct LabQueryStudio: View {
    let surface: LabSurface

    var body: some View {
        HStack(spacing: 0) {
            ZStack {
                Color(nsColor: .windowBackgroundColor)
                LabModeRail(surface: surface)
            }
            .frame(width: 56)
            .overlay(alignment: .trailing) { Divider() }

            VStack(spacing: 0) {
                HStack(spacing: 10) {
                    VStack(alignment: .leading, spacing: 1) {
                        Text(surface == .sqlResults ? "Revenue by region" : surface.title)
                            .font(.headline)
                        Text("Northstar Analytics / analytics / public")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    if surface != .sqlResults {
                        Button("Open Catalog", systemImage: "books.vertical") {}
                    }
                    LabBadge(text: "PRODUCTION", tint: .orange, symbol: "exclamationmark.triangle.fill")
                    LabIconButton(title: "Command palette", symbol: "command")
                }
                .padding(.horizontal, 14)
                .frame(height: 52)
                .background(.bar)
                .overlay(alignment: .bottom) { Divider() }

                if surface == .sqlResults {
                    LabTabStrip(sqlSelected: true)
                }

                LabSurfaceContent(surface: surface, compact: false)
            }

            VStack(spacing: 0) {
                HStack {
                    Text("CATALOG")
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button(action: {}) { Image(systemName: "xmark") }
                        .buttonStyle(.plain)
                }
                .padding(.horizontal, 10)
                .frame(height: 38)
                Divider()
                LabCatalogSidebar(compact: true)
            }
            .frame(width: surface == .sqlResults ? 210 : 0)
            .clipped()
            .background(Color(nsColor: .windowBackgroundColor))
            .overlay(alignment: .leading) { if surface == .sqlResults { Divider() } }
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}

private struct LabColumnObservatory: View {
    let surface: LabSurface

    var body: some View {
        HSplitView {
            if surface == .connections || surface == .setup {
                LabConnectionGroupsColumn()
                    .frame(minWidth: 170, idealWidth: 190, maxWidth: 220)
                    .background(.bar)

                LabConnectionProfilesColumn(setupSelected: surface == .setup)
                    .frame(minWidth: 190, idealWidth: 220, maxWidth: 260)
                    .background(Color(nsColor: .windowBackgroundColor))
            } else {
                LabSourceColumn()
                    .frame(minWidth: 170, idealWidth: 190, maxWidth: 220)
                    .background(.bar)

                LabObjectColumn()
                    .frame(minWidth: 190, idealWidth: 220, maxWidth: 260)
                    .background(Color(nsColor: .windowBackgroundColor))
            }

            VStack(spacing: 0) {
                HStack(spacing: 8) {
                    Image(systemName: surface.symbol)
                        .foregroundStyle(.blue)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(surface == .dataGrid ? "customers" : surface.title)
                            .font(.headline)
                        Text(contextDetail)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    LabBadge(text: "SAFE MODE", tint: .green, symbol: "lock.shield.fill")
                    LabIconButton(title: "Search", symbol: "magnifyingglass")
                    LabIconButton(title: "More actions", symbol: "ellipsis")
                }
                .padding(.horizontal, 12)
                .frame(height: 50)
                .background(.bar)
                .overlay(alignment: .bottom) { Divider() }

                LabSurfaceContent(surface: surface, compact: true)
            }
            .frame(minWidth: 560)
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var contextDetail: String {
        switch surface {
        case .connections: "All Connections · 3 profiles"
        case .setup: "New profile · PostgreSQL"
        default: "Northstar Analytics · analytics.public"
        }
    }
}

private struct LabGridCanvas: View {
    let surface: LabSurface

    var body: some View {
        ZStack {
            LabSurfaceContent(surface: surface)
                .padding(.top, 84)

            VStack {
                HStack(alignment: .top) {
                    GlassEffectContainer(spacing: 12) {
                        HStack(spacing: 8) {
                            LabIconButton(title: "Navigation", symbol: "sidebar.left")
                            Divider().frame(height: 20)
                            Image(systemName: "cylinder.split.1x2")
                                .foregroundStyle(.blue)
                            VStack(alignment: .leading, spacing: 0) {
                                Text("Northstar Analytics").font(.caption.weight(.semibold))
                                Text("analytics.public").font(.caption2).foregroundStyle(.secondary)
                            }
                            Image(systemName: "chevron.down").font(.caption2)
                        }
                    }
                    .frame(maxWidth: 330)
                    .modifier(LabFloatingGlassModifier())

                    Spacer()

                    GlassEffectContainer(spacing: 8) {
                        HStack(spacing: 7) {
                            LabIconButton(title: "Search", symbol: "magnifyingglass")
                            LabIconButton(title: "Refresh", symbol: "arrow.clockwise")
                            LabIconButton(title: "New query", symbol: "plus.rectangle.on.rectangle", prominent: true)
                        }
                    }
                    .modifier(LabFloatingGlassModifier())
                }
                .padding(14)

                Spacer()

                HStack {
                    Spacer()
                    LabGlassPanel(radius: 12, padding: 8) {
                        HStack(spacing: 10) {
                            LabBadge(text: "PRODUCTION", tint: .orange, symbol: "exclamationmark.triangle.fill")
                            Divider().frame(height: 18)
                            Label("48,224 rows", systemImage: "tablecells")
                            Label("4 changes", systemImage: "checklist")
                                .foregroundStyle(.orange)
                            Button("Review") {}
                                .buttonStyle(.borderedProminent)
                                .tint(.orange)
                        }
                        .font(.caption)
                    }
                    Spacer()
                }
                .padding(.bottom, 14)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}

private struct LabFloatingGlassModifier: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) private var systemReduceTransparency
    @Environment(\.labAccessibilityPreview) private var preview

    func body(content: Content) -> some View {
        if systemReduceTransparency || preview == .reduceTransparency {
            content
                .padding(8)
                .background(Color(nsColor: .controlBackgroundColor), in: .rect(cornerRadius: 12))
                .overlay { RoundedRectangle(cornerRadius: 12).stroke(Color(nsColor: .separatorColor)) }
                .shadow(color: .black.opacity(0.15), radius: 10, y: 4)
        } else {
            content
                .padding(8)
                .glassEffect(.regular.interactive(), in: .rect(cornerRadius: 12))
        }
    }
}

private struct LabChangeDesk: View {
    let surface: LabSurface

    var body: some View {
        HSplitView {
            VStack(spacing: 0) {
                HStack {
                    Image(systemName: "square.grid.2x2.fill")
                        .foregroundStyle(.blue)
                    Text("TableRock").font(.headline)
                    Spacer()
                }
                .padding(.horizontal, 12)
                .frame(height: 48)
                LabConnectionNavigator(surface: surface)
            }
            .frame(minWidth: 170, idealWidth: 190, maxWidth: 220)
            .background(.bar)

            VStack(spacing: 0) {
                LabContextToolbar(compact: true)
                    .background(.bar)
                    .overlay(alignment: .bottom) { Divider() }
                if surface == .dataGrid || surface == .sqlResults || surface == .changeReview {
                    LabTabStrip(sqlSelected: surface == .sqlResults)
                }
                LabSurfaceContent(surface: surface, compact: true)
            }
            .frame(minWidth: 540)

            LabChangeLedger(compact: surface != .changeReview)
                .frame(minWidth: 230, idealWidth: 270, maxWidth: 310)
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
