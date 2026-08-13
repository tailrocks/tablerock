import Foundation
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  func addQueryTab() {
    guard queryTabs.count + objectTabs.count < 64 else {
      profileActionError = "At most 64 workbench tabs are allowed"
      return
    }
    let tab = NativeQueryTab(
      id: dependencies.identifiers.next(),
      title: "Query \(queryTabs.count + 1)",
      statementText: ""
    )
    queryTabs.append(tab)
    workspaceTabOrder.append(.query(tab.id))
    selectedQueryTabId = tab.id
    selectedWorkbenchKind = "query"
    Task { await persistSessionIntent() }
  }

  func selectQueryTab(_ tab: NativeQueryTab) {
    guard activeObjectTab?.mutationReview == nil, rowEditDraft == nil else {
      mutationReviewPresented = true
      profileActionError = "Apply or discard the staged row update before switching tabs."
      return
    }
    if selectedWorkbenchKind == "object" { activeObjectTab?.pinned = true }
    selectedQueryTabId = tab.id
    selectedWorkbenchKind = "query"
    Task { await persistSessionIntent() }
  }

  func requestCloseQueryTab(_ tab: NativeQueryTab) {
    guard queryTabs.count > 1 else {
      profileActionError = "At least one query tab must remain open"
      return
    }
    guard !tab.isRunning else {
      profileActionError = "Cancel the running query before closing its tab"
      return
    }
    if tab.statementText != tab.sqlFileBaseline {
      pendingQueryTabClose = tab
    } else {
      closeQueryTab(tab)
    }
  }

  func closePendingQueryTab() {
    guard let tab = pendingQueryTabClose else { return }
    pendingQueryTabClose = nil
    closeQueryTab(tab)
  }

  private func closeQueryTab(_ tab: NativeQueryTab) {
    guard let index = queryTabs.firstIndex(where: { $0.id == tab.id }), queryTabs.count > 1 else {
      return
    }
    queryTabs.remove(at: index)
    workspaceTabOrder.removeAll(where: { $0 == .query(tab.id) })
    if selectedQueryTabId == tab.id {
      selectedQueryTabId = queryTabs[min(index, queryTabs.count - 1)].id
    }
    Task { await persistSessionIntent() }
  }

  func beginRenameQueryTab(_ tab: NativeQueryTab) {
    queryTabRename = tab
    queryTabRenameText = tab.title
  }

  func renameQueryTab() {
    let title = queryTabRenameText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard let tab = queryTabRename, !title.isEmpty, title.utf8.count <= 256 else {
      profileActionError = "Tab title must be 1 to 256 bytes"
      return
    }
    tab.title = title
    queryTabRename = nil
    queryTabRenameText = ""
    Task { await persistSessionIntent() }
  }

  func openCatalogObject(nodeKey: String) async {
    guard let node = catalogSnapshot?.first(where: { catalogNodeKey($0.idBytes) == nodeKey })
    else { return }
    let browsableKinds: Set<String> = [
      "postgresql_table", "postgresql_view", "postgresql_materialized_view",
      "postgresql_foreign_table", "postgresql_partitioned_table", "postgresql_sequence",
      "clickhouse_table", "clickhouse_view", "clickhouse_materialized_view",
      "clickhouse_dictionary",
      "sqlite_table",
      "redis_key_unknown", "redis_key_string", "redis_key_hash",
      "redis_key_list", "redis_key_set", "redis_key_sorted_set",
      "redis_key_stream",
    ]
    guard browsableKinds.contains(node.kind) else {
      profileActionError = "\(node.name) is not a browsable table-like object"
      return
    }
    guard queryTabs.count + objectTabs.count < 64 else {
      profileActionError = "At most 64 workbench tabs are allowed"
      return
    }
    objectTabs.last(where: { !$0.pinned })?.pinned = true
    let tab = NativeObjectTab(id: dependencies.identifiers.next(), node: node)
    objectTabs.append(tab)
    workspaceTabOrder.append(.object(tab.id))
    selectedObjectTabId = tab.id
    selectedWorkbenchKind = "object"
    await loadObjectTab(tab)
    await loadObjectFilterPresets(tab)
  }

  func openCatalogObject(nodeId: Data) async {
    await openCatalogObject(nodeKey: catalogNodeKey(nodeId))
  }

  func selectObjectTab(_ tab: NativeObjectTab) {
    if selectedObjectTabId != tab.id,
      activeObjectTab?.mutationReview != nil || rowEditDraft != nil
    {
      mutationReviewPresented = true
      profileActionError = "Apply or discard the staged row update before switching tabs."
      return
    }
    if selectedWorkbenchKind == "object", selectedObjectTabId != tab.id {
      activeObjectTab?.pinned = true
    }
    selectedObjectTabId = tab.id
    selectedWorkbenchKind = "object"
    Task { await loadObjectFilterPresets(tab) }
  }

  func pinObjectTab(_ tab: NativeObjectTab) {
    tab.pinned = true
  }

  func closeObjectTab(_ tab: NativeObjectTab) {
    guard !tab.isRunning else {
      profileActionError = "Cancel the running browse before closing its tab"
      return
    }
    guard tab.mutationReview == nil,
      !(selectedObjectTabId == tab.id && rowEditDraft != nil)
    else {
      selectedObjectTabId = tab.id
      selectedWorkbenchKind = "object"
      mutationReviewPresented = true
      profileActionError = "Apply or discard the staged row update before closing this tab."
      return
    }
    guard let index = objectTabs.firstIndex(where: { $0.id == tab.id }) else { return }
    objectTabs.remove(at: index)
    workspaceTabOrder.removeAll(where: { $0 == .object(tab.id) })
    if selectedObjectTabId == tab.id {
      if objectTabs.isEmpty {
        selectedObjectTabId = nil
        selectedWorkbenchKind = "query"
      } else {
        let next = objectTabs[min(index, objectTabs.count - 1)]
        selectedObjectTabId = next.id
        selectedWorkbenchKind = "object"
      }
    }
  }

  func loadObjectTab(_ tab: NativeObjectTab) async {
    guard let client, let session = sessionData else { return }
    tab.error = nil
    tab.summary = nil
    do {
      if tab.kind.hasPrefix("redis_key_") {
        tab.isRunning = true
        defer { tab.isRunning = false }
        let view = try await client.redisKeyView(
          sessionId: session, catalogNodeId: tab.catalogNodeId,
          collectionSkip: 0
        )
        tab.redisView = view
        tab.summary = "Redis \(view.kind) · \(view.lines.count) lines"
        return
      }
      let operation = try await client.submitCatalogBrowse(
        session: session, nodeId: tab.catalogNodeId, sort: tab.sort, filters: tab.filters,
        rawWhere: tab.rawWhere
      )
      tab.activeOperationId = operation
      tab.isRunning = true
      defer {
        tab.activeOperationId = nil
        tab.isRunning = false
      }
      let projection = try await client.finish(operationId: operation)
      tab.resultTable = projection.table
      if let envelope = projection.envelope {
        tab.resultIdData = envelope.resultId
        tab.resultRevision = envelope.revision
        tab.nextStartRow =
          envelope.rowCount == 500
          ? envelope.startRow + UInt64(envelope.rowCount) : nil
      }
      if let table = projection.table {
        tab.summary =
          "\(counted(table.rows.count, "row")) · \(counted(table.columns.count, "column"))"
      } else {
        tab.summary = "No rows"
      }
      await refreshMutationEditability(for: tab)
    } catch { tab.error = "Object browse failed: \(error)" }
  }

  func loadMoreRedisKey() async {
    guard let tab = activeObjectTab, let client, let session = sessionData,
      let skip = tab.redisView?.nextSkip, !tab.isRunning
    else { return }
    tab.isRunning = true
    defer { tab.isRunning = false }
    do {
      let next = try await client.redisKeyView(
        sessionId: session, catalogNodeId: tab.catalogNodeId,
        collectionSkip: skip
      )
      let existing = tab.redisView?.lines ?? []
      tab.redisView = WorkbenchRedisKeyView(
        kind: next.kind,
        lines: existing + Array(next.lines.dropFirst(min(2, next.lines.count))),
        nextSkip: next.nextSkip
      )
      tab.summary = "Redis \(next.kind) · \(tab.redisView?.lines.count ?? 0) lines"
    } catch { tab.error = "Redis key page failed: \(error)" }
  }

  func loadMoreObjectRows() async {
    guard let tab = activeObjectTab, let client, let resultId = tab.resultIdData,
      let start = tab.nextStartRow
    else { return }
    do {
      let (more, envelope) = try await client.fetchPage(
        resultId: resultId, startRow: start, revision: tab.resultRevision
      )
      if more.rows.isEmpty {
        tab.nextStartRow = nil
        return
      }
      if let table = tab.resultTable {
        guard let table = table.appending(more) else {
          tab.error = "Load more returned incompatible page metadata"
          return
        }
        tab.resultTable = table
        tab.summary =
          "\(counted(table.rows.count, "row")) · \(counted(table.columns.count, "column"))"
      }
      tab.nextStartRow =
        envelope.rowCount == 500
        ? envelope.startRow + UInt64(envelope.rowCount) : nil
    } catch { tab.error = "Load more failed: \(error)" }
  }

  func reloadObjectTab() async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    await loadObjectTab(tab)
  }

  func addObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.sort.count < 16,
      !tab.sort.contains(where: { $0.column == column })
    else { return }
    tab.sort.append(WorkbenchBrowseSort(column: column))
    await loadObjectTab(tab)
  }

  func toggleObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning,
      let index = tab.sort.firstIndex(where: { $0.column == column })
    else { return }
    let current = tab.sort[index]
    tab.sort[index] = WorkbenchBrowseSort(
      column: current.column, descending: !current.descending)
    await loadObjectTab(tab)
  }

  func removeObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.sort.removeAll(where: { $0.column == column })
    await loadObjectTab(tab)
  }

  func addObjectFilter() async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.filters.count < 32,
      !tab.filterColumn.isEmpty
    else { return }
    let value =
      ["is_null", "is_not_null"].contains(tab.filterOperator)
      ? nil : tab.filterValue
    tab.filters.append(
      WorkbenchBrowseFilter(
        id: dependencies.identifiers.next(), column: tab.filterColumn,
        operatorName: tab.filterOperator, value: value))
    tab.filterValue = ""
    await loadObjectTab(tab)
  }

  func removeObjectFilter(id: UUID) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.filters.removeAll(where: { $0.id == id })
    await loadObjectTab(tab)
  }

  func clearObjectFilters() async {
    guard let tab = activeObjectTab, !tab.isRunning, !tab.filters.isEmpty else { return }
    tab.filters.removeAll()
    await loadObjectTab(tab)
  }

  func applyObjectRawWhere() async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    let fragment = tab.rawWhereDraft.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !fragment.isEmpty, fragment.utf8.count <= 65_536 else { return }
    tab.rawWhere = fragment
    await loadObjectTab(tab)
  }

  func clearObjectRawWhere() async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.rawWhere != nil else { return }
    tab.rawWhere = nil
    tab.rawWhereDraft = ""
    await loadObjectTab(tab)
  }

  func loadObjectFilterPresets(_ tab: NativeObjectTab) async {
    guard let client, let session = sessionData, !tab.kind.hasPrefix("redis_key_") else { return }
    do {
      tab.filterPresets = try await client.listCatalogFilterPresets(
        session: session, nodeId: tab.catalogNodeId)
      tab.filterPresetError = nil
    } catch {
      tab.filterPresets = []
      tab.filterPresetError = "Could not load filter presets: \(error)"
    }
  }

  func saveObjectFilterPreset() async {
    guard let tab = activeObjectTab, let client, let session = sessionData, !tab.isRunning else {
      return
    }
    let name = tab.filterPresetName.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty, name.utf8.count <= 64 else { return }
    do {
      try await client.saveCatalogFilterPreset(
        session: session, nodeId: tab.catalogNodeId,
        preset: WorkbenchSavedFilterPreset(
          name: name, filters: tab.filters, rawWhere: tab.rawWhere))
      tab.filterPresetName = ""
      tab.filterPresetOutcome = "Saved filter preset \(name)"
      tab.filterPresetError = nil
      await loadObjectFilterPresets(tab)
    } catch {
      tab.filterPresetOutcome = nil
      tab.filterPresetError = "Could not save filter preset: \(error)"
    }
  }

  func applyObjectFilterPreset(_ preset: WorkbenchSavedFilterPreset) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.filters = preset.filters.map {
      WorkbenchBrowseFilter(
        id: dependencies.identifiers.next(), column: $0.column,
        operatorName: $0.operatorName, value: $0.value)
    }
    tab.rawWhere = preset.rawWhere
    tab.rawWhereDraft = preset.rawWhere ?? ""
    tab.filterPresetOutcome = "Loaded filter preset \(preset.name)"
    tab.filterPresetError = nil
    await loadObjectTab(tab)
  }

  func loadObjectStructure() async {
    guard let tab = activeObjectTab, let client, let session = sessionData,
      !tab.structureLoading
    else { return }
    tab.selectedSection = "structure"
    tab.structureLoading = true
    tab.structureError = nil
    defer { tab.structureLoading = false }
    do {
      tab.structure = try await client.relationStructure(
        sessionId: session, catalogNodeId: tab.catalogNodeId
      )
    } catch {
      tab.structure = nil
      tab.structureError = "Structure unavailable: \(error)"
    }
  }
}
