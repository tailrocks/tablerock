import Foundation
import TableRockBridge
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  private var hasUnsavedEditorText: Bool { queryText != sqlFileBaseline }

  func requestOpenSqlFile() {
    if hasUnsavedEditorText {
      confirmDiscardForOpen = true
    } else {
      Task { await openSqlFile() }
    }
  }

  func openSqlFile() async {
    confirmDiscardForOpen = false
    guard
      let url = dependencies.filePanels.chooseOpenFile(
        AppFilePanelRequest(
          title: "Open SQL File", prompt: "Open", allowedExtensions: ["sql"]
        )), let client
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let file = try await client.readSqlFile(path: url.path)
      sqlFile = file
      sqlFileBaseline = file.statementText
      queryText = file.statementText
      sqlFileError = nil
      profileActionOutcome = "Opened \(url.lastPathComponent)"
    } catch { sqlFileError = "Open SQL file failed: \(error)" }
  }

  func saveSqlFile(saveAs: Bool = false, overwriteExternalChange: Bool = false) async {
    guard let client else { return }
    var url = sqlFile.map { URL(fileURLWithPath: $0.path) }
    if saveAs || url == nil {
      guard
        let selected = dependencies.filePanels.chooseSaveFile(
          AppFilePanelRequest(
            title: "Save SQL File", prompt: "Save", suggestedFilename: "query.sql",
            allowedExtensions: ["sql"]
          ))
      else { return }
      url =
        selected.pathExtension == "sql"
        ? selected : selected.appendingPathExtension("sql")
    }
    guard let url else { return }
    let sameFile = !saveAs && sqlFile?.path == url.path
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let written = try await client.writeSqlFile(
        path: url.path,
        statement: queryText,
        expectedModifiedNanos: sameFile ? sqlFile?.modifiedNanos : nil,
        expectedLength: sameFile ? sqlFile?.len : nil,
        overwriteExternalChange: overwriteExternalChange
      )
      sqlFile = written
      sqlFileBaseline = queryText
      sqlFileError = nil
      confirmExternalOverwrite = false
      profileActionOutcome = "Saved \(url.lastPathComponent)"
    } catch let error as BridgeError {
      if case .Rejected(code: "sql-file-external-change", message: _) = error {
        confirmExternalOverwrite = true
      } else {
        sqlFileError = "Save SQL file failed: \(error)"
      }
    } catch { sqlFileError = "Save SQL file failed: \(error)" }
  }

  func reloadSqlFile() async {
    guard let file = sqlFile, let client else { return }
    let url = URL(fileURLWithPath: file.path)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let loaded = try await client.readSqlFile(path: file.path)
      sqlFile = loaded
      sqlFileBaseline = loaded.statementText
      queryText = loaded.statementText
      sqlFileError = nil
      confirmExternalOverwrite = false
      profileActionOutcome = "Reloaded \(url.lastPathComponent)"
    } catch { sqlFileError = "Reload SQL file failed: \(error)" }
  }
}
