import Foundation

@MainActor
extension WorkbenchPresentationStore {
  func showDdlChange() {
    guard canEditSelectedStructure else { return }
    ddlChangeKind = "add_column"
    ddlChangeObjectName = ""
    ddlChangeDefinition = ""
    ddlChangeReview = nil
    ddlChangeOutcome = nil
    ddlChangeError = nil
    ddlChangeCatalogNodeId = activeObjectTab?.catalogNodeId
    ddlChangePresented = true
  }

  func stageDdlChange() async {
    guard let client, let session = sessionData, let nodeId = ddlChangeCatalogNodeId,
      ddlChangeReview == nil, !ddlChangeApplying
    else { return }
    ddlChangeError = nil
    ddlChangeOutcome = nil
    do {
      ddlChangeReview = try await client.stageDdlChange(
        sessionId: session, catalogNodeId: nodeId, kind: ddlChangeKind,
        objectName: ddlChangeObjectName.trimmingCharacters(in: .whitespacesAndNewlines),
        definition: ddlChangeDefinition.trimmingCharacters(in: .whitespacesAndNewlines),
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      ddlChangeReview = nil
      ddlChangeError = "Structure review rejected: \(error)"
    }
  }

  func applyDdlChange() async {
    guard let client, let session = sessionData, let review = ddlChangeReview else { return }
    let nodeId = ddlChangeCatalogNodeId
    ddlChangeReview = nil
    ddlChangeApplying = true
    ddlChangeError = nil
    defer { ddlChangeApplying = false }
    do {
      ddlChangeOutcome = try await client.applyDdlChange(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmed: true)
      if let nodeId,
        let tab = objectTabs.first(where: { $0.catalogNodeId == nodeId })
      {
        tab.structure = try await client.relationStructure(
          sessionId: session, catalogNodeId: nodeId)
        tab.structureError = nil
      }
    } catch {
      ddlChangeError = "Structure outcome unknown or failed; review consumed: \(error)"
    }
  }

  func discardDdlChangeReview() async {
    if let review = ddlChangeReview, let client {
      _ = try? await client.revokeDdlChange(tokenId: review.tokenId)
    }
    ddlChangeReview = nil
  }

  func closeDdlChange() async {
    await discardDdlChangeReview()
    ddlChangePresented = false
  }

  func showTableOperation() {
    guard canOperateSelectedTable else { return }
    tableOperationKind = connectedEngine == "clickhouse" ? "optimize" : "truncate"
    tableOperationNewName = ""
    tableOperationConfirmation = ""
    tableOperationReview = nil
    tableOperationStatus = nil
    tableOperationOutcome = nil
    tableOperationError = nil
    tableOperationCatalogNodeId = activeObjectTab?.catalogNodeId
    tableOperationPresented = true
  }

  func resetTableOperationReview() async {
    guard !tableOperationApplying else { return }
    if let review = tableOperationReview, let client {
      _ = try? await client.revokeTableOperation(tokenId: review.tokenId)
    }
    if let operationId = tableOperationId, let client {
      _ = try? await client.dismissTableOperation(operationId: operationId)
    }
    tableOperationReview = nil
    tableOperationStatus = nil
    tableOperationId = nil
    tableOperationConfirmation = ""
    tableOperationOutcome = nil
    tableOperationError = nil
  }

  func stageTableOperation() async {
    guard let client, let session = sessionData, let nodeId = tableOperationCatalogNodeId,
      tableOperationReview == nil, !tableOperationApplying
    else { return }
    tableOperationError = nil
    tableOperationOutcome = nil
    do {
      tableOperationReview = try await client.stageTableOperation(
        sessionId: session, catalogNodeId: nodeId, kind: tableOperationKind,
        newName: tableOperationNewName.trimmingCharacters(in: .whitespacesAndNewlines),
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      tableOperationError = "Table operation review rejected: \(error)"
    }
  }

  func applyTableOperation() async {
    guard let client, let session = sessionData, let review = tableOperationReview else { return }
    guard tableOperationConfirmation == review.confirmation else {
      tableOperationError = "Type the exact target table name to authorize this operation."
      return
    }
    let kind = tableOperationKind
    let nodeId = tableOperationCatalogNodeId
    tableOperationReview = nil
    tableOperationApplying = true
    tableOperationStatus = nil
    tableOperationError = nil
    defer { tableOperationApplying = false }
    do {
      let operationId = try await client.startTableOperation(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmation: tableOperationConfirmation)
      tableOperationId = operationId
      while true {
        let status = try await client.tableOperationStatus(operationId: operationId)
        tableOperationStatus = status
        if status.phase != "running" { break }
        try await Task.sleep(for: .milliseconds(100))
      }
      guard let status = tableOperationStatus else { return }
      if status.phase == "succeeded" {
        tableOperationOutcome = status.summary
      } else {
        tableOperationError = "Table operation \(status.phase): \(status.summary)"
        return
      }
      if ["rename", "drop"].contains(kind), let nodeId {
        objectTabs.removeAll(where: { $0.catalogNodeId == nodeId })
        selectedObjectTabId = nil
        selectedWorkbenchKind = "query"
        await browse()
      } else if kind == "truncate", let nodeId,
        let tab = objectTabs.first(where: { $0.catalogNodeId == nodeId })
      {
        await loadObjectTab(tab)
      }
    } catch {
      tableOperationError = "Table operation failed or outcome unknown; review consumed: \(error)"
    }
  }

  func closeTableOperation() async {
    await resetTableOperationReview()
    tableOperationPresented = false
  }
}
