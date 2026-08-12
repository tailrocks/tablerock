import Foundation
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  func presentHistory() async {
    historyPresented = true
    await refreshHistory()
  }

  func refreshHistory() async {
    guard let client else { return }
    historyGeneration &+= 1
    let generation = historyGeneration
    historyLoading = true
    historyError = nil
    do {
      let search = historySearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.listHistory(search.isEmpty ? nil : search)
      guard generation == historyGeneration else { return }
      historyItems = loaded
    } catch {
      guard generation == historyGeneration else { return }
      historyError = "History failed: \(error)"
    }
    if generation == historyGeneration { historyLoading = false }
  }

  func setHistoryRetention(_ retention: String) async {
    guard let client else { return }
    do {
      try await client.setHistoryRetention(retention)
      historyRetention = retention
    } catch { historyError = "Retention change failed: \(error)" }
  }

  func restoreHistory(_ item: WorkbenchHistoryItem) {
    guard let statement = item.statementText else {
      historyError = "SQL text was not retained for this entry"
      return
    }
    queryText = statement
    historyPresented = false
    profileActionOutcome = "History restored to editor"
  }

  func presentSavedQueries() async {
    savedQueriesPresented = true
    await refreshSavedQueries()
  }

  func refreshSavedQueries() async {
    guard let client else { return }
    savedQueriesGeneration &+= 1
    let generation = savedQueriesGeneration
    savedQueriesLoading = true
    savedQueriesError = nil
    do {
      let search = savedQuerySearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.listSavedQueries(
        engine: savedQueryEngine.isEmpty ? nil : savedQueryEngine,
        search: search.isEmpty ? nil : search
      )
      guard generation == savedQueriesGeneration else { return }
      savedQueries = loaded
    } catch {
      guard generation == savedQueriesGeneration else { return }
      savedQueriesError = "Saved queries failed: \(error)"
    }
    if generation == savedQueriesGeneration { savedQueriesLoading = false }
  }

  func beginSaveCurrentQuery() {
    guard !queryText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      profileActionError = "Cannot save an empty query"
      return
    }
    savedQueryName = ""
    saveQueryDialog = true
  }

  func saveCurrentQuery() async {
    guard let client else { return }
    let name = savedQueryName.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty else {
      savedQueriesError = "Query name is required"
      return
    }
    do {
      let engine = connectedEngine.isEmpty ? formEngine : connectedEngine
      _ = try await client.saveQuery(name: name, engine: engine, statement: queryText)
      saveQueryDialog = false
      savedQueryName = ""
      profileActionOutcome = "Saved query: \(name)"
      await refreshSavedQueries()
    } catch { savedQueriesError = "Save query failed: \(error)" }
  }

  func restoreSavedQuery(_ item: WorkbenchSavedQueryItem) {
    queryText = item.statementText
    savedQueriesPresented = false
    profileActionOutcome = "Saved query restored to editor"
  }

  func removePendingSavedQuery() async {
    guard let client, let item = pendingSavedQueryRemoval else { return }
    pendingSavedQueryRemoval = nil
    do {
      _ = try await client.deleteSavedQuery(item.queryId)
      await refreshSavedQueries()
    } catch { savedQueriesError = "Delete query failed: \(error)" }
  }
}
