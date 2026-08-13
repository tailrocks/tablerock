import SwiftUI

/// Dense document tabs from the confirmed Native Workbench. Tabs are content
/// selectors on an opaque underlay, never a row of independent glass pills.
struct QueryTabStrip: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 1) {
        ForEach(model.orderedWorkspaceTabs) { reference in
          switch reference {
          case .query(let id):
            if let tab = model.queryTabs.first(where: { $0.id == id }) {
              QueryDocumentTab(tab: tab)
            }
          case .object(let id):
            if let tab = model.objectTabs.first(where: { $0.id == id }) {
              ObjectDocumentTab(tab: tab)
            }
          }
        }

        Button {
          model.addQueryTab()
        } label: {
          Image(systemName: "plus")
            .frame(width: 28, height: 28)
        }
        .buttonStyle(.plain)
        .foregroundStyle(.secondary)
        .accessibilityLabel("New query tab")
        .disabled(model.queryTabs.count + model.objectTabs.count >= 64)

        Spacer(minLength: 0)
      }
      .padding(.horizontal, 8)
    }
    .frame(height: 34)
    .background(Color(nsColor: .windowBackgroundColor))
    .overlay(alignment: .bottom) { Divider() }
    .accessibilityIdentifier("workbench.tab-strip")
  }
}

private struct QueryDocumentTab: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeQueryTab

  private var selected: Bool {
    model.queryWorkbenchSelected && tab.id == model.selectedQueryTabId
  }

  var body: some View {
    HStack(spacing: 0) {
      WorkbenchDocumentTab(
        title: tab.title,
        symbol: "chevron.left.forwardslash.chevron.right",
        selected: selected,
        dirty: tab.statementText != tab.sqlFileBaseline,
        running: tab.isRunning
      ) {
        model.selectQueryTab(tab)
      }
      .accessibilityIdentifier("query.tab.\(tab.id.uuidString.lowercased())")
      .accessibilityValue(selected ? "Selected" : "Not selected")

      Button(role: .destructive) {
        model.requestCloseQueryTab(tab)
      } label: {
        Image(systemName: "xmark")
          .font(.system(size: 8, weight: .bold))
          .foregroundStyle(.tertiary)
          .frame(width: 18, height: 28)
      }
      .buttonStyle(.plain)
      .disabled(model.queryTabs.count == 1 || tab.isRunning)
      .accessibilityIdentifier("query.tab.actions.\(tab.id.uuidString.lowercased())")
      .accessibilityLabel("Close \(tab.title)")
    }
    .modifier(SelectedDocumentTabBackground(selected: selected))
    .contextMenu {
      Button("Rename…") { model.beginRenameQueryTab(tab) }
      Button("Close", role: .destructive) { model.requestCloseQueryTab(tab) }
        .accessibilityIdentifier("query.tab.close")
        .disabled(model.queryTabs.count == 1 || tab.isRunning)
    }
  }
}

private struct ObjectDocumentTab: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab

  private var selected: Bool {
    !model.queryWorkbenchSelected && tab.id == model.selectedObjectTabId
  }

  var body: some View {
    HStack(spacing: 0) {
      WorkbenchDocumentTab(
        title: tab.title,
        symbol: "tablecells",
        selected: selected,
        dirty: model.changeReviewOpen && selected,
        running: tab.isRunning
      ) {
        model.selectObjectTab(tab)
      }
      .accessibilityIdentifier("object.tab.\(tab.id.uuidString.lowercased())")
      .accessibilityValue(selected ? "Selected" : "Not selected")

      Button(role: .destructive) {
        model.closeObjectTab(tab)
      } label: {
        Image(systemName: "xmark")
          .font(.system(size: 8, weight: .bold))
          .foregroundStyle(.tertiary)
          .frame(width: 18, height: 28)
      }
      .buttonStyle(.plain)
      .disabled(tab.isRunning)
      .accessibilityLabel("Close \(tab.title)")
    }
    .modifier(SelectedDocumentTabBackground(selected: selected))
    .contextMenu {
      if !tab.pinned { Button("Pin") { model.pinObjectTab(tab) } }
      Button("Refresh") { Task { await model.reloadObjectTab() } }
      Button("Close", role: .destructive) { model.closeObjectTab(tab) }
        .disabled(tab.isRunning)
    }
  }
}

private struct SelectedDocumentTabBackground: ViewModifier {
  let selected: Bool

  func body(content: Content) -> some View {
    content
      .padding(.trailing, 2)
      .background(selected ? Color(nsColor: .controlBackgroundColor) : .clear)
      .clipShape(.rect(cornerRadius: 7))
      .overlay {
        if selected {
          RoundedRectangle(cornerRadius: 7)
            .stroke(Color(nsColor: .separatorColor), lineWidth: 0.5)
        }
      }
  }
}

private struct WorkbenchDocumentTab: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let title: String
  let symbol: String
  let selected: Bool
  let dirty: Bool
  let running: Bool
  let action: () -> Void

  var body: some View {
    Button(action: action) {
      HStack(spacing: 6) {
        Image(systemName: running ? "progress.indicator" : symbol)
          .font(.caption)
          .foregroundStyle(selected ? Color.accentColor : .secondary)
        Text(title)
          .font(.caption)
          .lineLimit(1)
        if dirty {
          Circle()
            .fill(.orange)
            .frame(width: 6, height: 6)
            .accessibilityHidden(true)
        }
        EnvironmentSafetyTabIndicators(model: model)
      }
      .padding(.leading, 10)
      .frame(height: 28)
      .contentShape(.rect)
    }
    .buttonStyle(.plain)
    .accessibilityLabel(accessibilityLabel)
    .accessibilityHint(dirty ? "Pending changes" : "No pending changes")
  }

  private var accessibilityLabel: String {
    var parts = [title]
    if model.activeProductionWarning {
      parts.append("Production")
    } else if let environment = model.activeEnvironmentLabel {
      parts.append("Environment \(environment)")
    }
    if let safety = model.activeSafetyLabel { parts.append(safety) }
    return parts.joined(separator: ", ")
  }
}

private struct EnvironmentSafetyTabIndicators: View {
  let model: WorkbenchPresentationStore

  var body: some View {
    Group {
      if model.activeProductionWarning {
        Image(systemName: "exclamationmark.triangle.fill")
          .foregroundStyle(.orange)
          .accessibilityLabel("Production")
      } else if let environment = model.activeEnvironmentLabel {
        Text(environment)
          .font(.caption2)
          .foregroundStyle(.secondary)
          .accessibilityLabel("Environment \(environment)")
      }
      if model.activeSafetyLabel == "Read only" {
        Image(systemName: "lock.fill")
          .foregroundStyle(.secondary)
          .accessibilityLabel("Read only")
      }
    }
    .accessibilityHidden(true)
  }
}
