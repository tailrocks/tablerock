import SwiftUI

@main
struct TableRockDesignLabApp: App {
    private let launch = LabLaunchConfiguration.parse(CommandLine.arguments)

    var body: some Scene {
        WindowGroup("TableRock Design Lab") {
            LabRootView(launch: launch)
                .frame(minWidth: 980, minHeight: 680)
        }
        .defaultSize(width: 1_440, height: 900)
        .windowStyle(.hiddenTitleBar)
    }
}

struct LabRootView: View {
    @State private var concept: LabConcept
    @State private var surface: LabSurface
    @State private var appearance: LabAppearance
    @State private var accessibility: LabAccessibilityMode

    private let captureMode: Bool

    init(launch: LabLaunchConfiguration) {
        _concept = State(initialValue: launch.concept)
        _surface = State(initialValue: launch.surface)
        _appearance = State(initialValue: launch.appearance)
        _accessibility = State(initialValue: launch.accessibility)
        captureMode = launch.captureMode
    }

    var body: some View {
        VStack(spacing: 0) {
            if !captureMode {
                LabControlBar(
                    concept: $concept,
                    surface: $surface,
                    appearance: $appearance,
                    accessibility: $accessibility
                )
            }

            LabConceptHost(concept: concept, surface: surface)
                .id("\(concept.rawValue)-\(surface.rawValue)")
                .accessibilityIdentifier("design-lab-concept")
        }
        .contrast(accessibility == .increaseContrast ? 1.22 : 1)
        .preferredColorScheme(preferredColorScheme)
        .environment(\.labAccessibilityPreview, accessibility)
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var preferredColorScheme: ColorScheme? {
        switch appearance {
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
    @Binding var concept: LabConcept
    @Binding var surface: LabSurface
    @Binding var appearance: LabAppearance
    @Binding var accessibility: LabAccessibilityMode

    var body: some View {
        HStack(spacing: 12) {
            Label("Design Lab", systemImage: "hammer")
                .font(.headline)

            Divider().frame(height: 22)

            Picker("Concept", selection: $concept) {
                ForEach(LabConcept.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .frame(width: 190)

            Picker("Surface", selection: $surface) {
                ForEach(LabSurface.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .frame(width: 170)

            Spacer()

            Picker("Accessibility", selection: $accessibility) {
                ForEach(LabAccessibilityMode.allCases) { item in
                    Text(item.title).tag(item)
                }
            }
            .frame(width: 180)

            Picker("Appearance", selection: $appearance) {
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
