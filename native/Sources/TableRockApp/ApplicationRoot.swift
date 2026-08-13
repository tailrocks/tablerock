import AppKit
import SwiftUI
import TableRockPresentation

private enum NativeWindowMetrics {
  static let minimumWidth: CGFloat = 760
  static let minimumHeight: CGFloat = 520
  static let defaultWidth: CGFloat = 1_440
  static let defaultHeight: CGFloat = 900
}

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
      Group {
        #if TABLEROCK_DEVELOPMENT_SUPPORT
        switch application.launchConfiguration.surface {
        case .accessibilityAudit:
          NativeAccessibilityFixtureView()
            .frame(
              minWidth: NativeWindowMetrics.minimumWidth,
              minHeight: NativeWindowMetrics.minimumHeight
            )
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
      }
      .background(NativeWindowConfiguration())
    } defaultValue: {
      application.dependencies.identifiers.next()
    }
    .defaultSize(
      width: NativeWindowMetrics.defaultWidth,
      height: NativeWindowMetrics.defaultHeight
    )
    .windowStyle(.hiddenTitleBar)
    .windowToolbarStyle(.unified)
    .restorationBehavior(
      application.disablesWindowRestoration ? .disabled : .automatic
    )
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
        .frame(
          minWidth: NativeWindowMetrics.minimumWidth,
          minHeight: NativeWindowMetrics.minimumHeight
        )
        .task { await openFixtureWindowIfNeeded() }
    } else {
      ContentView()
        .environment(model)
        .modifier(
          NativeAppearanceFixtureModifier(
            fixture: application.appearanceFixture
          )
        )
        .frame(
          minWidth: NativeWindowMetrics.minimumWidth,
          minHeight: NativeWindowMetrics.minimumHeight
        )
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
      .frame(
        minWidth: NativeWindowMetrics.minimumWidth,
        minHeight: NativeWindowMetrics.minimumHeight
      )
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
  func makeNSView(context: Context) -> NativeWindowAttachmentView {
    NativeWindowAttachmentView()
  }

  func updateNSView(_ view: NativeWindowAttachmentView, context: Context) {
    view.configureWindow()
  }
}

private final class NativeWindowAttachmentView: NSView {
  override func viewDidMoveToWindow() {
    super.viewDidMoveToWindow()
    configureWindow()
  }

  func configureWindow() {
    guard let window else { return }
    window.setAccessibilityIdentifier("window.workbench")
    window.tabbingIdentifier = "tablerock-workbench"
    window.tabbingMode = .preferred
    window.tab.title = window.title
    Task { @MainActor [weak self] in
      await Task.yield()
      self?.configureAccessibilityHierarchy()
    }
  }

  private func configureAccessibilityHierarchy() {
    var current = superview
    var root: NSView?
    while let view = current {
      if view.isAccessibilityElement(), view.accessibilityRole() == .group {
        view.setAccessibilityLabel("TableRock workbench")
        root = view
        break
      }
      current = view.superview
    }
    guard let root,
      let splitGroup = firstAccessibilityElement(
        withRole: .splitGroup,
        below: root.accessibilityChildren() ?? []
      )
    else { return }

    let columns = accessibilityChildren(of: splitGroup).filter {
      accessibilityRole(of: $0) == .group
    }
    for (index, column) in columns.enumerated()
    where accessibilityLabel(of: column)?.isEmpty != false {
      setAccessibilityLabel(index == 0 ? "Navigation sidebar" : "Workspace", on: column)
    }
  }

  private func firstAccessibilityElement(
    withRole role: NSAccessibility.Role,
    below initialElements: [Any]
  ) -> Any? {
    var elements = initialElements
    for _ in 0..<4 {
      if let match = elements.first(where: { accessibilityRole(of: $0) == role }) {
        return match
      }
      elements = elements.flatMap(accessibilityChildren)
    }
    return nil
  }

  private func accessibilityRole(of element: Any) -> NSAccessibility.Role? {
    (element as? any NSAccessibilityProtocol)?.accessibilityRole()
  }

  private func accessibilityLabel(of element: Any) -> String? {
    (element as? any NSAccessibilityProtocol)?.accessibilityLabel()
  }

  private func accessibilityChildren(of element: Any) -> [Any] {
    (element as? any NSAccessibilityProtocol)?.accessibilityChildren() ?? []
  }

  private func setAccessibilityLabel(_ label: String, on element: Any) {
    (element as? any NSAccessibilityProtocol)?.setAccessibilityLabel(label)
  }
}
