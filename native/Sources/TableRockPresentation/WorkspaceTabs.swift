import SwiftUI

/// Dense document tabs from the confirmed Native Workbench. Tabs are content
/// selectors on an opaque underlay, never a row of independent glass pills.
struct QueryTabStrip: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 1) {
        ForEach(model.queryTabs) { tab in
          let selected = model.queryWorkbenchSelected && tab.id == model.selectedQueryTabId
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

            Menu {
              Button("Rename…") { model.beginRenameQueryTab(tab) }
              Button("Close", role: .destructive) { model.requestCloseQueryTab(tab) }
                .accessibilityIdentifier("query.tab.close")
                .disabled(model.queryTabs.count == 1 || tab.isRunning)
            } label: {
              Image(systemName: "xmark")
                .font(.system(size: 8, weight: .bold))
                .foregroundStyle(.tertiary)
                .frame(width: 18, height: 28)
            }
            .menuStyle(.borderlessButton)
            .accessibilityIdentifier("query.tab.actions.\(tab.id.uuidString.lowercased())")
            .accessibilityLabel("Actions for \(tab.title)")
          }
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

        ForEach(model.objectTabs) { tab in
          let selected = !model.queryWorkbenchSelected && tab.id == model.selectedObjectTabId
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

            Menu {
              if !tab.pinned { Button("Pin") { model.pinObjectTab(tab) } }
              Button("Refresh") { Task { await model.reloadObjectTab() } }
              Button("Close", role: .destructive) { model.closeObjectTab(tab) }
                .disabled(tab.isRunning)
            } label: {
              Image(systemName: "xmark")
                .font(.system(size: 8, weight: .bold))
                .foregroundStyle(.tertiary)
                .frame(width: 18, height: 28)
            }
            .menuStyle(.borderlessButton)
            .accessibilityLabel("Actions for object \(tab.title)")
          }
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

private struct WorkbenchDocumentTab: View {
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
      }
      .padding(.leading, 10)
      .frame(height: 28)
      .contentShape(.rect)
    }
    .buttonStyle(.plain)
    .accessibilityLabel(title)
    .accessibilityHint(dirty ? "Pending changes" : "No pending changes")
  }
}
