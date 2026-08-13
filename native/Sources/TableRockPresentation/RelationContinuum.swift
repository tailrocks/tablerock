import Foundation
import TableRockFeature

struct RelationContinuumState: Equatable {
  let edgeTitle: String
  let directionWord: String
  let fromColumn: String
  let fromValue: String
  let relatedSchema: String
  let relatedTable: String
  let relatedColumn: String
  let columns: [String]
  let rows: [[String]]
  let statusWord: String
}

@MainActor
extension WorkbenchPresentationStore {
  var canOpenRelationContinuum: Bool {
    guard connectedEngine == "postgresql",
      selectedWorkbenchKind == "object",
      activeObjectTab != nil,
      !relationContinuumLoading,
      let selection = selectedCellSnapshot
    else { return false }
    return !selection.1.isTruncated && ![0, 8, 10, 11].contains(selection.1.kind)
  }

  func openRelationContinuumFromSelection() async {
    relationContinuumError = nil
    relationContinuum = nil
    guard let client, let session = sessionData, let object = activeObjectTab else {
      relationContinuumError = "DISCONNECTED — connect before Continuum"
      return
    }
    guard let selection = selectedCellSnapshot else {
      relationContinuumError = "Select a cell that participates in a relation"
      return
    }
    guard !selection.1.isTruncated, ![0, 8, 10, 11].contains(selection.1.kind) else {
      relationContinuumError = "Continuum requires a complete comparable cell value"
      return
    }

    relationContinuumLoading = true
    defer { relationContinuumLoading = false }
    do {
      let submission = try await client.submitPostgresRelationBrowse(
        sessionId: session,
        catalogNodeId: object.catalogNodeId,
        selectedColumn: selection.0.name,
        cell: selection.1
      )
      let route: (schema: String, table: String, column: String)
      switch submission.direction {
      case "outbound":
        route = (
          submission.edge.toSchema, submission.edge.toTable, submission.edge.toColumn)
      case "inbound":
        route = (
          submission.edge.fromSchema, submission.edge.fromTable, submission.edge.fromColumn)
      default:
        relationContinuumError = "Continuum unavailable: invalid relation direction"
        return
      }
      let result = try await client.finish(operationId: submission.operationId)
      guard activeObjectTab?.id == object.id,
        let currentSelection = selectedCellSnapshot,
        currentSelection.0 == selection.0,
        currentSelection.1 == selection.1,
        currentSelection.2 == selection.2,
        currentSelection.3 == selection.3
      else { return }
      guard let table = result.table else {
        relationContinuumError = "Continuum unavailable: related browse returned no table"
        return
      }
      relationContinuum = RelationContinuumState(
        edgeTitle:
          "\(submission.edge.fromTable).\(submission.edge.fromColumn) → \(submission.edge.toTable).\(submission.edge.toColumn)",
        directionWord: submission.direction,
        fromColumn: selection.0.name,
        fromValue: selection.1.kind == 0 ? "NULL" : selection.1.display,
        relatedSchema: route.schema,
        relatedTable: route.table,
        relatedColumn: route.column,
        columns: table.columns,
        rows: table.rows,
        statusWord: table.rows.isEmpty ? "EMPTY" : "READY"
      )
      queryStateRevision &+= 1
    } catch {
      relationContinuumError = "Continuum unavailable: \(error)"
    }
  }

  func closeRelationContinuum() {
    relationContinuum = nil
    relationContinuumError = nil
    queryStateRevision &+= 1
  }

}
