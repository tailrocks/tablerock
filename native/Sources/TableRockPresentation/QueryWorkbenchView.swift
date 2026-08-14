import Foundation
import SwiftUI
import TableRockFeature

struct QueryWorkbenchView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    @Bindable var tab = model.activeQueryTabForPresentation

    VStack(alignment: .leading, spacing: 0) {
      QueryWorkbenchHeader(tab: tab)

      VSplitView {
        SqlTextEditor(
          text: $model.queryText,
          selection: $model.queryEditorSelection,
          isRunning: model.isRunning
        )
        .frame(minHeight: 150)
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Query editor pane")
        .task(id: model.queryText) {
          try? await Task.sleep(for: .milliseconds(300))
          guard !Task.isCancelled else { return }
          await model.persistSessionIntent()
        }

        QueryResultPlane(tab: tab)
          .frame(minHeight: 190)
          .accessibilityElement(children: .contain)
          .accessibilityLabel("Query result pane")
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    .background(Color(nsColor: .textBackgroundColor))
    .accessibilityIdentifier("query.workbench")
  }
}

private struct QueryWorkbenchHeader: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeQueryTab

  private var caretChip: String {
    SqlEditorMetrics.statusChip(
      text: model.queryText,
      selection: model.queryEditorSelection,
      isRunning: model.isRunning,
      hasError: tab.queryError != nil
    )
  }

  var body: some View {
    HStack(spacing: 9) {
      Image(systemName: "chevron.left.forwardslash.chevron.right")
        .foregroundStyle(.blue)
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 0) {
        Text(tab.title)
          .font(.headline)
          .lineLimit(1)
          .accessibilityIdentifier("query.header")
        Text(queryContext)
          .font(.caption2)
          .foregroundStyle(Color(nsColor: .textColor))
          .lineLimit(1)
      }
      Spacer(minLength: 8)
      Text(caretChip)
        .font(.title3.weight(.heavy).monospaced())
        .statusMetricStyle()
        .lineLimit(1)
        .truncationMode(.middle)
        .accessibilityIdentifier("query.editor.metrics")
        .accessibilityValue(caretChip)
      if model.activeProductionWarning {
        Label("PRODUCTION", systemImage: "exclamationmark.triangle.fill")
          .font(.caption2.weight(.bold))
          .foregroundStyle(.orange)
          .accessibilityLabel("Production — writes need review")
      }
      NativeActionMenu(
        title: "",
        systemImage: "ellipsis.circle",
        accessibilityLabel: "Query actions",
        entries: queryActionEntries
      )

      Button {
        Task { await model.runExplain() }
      } label: {
        Label("Explain", systemImage: "chart.xyaxis.line")
      }
      .buttonStyle(.bordered)
      .disabled(model.isRunning || model.isCatalogRefreshing)
      .accessibilityIdentifier("query.explain")

      if model.isRunning {
        Button(role: .destructive) {
          Task { await model.cancel() }
        } label: {
          Label("Cancel", systemImage: "stop.fill")
        }
        .buttonStyle(.bordered)
        .accessibilityIdentifier("query.cancel")
      } else {
        Button {
          Task { await model.runQuery() }
        } label: {
          Label("Run", systemImage: "play.fill")
        }
        .buttonStyle(.borderedProminent)
        .keyboardShortcut("r", modifiers: .command)
        .disabled(model.isCatalogRefreshing)
        .help("Run selection or whole buffer")
        .accessibilityIdentifier("query.run")
      }
    }
    .controlSize(.small)
    .padding(.horizontal, 12)
    .frame(height: 48)
    .background(Color(nsColor: .windowBackgroundColor))
    .overlay(alignment: .bottom) { Divider() }
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("query.header.container")
  }

  private var queryActionEntries: [NativeActionMenuEntry] {
    var entries: [NativeActionMenuEntry] = [
      .command(title: "Saved Queries", systemImage: "bookmark") {
        Task { await model.presentSavedQueries() }
      },
      .command(
        title: "Save Query",
        systemImage: "bookmark.badge.plus",
        isEnabled: model.queryWorkbenchSelected
      ) {
        model.beginSaveCurrentQuery()
      },
      .separator,
      .command(title: "Open SQL File…", systemImage: "folder") {
        model.requestOpenSqlFile()
      },
      .command(title: "Save SQL File", systemImage: "square.and.arrow.down") {
        Task { await model.saveSqlFile() }
      },
      .command(title: "Save SQL File As…", systemImage: "square.and.arrow.down.on.square") {
        Task { await model.saveSqlFile(saveAs: true) }
      },
      .command(
        title: "Reload SQL File",
        systemImage: "arrow.clockwise",
        isEnabled: model.sqlFile != nil
      ) {
        Task { await model.reloadSqlFile() }
      },
      .separator,
      .command(
        title: "Find and Replace…",
        systemImage: "magnifyingglass",
        identifier: "query.find"
      ) {
        model.findReplacePresented = true
      },
    ]
    if model.connectedEngine == "redis" {
      entries.append(
        .command(
          title: "Redis Overview",
          systemImage: "gauge.with.dots.needle.bottom.50percent",
          isEnabled: !model.redisOverviewLoading
        ) {
          Task { await model.showRedisOverview() }
        })
    }
    return entries
  }

  private var queryContext: String {
    let connection = model.activeProfile?.name ?? model.connectedEngine
    if let file = model.sqlFile {
      return "Query · \(connection) · \(URL(fileURLWithPath: file.path).lastPathComponent)"
    }
    return "Query · \(connection)"
  }
}

