import AppKit
import SwiftUI

@main
struct TableRockDesignLabApp: App {
    @NSApplicationDelegateAdaptor(LabApplicationDelegate.self) private var appDelegate
    private let launch: LabLaunchConfiguration
    private let session: LabSession

    init() {
        let launch = LabLaunchConfiguration.parse(CommandLine.arguments)
        self.launch = launch
        session = LabSession(launch: launch)
    }

    var body: some Scene {
        WindowGroup("TableRock Design Lab") {
            LabRootView(session: session)
                .frame(minWidth: 980, minHeight: 680)
                .environmentObject(session)
        }
        .defaultSize(
            width: launch.windowSize.dimensions.width,
            height: launch.windowSize.dimensions.height
        )
        .windowStyle(.hiddenTitleBar)
        .windowToolbarStyle(.unified)
        .commands { LabCommands(session: session) }
    }
}

@MainActor
final class LabApplicationDelegate: NSObject, NSApplicationDelegate {
    private let launch = LabLaunchConfiguration.parse(CommandLine.arguments)

    func applicationDidFinishLaunching(_ notification: Notification) {
        Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(150))
            openInitialWindowIfNeeded()
            try? await Task.sleep(for: .milliseconds(150))
            sizeInitialWindow()
        }
    }

    private func openInitialWindowIfNeeded() {
        guard NSApplication.shared.windows.allSatisfy({ !$0.isVisible }),
              let fileMenu = NSApplication.shared.mainMenu?.item(withTitle: "File")?.submenu,
              let item = fileMenu.items.first(where: {
                  $0.title == "New TableRock Design Lab Window"
              }),
              let action = item.action
        else { return }
        NSApplication.shared.sendAction(action, to: item.target, from: item)
    }

    private func sizeInitialWindow() {
        guard let window = NSApplication.shared.windows.first(where: { $0.isVisible }) else {
            return
        }
        window.setContentSize(launch.windowSize.dimensions)
        window.center()
    }
}

struct LabRootView: View {
    @ObservedObject var session: LabSession

    var body: some View {
        VStack(spacing: 0) {
            if !session.captureMode {
                LabControlBar(session: session)
            }

            LabConceptHost(concept: session.concept, surface: session.surface)
                .id("\(session.concept.rawValue)-\(session.surface.rawValue)")
        }
        .preferredColorScheme(preferredColorScheme)
        .environment(\.labAccessibilityPreview, session.accessibility)
        .background(Color(nsColor: .windowBackgroundColor))
        .toolbarRole(.editor)
        .toolbar { LabAppToolbar(session: session) }
        .inspector(isPresented: $session.inspectorPresented) {
            LabValueInspector()
                .frame(minWidth: 240, idealWidth: 280, maxWidth: 340)
                .environmentObject(session)
        }
        .sheet(isPresented: $session.connectionSheetPresented) {
            LabConnectionSetupSheet()
                .environmentObject(session)
        }
        .sheet(isPresented: $session.reviewSheetPresented) {
            LabDestructiveReviewSheet()
                .environmentObject(session)
        }
        .task {
            guard session.inactiveCapture else { return }
            try? await Task.sleep(for: .milliseconds(450))
            NSApplication.shared.deactivate()
        }
    }

    private var preferredColorScheme: ColorScheme? {
        switch session.appearance {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }
}

private struct LabAccessibilityPreviewKey: EnvironmentKey {
    static let defaultValue = LabAccessibilityMode.system
}

extension EnvironmentValues {
    var labAccessibilityPreview: LabAccessibilityMode {
        get { self[LabAccessibilityPreviewKey.self] }
        set { self[LabAccessibilityPreviewKey.self] = newValue }
    }
}

private struct LabControlBar: View {
    @ObservedObject var session: LabSession

    var body: some View {
        HStack(spacing: 12) {
            Label("Design Lab", systemImage: "hammer")
                .font(.headline)

            Divider().frame(height: 22)

            Picker("Concept", selection: $session.concept) {
                ForEach(LabConcept.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .frame(width: 190)

            Picker("Surface", selection: $session.surface) {
                ForEach(LabSurface.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .frame(width: 170)

            Picker("Fixture", selection: $session.fixture) {
                ForEach(LabFixtureScenario.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .frame(width: 160)

            Spacer(minLength: 8)

            Picker("Accessibility", selection: $session.accessibility) {
                ForEach(LabAccessibilityMode.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .frame(width: 180)

            Picker("Appearance", selection: $session.appearance) {
                ForEach(LabAppearance.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .pickerStyle(.segmented)
            .frame(width: 164)
        }
        .padding(.horizontal, 16)
        .frame(height: 50)
        .background(.bar)
        .overlay(alignment: .bottom) { Divider() }
    }
}

private struct LabAppToolbar: ToolbarContent {
    @ObservedObject var session: LabSession

    var body: some ToolbarContent {
        ToolbarItem(placement: .navigation) {
            Menu {
                ForEach(LabEngine.allCases) { engine in
                    Button {
                        session.engine = engine
                    } label: {
                        Label(engine.title, systemImage: engine.symbol)
                    }
                }
            } label: {
                Label(session.engine.connectionName, systemImage: session.engine.symbol)
            }
            .accessibilityIdentifier("design-lab-engine-menu")
            .help("Database engine and connection")
        }

        ToolbarItemGroup(placement: .primaryAction) {
            Button {
                session.connectionSheetPresented = true
            } label: {
                Label("New Connection", systemImage: "externaldrive.badge.plus")
            }
            .accessibilityIdentifier("design-lab-new-connection")

            Button {
                session.show(.sqlResults)
            } label: {
                Label("New Query", systemImage: "plus.rectangle.on.rectangle")
            }
            .accessibilityIdentifier("design-lab-new-query")

            Button {
                session.inspectorPresented.toggle()
            } label: {
                Label("Toggle Inspector", systemImage: "sidebar.right")
            }
            .accessibilityIdentifier("design-lab-toggle-inspector")

            Button {
                session.reviewSheetPresented = true
            } label: {
                Label("Review Changes", systemImage: "checklist")
            }
            .accessibilityIdentifier("design-lab-review-changes")
        }
    }
}
