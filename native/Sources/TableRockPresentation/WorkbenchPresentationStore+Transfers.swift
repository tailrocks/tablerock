import Foundation
import TableRockFeature

private enum TransferWorkflowError: Error {
  case unavailable(String)
}

@MainActor
extension WorkbenchPresentationStore {
  func copyResult(scope: String, preferredFormat: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "No resident result to copy"
      return
    }
    let selection = selectedCell
    if scope != "loaded", selection == nil {
      copyError = "Select a result cell first"
      return
    }
    copyOutcome = nil
    copyError = nil
    do {
      let row = selection.map { UInt64($0.row) }
      let column = selection.map { UInt32($0.column) }
      var payloads: [String: String] = [:]
      for format in ["csv", "tsv", "json", "markdown"] {
        payloads[format] = try await client.formatResultCopy(
          resultId: resultId, revision: resultRevision, scope: scope,
          row: row, column: column, format: format
        )
      }
      if preferredFormat == "sql_insert" {
        payloads[preferredFormat] = try await client.formatResultCopy(
          resultId: resultId, revision: resultRevision, scope: scope,
          row: row, column: column, format: preferredFormat
        )
      }
      let preferred = payloads[preferredFormat] ?? payloads["tsv"] ?? ""
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: preferred),
        AppPasteboardRepresentation(
          type: "public.comma-separated-values-text", value: payloads["csv"] ?? ""
        ),
        AppPasteboardRepresentation(
          type: "public.utf8-tab-separated-values-text", value: payloads["tsv"] ?? ""),
        AppPasteboardRepresentation(type: "public.json", value: payloads["json"] ?? ""),
        AppPasteboardRepresentation(
          type: "net.daringfireball.markdown", value: payloads["markdown"] ?? ""
        ),
      ])
      copyOutcome =
        "Copied \(scope) as \(preferredFormat.uppercased()) with CSV, TSV, JSON, and Markdown representations"
    } catch { copyError = "Copy failed: \(error)" }
  }

  func exportLoadedResult(format: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "No resident result to export"
      return
    }
    let fileExtension = format == "sql_insert" ? "sql" : format
    guard
      let selected = dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Loaded Result", prompt: "Export",
          suggestedFilename: "result.\(fileExtension)", allowedExtensions: [fileExtension]
        ))
    else { return }
    let url =
      selected.pathExtension.lowercased() == fileExtension
      ? selected : selected.appendingPathExtension(fileExtension)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    copyOutcome = nil
    copyError = nil
    do {
      let bytes = try await client.exportLoadedResult(
        resultId: resultId, revision: resultRevision, format: format, path: url.path
      )
      copyOutcome = "Exported \(bytes) bytes to \(url.lastPathComponent)"
    } catch { copyError = "Export failed: \(error)" }
  }

  func exportFullResult(format: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "Full-result export requires a resident result"
      return
    }
    let statement = queryText.trimmingCharacters(in: .whitespacesAndNewlines)
    if selectedWorkbenchKind == "query" && statement.isEmpty {
      copyError = "Query is empty"
      return
    }
    let fileExtension = format
    guard
      let selected = dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Full Result", prompt: "Export",
          suggestedFilename: "result.\(fileExtension)", allowedExtensions: [fileExtension]
        ))
    else { return }
    let url =
      selected.pathExtension.lowercased() == fileExtension
      ? selected : selected.appendingPathExtension(fileExtension)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    copyOutcome = nil
    copyError = nil
    streamExportError = nil
    streamExportProgress = nil
    streamExportPresented = true
    do {
      let operationId: Data
      if selectedWorkbenchKind == "object" {
        operationId = try await client.startCatalogStreamExport(
          resultId: resultId, revision: resultRevision, format: format, path: url.path)
      } else {
        guard let session = sessionData else {
          throw TransferWorkflowError.unavailable("stream-export-session")
        }
        operationId = try await client.startStreamExport(
          sessionId: session, statement: statement, format: format, path: url.path)
      }
      streamExportOperationId = operationId
      while streamExportOperationId == operationId {
        let progress = try await client.streamExportProgress(operationId: operationId)
        streamExportProgress = progress
        if !["running", "cancel_requested"].contains(progress.phase) {
          copyOutcome = progress.summary
          _ = try? await client.dismissStreamExport(operationId: operationId)
          streamExportOperationId = nil
          break
        }
        try await Task.sleep(for: .milliseconds(100))
      }
    } catch {
      streamExportOperationId = nil
      streamExportError = "Full-result export failed: \(error)"
    }
  }

  func pollStreamExport(
    client: any WorkbenchBackend, operationId: Data
  ) async throws -> WorkbenchStreamExportProgress {
    while true {
      let progress = try await client.streamExportProgress(operationId: operationId)
      if !["running", "cancel_requested"].contains(progress.phase) { return progress }
      try await Task.sleep(for: .milliseconds(50))
    }
  }

  func cancelStreamExport() async {
    guard let client, let operationId = streamExportOperationId else { return }
    do {
      if try await client.cancelStreamExport(operationId: operationId) {
        streamExportProgress = try await client.streamExportProgress(operationId: operationId)
      }
    } catch { streamExportError = "Cancel export failed: \(error)" }
  }

  func closeStreamExport() {
    guard streamExportOperationId == nil else { return }
    streamExportPresented = false
    streamExportProgress = nil
    streamExportError = nil
  }

  func chooseCsvImport() async {
    guard let client, sqlInsertCopyAvailable else { return }
    guard
      let url = dependencies.filePanels.chooseOpenFile(
        AppFilePanelRequest(
          title: "Import CSV into Table", prompt: "Preview", allowedExtensions: ["csv"]
        ))
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let preview = try await client.previewCsvImport(path: url.path)
      csvImportUrl = url
      csvImportPreview = preview
      csvImportMappedColumns = preview.headers
      csvImportColumnTypes = Array(repeating: "text", count: preview.headers.count)
      csvImportReview = nil
      csvImportError = nil
      csvImportOutcome = nil
      csvImportProgress = nil
      csvImportErrorCopyOutcome = nil
      csvImportPresented = true
    } catch { csvImportError = "CSV preview failed: \(error)" }
  }

  func stageCsvImport() async {
    guard let client, let session = sessionData, let object = activeObjectTab,
      let url = csvImportUrl
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    csvImportError = nil
    do {
      csvImportReview = try await client.stageCsvImport(
        sessionId: session, catalogNodeId: object.catalogNodeId, path: url.path,
        mappedColumns: csvImportMappedColumns,
        mappedTypes: csvImportColumnTypes,
        expectedFingerprint: csvImportPreview?.fingerprint ?? "",
        nowMs: dependencies.clock.nowMilliseconds()
      )
    } catch { csvImportError = "Stage import failed: \(error)" }
  }

  func applyCsvImport() async {
    guard let client, let session = sessionData, let review = csvImportReview else { return }
    csvImportApplying = true
    csvImportError = nil
    defer { csvImportApplying = false }
    do {
      let operationId = try await client.startCsvImportApply(
        tokenId: review.tokenId,
        nowMs: dependencies.clock.nowMilliseconds(),
        sessionId: session
      )
      csvImportReview = nil
      csvImportOperationId = operationId
      while csvImportOperationId == operationId {
        let progress = try await client.csvImportProgress(operationId: operationId)
        csvImportProgress = progress
        if !["running", "cancel_requested"].contains(progress.phase) {
          csvImportOutcome = progress.summary
          _ = try? await client.dismissCsvImport(operationId: operationId)
          csvImportOperationId = nil
          if progress.phase == "completed" { await reloadObjectTab() }
          break
        }
        try await Task.sleep(for: .milliseconds(100))
      }
    } catch {
      csvImportReview = nil
      csvImportOperationId = nil
      csvImportError = "Import progress failed after authority was consumed: \(error)"
    }
  }

  func cancelCsvImport() async {
    guard let client, let operationId = csvImportOperationId else { return }
    do {
      if try await client.cancelCsvImport(operationId: operationId) {
        csvImportProgress = try await client.csvImportProgress(operationId: operationId)
      }
    } catch { csvImportError = "Cancel import failed: \(error)" }
  }

  func copyCsvImportErrors() {
    guard let progress = csvImportProgress, !progress.errors.isEmpty else { return }
    var text = progress.errors.joined(separator: "\n")
    if progress.errorsTruncated { text += "\n… additional errors omitted" }
    do {
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: text)
      ])
      csvImportErrorCopyOutcome = "Copied \(progress.errors.count) import errors"
    } catch { csvImportErrorCopyOutcome = "Copy errors failed: \(error)" }
  }

  func discardCsvImportReview() async {
    if let review = csvImportReview, let client {
      _ = try? await client.revokeReviewToken(tokenId: review.tokenId)
    }
    csvImportReview = nil
  }

  func closeCsvImport() async {
    await discardCsvImportReview()
    csvImportPresented = false
    csvImportPreview = nil
    csvImportMappedColumns = []
    csvImportColumnTypes = []
    csvImportUrl = nil
    csvImportProgress = nil
    csvImportErrorCopyOutcome = nil
  }
}