private struct QueryResultPlane: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeQueryTab

  @State private var quickFilterPresented = false

  private var queryStatus: String {
    tab.queryError ?? tab.cancelOutcome ?? tab.querySummary ?? "Idle"
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      HStack(spacing: 8) {
        Picker(
          "Query result section",
          selection: Binding(
            get: { tab.selectedResultSection },
            set: { tab.selectedResultSection = $0 })
        ) {
          Text("Results").tag("results")
          Text("Messages").tag("messages")
          Text("Plan").tag("plan")
        }
        .labelsHidden()
        .pickerStyle(.segmented)
        .frame(width: 240)
        .accessibilityIdentifier("query.result-section")

        Spacer(minLength: 8)
        if let resultTable = tab.resultTable {
          Button {
            quickFilterPresented = true
          } label: {
            Image(
              systemName: model.loadedRowQuickFilter.isEmpty
                ? "magnifyingglass" : "line.3.horizontal.decrease.circle.fill")
          }
          .buttonStyle(.plain)
          .help("Filter loaded rows")
          .accessibilityLabel("Filter loaded rows")
          .accessibilityIdentifier("query.quick-filter.open")
          .popover(isPresented: $quickFilterPresented, arrowEdge: .bottom) {
            QueryLoadedRowFilter(table: resultTable)
          }
          if tab.nextStartRow != nil {
            Button("Load more") { Task { await model.loadMore() } }
              .accessibilityIdentifier("results.next-page")
          }
          ResultCopyMenu()
          ResultExportMenu()
          ResultTransferFeedback()
          Button {
            Task { await model.openRelationContinuumFromSelection() }
          } label: {
            Image(systemName: "arrow.triangle.branch")
          }
          .buttonStyle(.plain)
          .disabled(!model.canOpenRelationContinuum)
          .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
          .help("Row Continuum: related rows for this cell (⌘⌥→)")
          .accessibilityLabel("Open Row Continuum")
          .accessibilityIdentifier("relation.continuum.open")
        }
        if tab.isRunning {
          ProgressView()
            .controlSize(.small)
            .accessibilityLabel("Running query")
        }
        Text(queryStatus)
          .font(.title3.weight(.heavy).monospacedDigit())
          .statusMetricStyle()
          .lineLimit(1)
          .truncationMode(.middle)
          .textSelection(.enabled)
          .accessibilityIdentifier("query.status")
          .accessibilityValue(queryStatus)
      }
      .font(.caption)
      .controlSize(.small)
      .padding(.horizontal, 10)
      .frame(height: 38)
      .background(Color(nsColor: .controlBackgroundColor))
      .overlay(alignment: .bottom) { Divider() }

      switch tab.selectedResultSection {
      case "messages":
        QueryMessagesPlane(tab: tab)
      case "plan":
        QueryPlanPlane(plan: tab.explainPlan)
      default:
        if let table = tab.resultTable {
          ResultGridWithInspector(
            table: table,
            minimumHeight: 140,
            exposesResultPaging: false,
            showsUtilityRail: false
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
          ContentUnavailableView(
            "No result yet",
            systemImage: "tablecells",
            description: Text("Run a query to fill the workbench grid.")
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .accessibilityIdentifier("workbench.query.empty-result")
        }
      }
    }
    .background(Color(nsColor: .textBackgroundColor))
  }
}

