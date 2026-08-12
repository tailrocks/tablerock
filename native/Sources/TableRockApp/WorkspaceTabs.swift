import SwiftUI

struct QueryTabStrip: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    // Hierarchy: tabs are content selectors, not a row of glass pills.
    // Only the selected tab uses glassProminent; unselected stay plain.
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 2) {
        ForEach(model.queryTabs) { tab in
          let selected =
            model.queryWorkbenchSelected && tab.id == model.selectedQueryTabId
          HStack(spacing: 0) {
            queryTabButton(tab: tab, selected: selected)
            Menu {
              Button("Rename…") { model.beginRenameQueryTab(tab) }
              Button("Close", role: .destructive) {
                model.requestCloseQueryTab(tab)
              }
              .accessibilityIdentifier("query.tab.close")
              .disabled(model.queryTabs.count == 1 || tab.isRunning)
            } label: {
              Image(systemName: tab.isRunning ? "progress.indicator" : "ellipsis")
                .font(.caption)
            }
            .menuStyle(.borderlessButton)
            .controlSize(.small)
            .accessibilityIdentifier("query.tab.actions.\(tab.id.uuidString.lowercased())")
            .accessibilityLabel("Actions for \(tab.title)")
          }
          .padding(.trailing, 2)
        }
        ForEach(model.objectTabs) { tab in
          let selected =
            !model.queryWorkbenchSelected && tab.id == model.selectedObjectTabId
          HStack(spacing: 0) {
            objectTabButton(tab: tab, selected: selected)
            Menu {
              if !tab.pinned {
                Button("Pin") { model.pinObjectTab(tab) }
              }
              Button("Refresh") { Task { await model.reloadObjectTab() } }
              Button("Close", role: .destructive) { model.closeObjectTab(tab) }
                .disabled(tab.isRunning)
            } label: {
              Image(systemName: tab.isRunning ? "progress.indicator" : "ellipsis")
                .font(.caption)
            }
            .menuStyle(.borderlessButton)
            .controlSize(.small)
            .accessibilityLabel("Actions for object \(tab.title)")
          }
          .padding(.trailing, 2)
        }
        Button {
          model.addQueryTab()
        } label: {
          Image(systemName: "plus")
        }
        .buttonStyle(.glass)
        .controlSize(.small)
        .accessibilityLabel("New query tab")
        .disabled(model.queryTabs.count + model.objectTabs.count >= 64)
      }
      .padding(.vertical, 2)
    }
    .accessibilityIdentifier("workbench.tab-strip")
  }

  @ViewBuilder
  private func queryTabButton(tab: NativeQueryTab, selected: Bool) -> some View {
    let label = WorkbenchTabLabel(title: tab.title, model: model)
    let id = "query.tab.\(tab.id.uuidString.lowercased())"
    if selected {
      Button {
        model.selectQueryTab(tab)
      } label: {
        label
      }
      .buttonStyle(.glassProminent)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Selected")
    } else {
      Button {
        model.selectQueryTab(tab)
      } label: {
        label
      }
      .buttonStyle(.plain)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Not selected")
    }
  }

  @ViewBuilder
  private func objectTabButton(tab: NativeObjectTab, selected: Bool) -> some View {
    let label = WorkbenchTabLabel(
      title: tab.title, model: model,
      leadingSystemImage: tab.pinned ? "pin.fill" : "eye")
    let id = "object.tab.\(tab.id.uuidString.lowercased())"
    if selected {
      Button {
        model.selectObjectTab(tab)
      } label: {
        label
      }
      .buttonStyle(.glassProminent)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Selected")
    } else {
      Button {
        model.selectObjectTab(tab)
      } label: {
        label
      }
      .buttonStyle(.plain)
      .controlSize(.small)
      .accessibilityIdentifier(id)
      .accessibilityValue("Not selected")
    }
  }
}

private struct WorkbenchTabLabel: View {
  let title: String
  let model: WorkbenchPresentationStore
  var leadingSystemImage: String?

  init(title: String, model: WorkbenchPresentationStore, leadingSystemImage: String? = nil) {
    self.title = title
    self.model = model
    self.leadingSystemImage = leadingSystemImage
  }

  var body: some View {
    HStack(spacing: 4) {
      if let leadingSystemImage { Image(systemName: leadingSystemImage) }
      Text(title)
      if model.activeProductionWarning {
        Image(systemName: "exclamationmark.triangle.fill")
          .accessibilityLabel("Production")
      } else if let environment = model.activeEnvironmentLabel {
        Text(environment).font(.caption2)
      }
      if model.activeSafetyLabel == "Read only" {
        Image(systemName: "lock.fill").accessibilityLabel("Read only")
      }
    }
    .accessibilityElement(children: .combine)
  }
}
