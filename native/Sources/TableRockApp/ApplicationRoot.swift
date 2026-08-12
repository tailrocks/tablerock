import AppKit
import SwiftUI
import TableRockPresentation

@main
struct TableRockApp: App {
  private let application = NativeApplicationModel()

  init() {
    #if TABLEROCK_DEVELOPMENT_SUPPORT
    application.appearanceFixture.applyApplicationAppearance()
    #endif
  }

  var body: some Scene {
    WindowGroup(for: UUID.self) { $windowId in
      #if TABLEROCK_DEVELOPMENT_SUPPORT
      switch application.launchConfiguration.surface {
      case .accessibilityAudit:
        NativeAccessibilityFixtureView()
          .frame(minWidth: 760, minHeight: 520)
      case .profileEditor:
        NativeProfileEditorFixtureView()
      case .performanceGrid, .workbench:
        WorkbenchWindowRoot(
          application: application, windowId: windowId
        )
      }
      #else
        WorkbenchWindowRoot(application: application, windowId: windowId)
      #endif
    } defaultValue: {
      application.dependencies.identifiers.next()
    }
    .restorationBehavior(.automatic)
    .commands {
      WorkbenchCommands()
    }
    Settings {
      NativeSettingsView(
        client: application.client,
        dependencies: application.dependencies
      )
    }
  }
}

private struct WorkbenchWindowRoot: View {
  @Environment(\.openWindow) private var openWindow
  @State private var model: WorkbenchPresentationStore
  private let application: NativeApplicationModel

  init(application: NativeApplicationModel, windowId: UUID) {
    self.application = application
    _model = State(
      initialValue: WorkbenchPresentationStore(
        client: application.client,
        startupError: application.bridgeError,
        windowId: windowId,
        dependencies: application.dependencies,
        dataRootPath: application.dataRootPath
      ))
  }

  var body: some View {
    #if TABLEROCK_DEVELOPMENT_SUPPORT
    if application.launchConfiguration.surface == .performanceGrid {
      PerformanceFixtureView(model: model)
        .frame(minWidth: 760, minHeight: 520)
        .task { await openFixtureWindowIfNeeded() }
    } else {
      ContentView()
        .environment(model)
        .background(NativeWindowConfiguration())
        .modifier(
          NativeAppearanceFixtureModifier(
            fixture: application.appearanceFixture
          )
        )
        .frame(minWidth: 760, minHeight: 520)
        .task { await launchFixturesIfNeeded() }
        .onOpenURL { url in
          Task { await model.receiveExternalURL(url) }
        }
    }
    #else
      productionWorkbench
    #endif
  }

  private var productionWorkbench: some View {
    ContentView()
      .environment(model)
      .background(NativeWindowConfiguration())
      .frame(minWidth: 760, minHeight: 520)
      .onOpenURL { url in
        Task { await model.receiveExternalURL(url) }
      }
  }

  #if TABLEROCK_DEVELOPMENT_SUPPORT
  private func launchFixturesIfNeeded() async {
    await model.receiveExternalUrlFixtureIfNeeded()
    await openFixtureWindowIfNeeded()
  }

  private func openFixtureWindowIfNeeded() async {
    guard application.launchConfiguration.opensSecondWindow,
      application.claimMultiWindowFixtureOpen()
    else { return }
    openWindow(value: application.dependencies.identifiers.next())
    try? await Task.sleep(for: .milliseconds(800))
    runNativeMultiWindowAudit()
  }
  #endif
}

private struct NativeWindowConfiguration: NSViewRepresentable {
  func makeNSView(context: Context) -> NSView { NSView() }

  func updateNSView(_ view: NSView, context: Context) {
    Task { @MainActor in
      guard let window = view.window else { return }
      window.setAccessibilityIdentifier("window.workbench")
      window.tabbingIdentifier = "tablerock-workbench"
      window.tabbingMode = .preferred
      window.tab.title = window.title
    }
  }
}