private struct QueryLoadedRowFilter: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let table: WorkbenchTable

  private var visibleRowCount: Int {
    let term = model.loadedRowQuickFilter.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !term.isEmpty else { return table.rows.count }
    return table.rows.filter { row in
      row.contains { value in
        value.range(of: term, options: [.caseInsensitive, .diacriticInsensitive]) != nil
      }
    }.count
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text("Loaded Rows").font(.headline)
      TextField(
        "Filter loaded rows",
        text: Binding(
          get: { model.loadedRowQuickFilter },
          set: { model.loadedRowQuickFilter = $0 })
      )
      .accessibilityIdentifier("results.quick-filter")
      let status = "Loaded rows only · \(visibleRowCount)/\(table.rows.count)"
      Text(status)
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("results.quick-filter.status")
        .accessibilityValue(status)
    }
    .padding(14)
    .frame(width: 300)
  }
}

private struct QueryMessagesPlane: View {
  let tab: NativeQueryTab

  private var messages: [(String, String)] {
    [
      tab.queryError.map { ("Query error", $0) },
      tab.cancelOutcome.map { ("Cancellation", $0) },
      tab.querySummary.map { ("Result", $0) },
      tab.reviewError.map { ("Review error", $0) },
      tab.reviewOutcome.map { ("Review", $0) },
      tab.sqlFileError.map { ("SQL file", $0) },
    ].compactMap { $0 }
  }

  var body: some View {
    if messages.isEmpty {
      ContentUnavailableView(
        "No messages",
        systemImage: "text.bubble",
        description: Text("Query notices, errors, and outcomes appear here.")
      )
      .frame(maxWidth: .infinity, maxHeight: .infinity)
    } else {
      ScrollView {
        VStack(alignment: .leading, spacing: 12) {
          ForEach(Array(messages.enumerated()), id: \.offset) { _, message in
            VStack(alignment: .leading, spacing: 4) {
              Text(message.0).font(.caption.weight(.semibold))
              Text(message.1)
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
          }
        }
        .padding(16)
      }
      .accessibilityIdentifier("query.messages")
    }
  }
}

private struct QueryPlanPlane: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let plan: String?

  var body: some View {
    if let plan, !plan.isEmpty {
      VStack(alignment: .leading, spacing: 0) {
        HStack {
          Spacer()
          Button("Copy Plan", systemImage: "doc.on.doc") { model.copyExplainPlan() }
            .controlSize(.small)
            .accessibilityIdentifier("explain.copy")
        }
        .padding(.horizontal, 10)
        .frame(height: 38)
        .background(Color(nsColor: .controlBackgroundColor))
        .overlay(alignment: .bottom) { Divider() }
        ScrollView([.horizontal, .vertical]) {
          Text(plan)
            .font(.system(.body, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(16)
            .accessibilityIdentifier("explain.plan")
        }
      }
    } else {
      ContentUnavailableView(
        "No plan",
        systemImage: "chart.xyaxis.line",
        description: Text("Explain the active statement to inspect its plan.")
      )
      .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
  }
}
