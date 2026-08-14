import SwiftUI

struct ObjectWorkbenchView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    if let tab = model.selectedObjectTab {
      VStack(alignment: .leading, spacing: 0) {
        ObjectWorkbenchHeader(tab: tab)
        if tab.selectedSection == "data", let table = tab.resultTable,
          !tab.kind.hasPrefix("redis_key_")
        {
          objectBrowseRail(tab: tab, table: table)
        }
        if let error = tab.error, tab.resultTable != nil || tab.redisView != nil {
          Label(error, systemImage: "exclamationmark.triangle")
            .font(.caption)
            .foregroundStyle(.red)
            .textSelection(.enabled)
            .padding(.horizontal, 10)
            .frame(minHeight: 30)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.red.opacity(0.06))
            .overlay(alignment: .bottom) { Divider() }
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
          ResultGridWithInspector(table: table, minimumHeight: 180, showsUtilityRail: false)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if tab.isRunning {
          ProgressView("Loading \(tab.title)…")
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .accessibilityIdentifier("object.loading")
        } else if let error = tab.error {
          ContentUnavailableView(
            "Object unavailable", systemImage: "exclamationmark.triangle",
            description: Text(error)
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .accessibilityIdentifier("object.error")
        } else if !tab.isRunning && tab.error == nil {
          ContentUnavailableView(
            "No object rows", systemImage: "tablecells",
            description: Text("Refresh to browse this object again.")
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .accessibilityIdentifier("object.empty")
        }
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
      .background(Color(nsColor: .textBackgroundColor))
    } else {
      ContentUnavailableView("No object tab", systemImage: "tablecells")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
  }
}

private struct ObjectWorkbenchHeader: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab

  var body: some View {
    HStack(spacing: 9) {
      Image(systemName: tab.kind.hasPrefix("redis_key_") ? "key.horizontal" : "tablecells")
        .foregroundStyle(.blue)
      VStack(alignment: .leading, spacing: 0) {
        HStack(spacing: 5) {
          Text(tab.title)
            .font(.headline)
            .lineLimit(1)
          if tab.pinned {
            Image(systemName: "pin.fill")
              .font(.caption2)
              .foregroundStyle(.secondary)
              .accessibilityLabel("Pinned")
          }
        }
        Text(contextDetail)
          .font(.caption2)
          .foregroundStyle(.primary)
          .lineLimit(1)
      }
      Spacer(minLength: 8)
      if tab.isRunning, tab.resultTable != nil || tab.redisView != nil {
        ProgressView()
          .controlSize(.small)
          .accessibilityLabel("Loading \(tab.title)")
      }
      if tab.resultTable != nil {
        ResultCopyMenu()
        ResultExportMenu()
        ResultTransferFeedback()
      }
      NativeActionMenu(
        title: "",
        systemImage: "ellipsis.circle",
        accessibilityLabel: "Object actions",
        identifier: tab.selectedSection == "structure" ? "structure.actions" : "object.actions",
        entries: objectActionEntries
      )
    }
    .controlSize(.small)
    .padding(.horizontal, 12)
    .frame(height: 48)
    .background(Color(nsColor: .windowBackgroundColor))
    .overlay(alignment: .bottom) { Divider() }
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("object.header")
  }

  private var objectActionEntries: [NativeActionMenuEntry] {
    var entries: [NativeActionMenuEntry] = []
    if !tab.pinned {
      entries.append(.command(title: "Pin", systemImage: "pin") { model.pinObjectTab(tab) })
    }
    entries.append(
      .command(
        title: "Refresh",
        systemImage: "arrow.clockwise",
        isEnabled: !tab.isRunning
      ) {
        Task { await model.reloadObjectTab() }
      })
    if model.canOpenRelationContinuum {
      entries.append(
        .command(title: "Open Row Continuum", systemImage: "arrow.triangle.branch") {
          Task { await model.openRelationContinuumFromSelection() }
        })
    }
    if model.sqlInsertCopyAvailable {
      entries.append(
        .command(
          title: "Import CSV",
          systemImage: "square.and.arrow.down",
          identifier: "import.csv.open",
          isEnabled: !tab.isRunning
        ) {
          Task { await model.chooseCsvImport() }
        })
    }
    if tab.selectedSection == "structure" {
      entries.append(.separator)
      entries.append(
        .command(
          title: "Copy DDL",
          systemImage: "doc.on.doc",
          isEnabled: tab.structure?.ddl.isEmpty == false
        ) {
          model.copyStructureDdl(tab.structure?.ddl ?? "")
        })
      entries.append(
        .command(
          title: "Change Structure…",
          systemImage: "slider.horizontal.3",
          identifier: "structure.change.open",
          isEnabled: model.canEditSelectedStructure
        ) {
          model.showDdlChange()
        })
      entries.append(
        .command(
          title: "Table Operations…",
          systemImage: "wrench.and.screwdriver",
          identifier: "table-operation.open",
          isEnabled: model.canOperateSelectedTable
        ) {
          model.showTableOperation()
        })
    }
    entries.append(.separator)
    entries.append(
      .command(title: "Close", systemImage: "xmark", isEnabled: !tab.isRunning) {
        model.closeObjectTab(tab)
      })
    return entries
  }

  private var contextDetail: String {
    let engine = engineLabel(model.connectedEngine)
    guard let nodes = model.catalogSnapshot,
      let node = nodes.first(where: { $0.idBytes == tab.catalogNodeId })
    else {
      return "\(kindLabel(tab.kind)) · \(engine)"
    }
    var names: [String] = []
    var parentId = node.parentIdBytes
    var visited = Set<Data>()
    while let id = parentId, !visited.contains(id),
      let parent = nodes.first(where: { $0.idBytes == id })
    {
      visited.insert(id)
      names.append(parent.name)
      parentId = parent.parentIdBytes
    }
    let namespace = names.reversed().joined(separator: ".")
    return namespace.isEmpty ? "\(kindLabel(tab.kind)) · \(engine)" : "\(namespace) · \(engine)"
  }

  private func engineLabel(_ engine: String) -> String {
    switch engine.lowercased() {
    case "postgresql": "PostgreSQL"
    case "clickhouse": "ClickHouse"
    case "redis": "Redis"
    case "sqlite": "SQLite"
    default: engine
    }
  }

  private func kindLabel(_ kind: String) -> String {
    kind.replacingOccurrences(of: "_", with: " ").capitalized
  }
}
