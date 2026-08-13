import Foundation
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  /// Submit a catalog refresh and poll events until the page arrives, then
  /// decode the v1 page envelope. Proves the operation/event/page flow.
  /// Submit an operation and poll events until the result page arrives.
  /// Returns the decoded table, or nil on terminal-without-page.
  func fetchPage(
    intent: String,
    statement: String?,
    tab: NativeQueryTab,
    bindings: [WorkbenchQueryParameter]? = nil
  ) async throws -> WorkbenchTable? {
    guard let client, let session = sessionData else { return nil }
    let operationId =
      if let bindings, let statement {
        try await client.submitNamed(session: session, statement: statement, bindings: bindings)
      } else {
        try await client.submit(session: session, intent: intent, statement: statement)
      }
    tab.activeOperationId = operationId
    tab.isRunning = true
    tab.cancelOutcome = nil
    queryStateRevision &+= 1
    defer {
      tab.activeOperationId = nil
      tab.isRunning = false
      queryStateRevision &+= 1
    }
    let projection = try await client.finish(operationId: operationId)
    tab.writeOutcome = projection.outcome
    if projection.historyFailed {
      profileActionError = "Query completed, but local history could not be saved"
    }
    if let env = projection.envelope {
      tab.resultIdData = env.resultId
      tab.resultRevision = env.revision
      tab.nextStartRow =
        env.rowCount == 500
        ? env.startRow + UInt64(env.rowCount) : nil
    }
    return projection.table
  }

  func cancel() async {
    if selectedWorkbenchKind == "object", let tab = activeObjectTab {
      guard let client, let operationId = tab.activeOperationId else { return }
      do {
        let outcome = try await client.cancel(operationId: operationId)
        tab.summary = cancelOutcomeText(outcome)
      } catch { tab.error = "Cancel failed: \(error)" }
      return
    }
    let tab = activeQueryTab
    guard let client, let operationId = tab.activeOperationId else { return }
    do {
      let outcome = try await client.cancel(operationId: operationId)
      tab.cancelOutcome = cancelOutcomeText(outcome)
    } catch {
      tab.cancelOutcome = "Cancel failed: \(error)"
    }
    queryStateRevision &+= 1
  }

  /// Fetch the next page of the current result and append its rows.
  func loadMore() async {
    let tab = activeQueryTab
    guard let client, let resultId = tab.resultIdData, let start = tab.nextStartRow else {
      return
    }
    do {
      let (more, env) = try await client.fetchPage(
        resultId: resultId, startRow: start, revision: tab.resultRevision)
      if more.rows.isEmpty {
        tab.nextStartRow = nil
        return
      }
      if let table = tab.resultTable {
        guard let table = table.appending(more) else {
          tab.queryError = "Load more returned incompatible page metadata"
          return
        }
        tab.resultTable = table
        tab.querySummary =
          "result · \(counted(table.columns.count, "column")) · \(counted(table.rows.count, "row")) loaded"
      }
      tab.nextStartRow =
        env.rowCount == 500
        ? env.startRow + UInt64(env.rowCount) : nil
    } catch {
      tab.queryError = "Load more failed: \(error)"
    }
  }

  private func cancelOutcomeText(_ outcome: WorkbenchCancelOutcome) -> String {
    guard let runtime = outcome.runtime, !runtime.isEmpty else { return outcome.core }
    return "\(outcome.core) · \(runtime)"
  }

  func browse(expandedNodeKey: String? = nil) async {
    guard !isRunning, !isCatalogRefreshing else { return }
    guard let client, let session = sessionData else { return }
    let hadSnapshot = catalogSnapshot != nil
    catalogRefreshState = .loading(nodeKey: expandedNodeKey)
    catalogSummary = nil
    catalogError = nil
    do {
      let parentId = expandedNodeKey.flatMap { key in
        catalogSnapshot?.first(where: { catalogNodeKey($0.idBytes) == key })?.idBytes
      }
      let loaded = try await client.refreshCatalog(
        session: session,
        parentNodeId: parentId
      )
      if let parentId {
        var retained = catalogSnapshot ?? []
        let staleIds = catalogDescendantIds(of: parentId, in: retained)
        retained.removeAll { staleIds.contains($0.idBytes) }
        retained.append(contentsOf: loaded)
        catalogSnapshot = retained
      } else {
        catalogSnapshot = loaded
      }
      catalogRefreshState = .loaded
      catalogSummary = "catalog · \(catalogSnapshot?.count ?? 0) nodes loaded"
    } catch {
      let message = "Browse failed: \(error)"
      catalogRefreshState =
        hadSnapshot
        ? .stale(nodeKey: expandedNodeKey, message: message)
        : .failed(message: message)
      catalogError = message
    }
  }

  func runQuery() async {
    let tab = activeQueryTab
    let sql = tab.statementText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !sql.isEmpty else {
      tab.queryError = "Enter SQL before running."
      tab.selectedResultSection = "messages"
      return
    }
    tab.querySummary = nil
    tab.queryError = nil
    tab.resultTable = nil
    tab.selectedResultSection = "results"
    do {
      if connectedEngine != "redis" {
        let names = try await client?.inspectNamedParameters(statement: sql) ?? []
        if !names.isEmpty {
          parameterizedStatement = sql
          queryParameterBindings = names.map { WorkbenchQueryParameter(name: $0) }
          queryParameterError = nil
          queryParametersPresented = true
          return
        }
      }
      if let table = try await fetchPage(intent: "execute", statement: sql, tab: tab) {
        tab.resultTable = table
        tab.querySummary =
          "result · \(counted(table.columns.count, "column")) · \(counted(table.rows.count, "row"))"
      } else if let outcome = tab.writeOutcome {
        tab.querySummary = "write ok · \(outcome)"
      } else {
        tab.querySummary = "query: no result"
      }
    } catch {
      tab.queryError = "Query failed: \(error)"
      tab.selectedResultSection = "messages"
    }
  }

  func runParameterizedQuery() async {
    guard let statement = parameterizedStatement, !isRunning else { return }
    let tab = activeQueryTab
    queryParameterError = nil
    tab.selectedResultSection = "results"
    do {
      if let table = try await fetchPage(
        intent: "execute", statement: statement, tab: tab,
        bindings: queryParameterBindings)
      {
        tab.resultTable = table
        tab.querySummary =
          "result · \(counted(table.columns.count, "column")) · \(counted(table.rows.count, "row"))"
      } else if let outcome = tab.writeOutcome {
        tab.querySummary = "write ok · \(outcome)"
      } else {
        tab.querySummary = "query: no result"
      }
      queryParametersPresented = false
      parameterizedStatement = nil
      queryParameterBindings = []
    } catch {
      queryParameterError = "Parameterized query failed: \(error)"
      tab.selectedResultSection = "messages"
    }
  }

  func cancelQueryParameters() {
    queryParametersPresented = false
    parameterizedStatement = nil
    queryParameterBindings = []
    queryParameterError = nil
  }

  func runExplain() async {
    let tab = activeQueryTab
    let sql = tab.statementText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !sql.isEmpty else {
      tab.queryError = "EXPLAIN needs SQL in the active editor"
      tab.selectedResultSection = "messages"
      return
    }
    tab.querySummary = nil
    tab.queryError = nil
    tab.explainPlan = nil
    do {
      guard let table = try await fetchPage(intent: "explain", statement: sql, tab: tab),
        !table.rows.isEmpty
      else {
        tab.queryError = "EXPLAIN returned no plan"
        return
      }
      tab.resultTable = table
      tab.explainPlan = table.rows.compactMap(\.first).joined(separator: "\n")
      tab.querySummary = "explain · \(counted(table.rows.count, "line"))"
      tab.selectedResultSection = "plan"
    } catch {
      tab.queryError = "Explain failed: \(error)"
      tab.selectedResultSection = "messages"
    }
  }

  func copyStructureDdl(_ ddl: String) {
    do {
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: ddl)
      ])
      copyOutcome = "Copied structure DDL"
    } catch {
      copyError = "Copy DDL failed: \(error)"
    }
  }

  func copyExplainPlan() {
    guard let plan = activeQueryTab.explainPlan else { return }
    do {
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: plan)
      ])
      copyOutcome = "Copied explain plan"
    } catch {
      copyError = "Copy explain plan failed: \(error)"
    }
  }
}
