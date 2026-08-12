import SwiftUI

struct SavedQueriesSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    NavigationStack {
      Group {
        if model.savedQueriesLoading && model.savedQueries.isEmpty {
          ProgressView("Loading saved queries…")
        } else if let error = model.savedQueriesError, model.savedQueries.isEmpty {
          ContentUnavailableView(
            "Saved queries failed", systemImage: "exclamationmark.triangle",
            description: Text(error)
          )
        } else if model.savedQueries.isEmpty {
          ContentUnavailableView(
            model.savedQuerySearch.isEmpty ? "No saved queries" : "No saved query matches",
            systemImage: "bookmark",
            description: Text(
              model.savedQuerySearch.isEmpty
                ? "Save current editor text to reuse it later."
                : "Try a different name or SQL-text search.")
          )
        } else {
          List(model.savedQueries, id: \.queryId) { item in
            HStack(spacing: 10) {
              Button {
                model.restoreSavedQuery(item)
              } label: {
                VStack(alignment: .leading, spacing: 5) {
                  Text(item.name).font(.headline)
                  Text(item.statementText)
                    .font(.system(.body, design: .monospaced))
                    .lineLimit(3)
                  Text("\(item.engine) · \(item.updatedAt)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
              }
              .buttonStyle(.plain)
              .accessibilityHint("Restore into the editor without running it")
              Button(role: .destructive) {
                model.pendingSavedQueryRemoval = item
              } label: {
                Image(systemName: "trash")
              }
              .buttonStyle(.borderless)
              .accessibilityLabel("Remove \(item.name)")
            }
            .padding(.vertical, 3)
          }
        }
      }
      .navigationTitle("Saved Queries")
      .searchable(text: $model.savedQuerySearch, prompt: "Search names and SQL text")
      .onChange(of: model.savedQuerySearch) { _, _ in
        Task { await model.refreshSavedQueries() }
      }
      .onChange(of: model.savedQueryEngine) { _, _ in
        Task { await model.refreshSavedQueries() }
      }
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Done") { dismiss() }
        }
        ToolbarItem(placement: .automatic) {
          Picker("Engine", selection: $model.savedQueryEngine) {
            Text("All engines").tag("")
            Text("PostgreSQL").tag("postgresql")
            Text("ClickHouse").tag("clickhouse")
            Text("Redis").tag("redis")
          }
        }
        ToolbarItem(placement: .primaryAction) {
          Button("Save Current…") { model.beginSaveCurrentQuery() }
        }
      }
    }
    .frame(minWidth: 700, minHeight: 500)
    .alert("Save Query", isPresented: $model.saveQueryDialog) {
      TextField("Name", text: $model.savedQueryName)
      Button("Save") { Task { await model.saveCurrentQuery() } }
      Button("Cancel", role: .cancel) { model.saveQueryDialog = false }
    } message: {
      Text("Save current editor text for the active database engine.")
    }
    .confirmationDialog(
      "Remove saved query?",
      isPresented: Binding(
        get: { model.pendingSavedQueryRemoval != nil },
        set: { if !$0 { model.pendingSavedQueryRemoval = nil } }
      ),
      presenting: model.pendingSavedQueryRemoval
    ) { _ in
      Button("Remove", role: .destructive) {
        Task { await model.removePendingSavedQuery() }
      }
      Button("Cancel", role: .cancel) { model.pendingSavedQueryRemoval = nil }
    } message: { item in
      Text("\(item.name) will be removed. Query history is unchanged.")
    }
  }
}

struct HistorySheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    NavigationStack {
      Group {
        if model.historyLoading && model.historyItems.isEmpty {
          ProgressView("Loading history…")
        } else if let error = model.historyError, model.historyItems.isEmpty {
          ContentUnavailableView(
            "History failed", systemImage: "exclamationmark.triangle",
            description: Text(error)
          )
        } else if model.historyItems.isEmpty {
          ContentUnavailableView(
            model.historySearch.isEmpty ? "No query history" : "No history matches",
            systemImage: "clock",
            description: Text(
              model.historySearch.isEmpty
                ? "Executed statements appear here when retention is enabled."
                : "Try a different SQL-text search.")
          )
        } else {
          List(model.historyItems, id: \.historyId) { item in
            Button {
              model.restoreHistory(item)
            } label: {
              VStack(alignment: .leading, spacing: 5) {
                Text(item.statementText ?? "SQL text not retained")
                  .font(.system(.body, design: .monospaced))
                  .lineLimit(3)
                Text(
                  [
                    item.engine, item.databaseName,
                    item.schemaName, item.outcome, item.createdAt,
                  ].compactMap { $0 }.joined(separator: " · ")
                )
                .font(.caption)
                .foregroundStyle(.secondary)
              }
              .frame(maxWidth: .infinity, alignment: .leading)
              .padding(.vertical, 3)
            }
            .buttonStyle(.plain)
            .disabled(item.statementText == nil)
            .accessibilityHint(
              item.statementText == nil
                ? "SQL text retention was disabled"
                : "Restore this statement into the editor without running it")
          }
        }
      }
      .navigationTitle("Query History")
      .searchable(text: $model.historySearch, prompt: "Search retained SQL text")
      .onChange(of: model.historySearch) { _, _ in
        Task { await model.refreshHistory() }
      }
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Done") { dismiss() }
        }
        ToolbarItem(placement: .automatic) {
          Picker("Retention", selection: $model.historyRetention) {
            Text("Full SQL").tag("full")
            Text("Metadata only").tag("metadata_only")
            Text("Private").tag("private")
          }
          .onChange(of: model.historyRetention) { _, value in
            Task { await model.setHistoryRetention(value) }
          }
        }
        ToolbarItem(placement: .automatic) {
          Button {
            Task { await model.refreshHistory() }
          } label: {
            Label("Refresh History", systemImage: "arrow.clockwise")
          }
          .disabled(model.historyLoading)
        }
      }
    }
    .frame(minWidth: 680, minHeight: 480)
  }
}
