import Foundation
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  var canEditSelectedRow: Bool {
    guard selectedWorkbenchKind == "object", connectedEngine == "postgresql",
      activeProfile?.safetyMode != "read_only",
      let tab = activeObjectTab, tab.mutationEditability?.editable == true,
      let selection = tab.selectedCell,
      tab.resultTable?.rows.indices.contains(selection.row) == true,
      tab.mutationReview == nil, !tab.mutationApplying
    else { return false }
    return true
  }

  func refreshMutationEditability(for tab: NativeObjectTab) async {
    guard let client, let session = sessionData, let resultId = tab.resultIdData,
      connectedEngine == "postgresql"
    else {
      tab.mutationEditability = WorkbenchMutationEditability(
        editable: false,
        reason: connectedEngine == "postgresql" ? "no base table" : "engine edit flow unavailable",
        identityColumns: [])
      return
    }
    do {
      tab.mutationEditability = try await client.mutationEditability(
        sessionId: session, resultId: resultId)
    } catch {
      tab.mutationEditability = WorkbenchMutationEditability(
        editable: false, reason: "editability proof unavailable", identityColumns: [])
    }
  }

  func showSelectedRowEditor() {
    guard canEditSelectedRow, let tab = activeObjectTab, let table = tab.resultTable,
      let selection = tab.selectedCell
    else { return }
    let identityColumns = Set(tab.mutationEditability?.identityColumns ?? [])
    let fields = table.columns.indices.compactMap { column -> NativeMutationField? in
      guard !identityColumns.contains(table.columns[column]),
        table.cells.indices.contains(selection.row),
        table.cells[selection.row].indices.contains(column)
      else { return nil }
      let cell = table.cells[selection.row][column]
      guard !cell.isTruncated,
        let kind = nativeMutationKind(cell: cell)
      else { return nil }
      return NativeMutationField(
        column: table.columns[column], kind: kind,
        original: mutationEditorValue(cell), value: mutationEditorValue(cell))
    }
    guard !fields.isEmpty else {
      tab.mutationError = "The selected row has no writable values."
      return
    }
    tab.mutationError = nil
    tab.mutationOutcome = nil
    rowEditDraft = NativeRowEditDraft(row: selection.row, relation: tab.title, fields: fields)
    mutationReviewPresented = true
  }

  func stageRowUpdate() async {
    guard let client, let session = sessionData, let tab = activeObjectTab,
      let draft = rowEditDraft, let resultId = tab.resultIdData, tab.mutationReview == nil
    else { return }
    let changed = draft.fields.filter { $0.value != $0.original }
    guard !changed.isEmpty else {
      tab.mutationError = "Change at least one value before review."
      return
    }
    tab.mutationError = nil
    do {
      tab.mutationReview = try await client.stageRowUpdate(
        sessionId: session, resultId: resultId, revision: tab.resultRevision,
        row: UInt64(draft.row),
        assignments: changed.map {
          WorkbenchMutationAssignment(
            column: $0.column, kind: $0.kind, value: Data($0.value.utf8))
        },
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      tab.mutationError = "Row update review rejected: \(error)"
    }
  }

  func backToRowEditor() async {
    guard let tab = activeObjectTab, let review = tab.mutationReview else { return }
    if let client { _ = try? await client.revokeReviewToken(tokenId: review.tokenId) }
    tab.mutationReview = nil
    tab.mutationError = nil
  }

  func discardRowUpdate() async {
    if let tab = activeObjectTab, let review = tab.mutationReview, let client {
      _ = try? await client.revokeReviewToken(tokenId: review.tokenId)
      tab.mutationReview = nil
    }
    rowEditDraft = nil
    mutationReviewPresented = false
  }

  func applyRowUpdate() async {
    guard let client, let session = sessionData, let tab = activeObjectTab,
      let review = tab.mutationReview, !tab.mutationApplying
    else { return }
    tab.mutationReview = nil
    tab.mutationApplying = true
    tab.mutationError = nil
    defer { tab.mutationApplying = false }
    do {
      let outcome = try await client.applyReviewToken(
        tokenId: review.tokenId, nowMs: dependencies.clock.nowMilliseconds(), sessionId: session)
      if outcome.conflictCount > 0 || outcome.failedCount > 0 {
        tab.mutationError =
          "Update not applied · \(outcome.conflictCount) conflict · \(outcome.failedCount) failed. Review consumed; edit again."
        return
      }
      tab.mutationOutcome = "Applied \(outcome.appliedCount) update in one transaction."
      rowEditDraft = nil
      mutationReviewPresented = false
      await loadObjectTab(tab)
    } catch {
      tab.mutationError = "Update failed or outcome unknown; review consumed: \(error)"
    }
  }

  private func nativeMutationKind(cell: WorkbenchCell) -> String? {
    switch cell.kind {
    case 1: "boolean"
    case 2: "signed"
    case 3: "unsigned"
    case 4: "float"
    case 5: "decimal"
    case 6: "temporal"
    case 7: "text"
    case 9: nil
    case 0: nil
    default: nil
    }
  }

  private func mutationEditorValue(_ cell: WorkbenchCell) -> String {
    if cell.kind == 0 { return "" }
    if cell.kind == 1 { return cell.bytes.first == 1 ? "true" : "false" }
    if [2, 3, 4].contains(cell.kind), cell.bytes.count == 8 {
      let bits = cell.bytes.reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
      if cell.kind == 2 { return String(Int64(bitPattern: bits)) }
      if cell.kind == 3 { return String(bits) }
      return String(Double(bitPattern: bits))
    }
    return cell.display
  }
}
