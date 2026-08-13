import SwiftUI

/// Window-level Native Workbench commands. Surface-specific actions remain in
/// their opaque content planes; the unified toolbar stays navigation-only.
struct WorkbenchToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    ToolbarItem(id: "connection", placement: .navigation) {
      Menu {
        ForEach(model.profiles, id: \.idBytes) { profile in
          Button {
            Task { await model.connect(profile) }
          } label: {
            Label(
              profile.name,
              systemImage: profile.idBytes == model.activeProfileId ? "checkmark" : "cylinder"
            )
          }
        }
      } label: {
        Label(
          model.activeProfile?.name ?? "Connections",
          systemImage: model.sessionHex == nil ? "cylinder" : "cylinder.split.1x2"
        )
      }
      .help("Database connection")
      .accessibilityIdentifier("toolbar.connection")
    }

    ToolbarItem(id: "new-connection", placement: .primaryAction) {
      Button {
        model.createProfile()
      } label: {
        Label("New Connection", systemImage: "externaldrive.badge.plus")
      }
      .accessibilityIdentifier("toolbar.new-connection")
    }

    ToolbarItem(id: "new-query", placement: .primaryAction) {
      Button {
        model.addQueryTab()
      } label: {
        Label("New Query", systemImage: "plus.rectangle.on.rectangle")
      }
      .disabled(model.sessionHex == nil)
      .accessibilityIdentifier("toolbar.new-query")
    }

    ToolbarItem(id: "history", placement: .primaryAction) {
      Button {
        Task { await model.presentHistory() }
      } label: {
        Label("Query History", systemImage: "clock.arrow.circlepath")
      }
      .disabled(model.sessionHex == nil)
      .accessibilityIdentifier("toolbar.history")
    }

    ToolbarItem(id: "inspector", placement: .primaryAction) {
      Button {
        model.toggleValueInspector()
      } label: {
        Label("Toggle Inspector", systemImage: "sidebar.right")
      }
      .disabled(model.resultTable?.rows.isEmpty ?? true)
      .accessibilityIdentifier("toolbar.inspector")
    }

    ToolbarItem(id: "review", placement: .primaryAction) {
      Button {
        model.presentActiveReview()
      } label: {
        Label("Review Changes", systemImage: "checklist")
      }
      .disabled(!model.changeReviewOpen)
      .accessibilityIdentifier("toolbar.review")
    }
  }
}
