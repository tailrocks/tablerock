import SwiftUI

struct ObjectWorkbenchView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    if let tab = model.selectedObjectTab {
      VStack(alignment: .leading, spacing: 6) {
        HStack(spacing: 8) {
          Label(tab.title, systemImage: tab.pinned ? "pin.fill" : "eye")
            .font(.subheadline.weight(.semibold))
          Text(tab.kind).font(.caption.monospaced()).foregroundStyle(.secondary)
          if !tab.kind.hasPrefix("redis_key_") {
            Picker(
              "Object section",
              selection: Binding(
                get: { tab.selectedSection },
                set: { section in
                  tab.selectedSection = section
                  if section == "structure" {
                    Task { await model.loadObjectStructure() }
                  }
                }
              )
            ) {
              Text("Data").tag("data")
              Text("Structure").tag("structure")
            }
            .pickerStyle(.segmented)
            .frame(width: 180)
          }
          Spacer(minLength: 0)
          GlassEffectContainer {
            HStack(spacing: 6) {
              if !tab.pinned {
                Button("Pin") { model.pinObjectTab(tab) }
                  .buttonStyle(.glass)
              }
              Button("Refresh") { Task { await model.reloadObjectTab() } }
                .buttonStyle(.glass)
                .disabled(tab.isRunning)
              if model.sqlInsertCopyAvailable {
                Button("Import CSV") { Task { await model.chooseCsvImport() } }
                  .buttonStyle(.glass)
                  .accessibilityIdentifier("import.csv.open")
                  .disabled(tab.isRunning)
              }
              Button("Close", role: .destructive) { model.closeObjectTab(tab) }
                .buttonStyle(.glass)
                .disabled(tab.isRunning)
            }
            .controlSize(.small)
          }
        }
        if tab.isRunning { ProgressView("Loading \(tab.title)…") }
        if let summary = tab.summary {
          Text(summary).font(.caption).foregroundStyle(.secondary)
        }
        if tab.selectedSection == "data", let table = tab.resultTable,
          !tab.kind.hasPrefix("redis_key_")
        {
          objectSortBar(tab: tab, table: table)
          objectFilterBar(tab: tab, table: table)
        }
        if let error = tab.error {
          Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)
        }
        if let view = tab.redisView {
          redisKeyObjectView(view: view)
          if view.nextSkip != nil {
            Button("Load more entries") { Task { await model.loadMoreRedisKey() } }
              .disabled(tab.isRunning)
          }
        } else if tab.selectedSection == "structure" {
          objectStructureView(tab: tab)
        } else if let table = tab.resultTable {
          ResultGridWithInspector(table: table, minimumHeight: 180)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
          if tab.nextStartRow != nil {
            Button("Load more rows") { Task { await model.loadMoreObjectRows() } }
              .buttonStyle(.glass)
              .controlSize(.small)
          }
        } else if !tab.isRunning && tab.error == nil {
          ContentUnavailableView(
            "No object rows", systemImage: "tablecells",
            description: Text("Refresh to browse this object again.")
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    } else {
      ContentUnavailableView("No object tab", systemImage: "tablecells")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
  }
}
