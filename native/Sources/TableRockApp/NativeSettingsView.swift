import Foundation
import SwiftUI
import TableRockFeature

struct NativeSettingsView: View {
  let application: NativeApplicationModel
  @State private var outcome: String?

  var body: some View {
    Form {
      LabeledContent("Storage", value: "Local only")
      LabeledContent("Telemetry", value: "Off by default")
      Section("Support") {
        Button("Export Safe Support Bundle…") { exportSupportBundle() }
          .accessibilityIdentifier("settings.support.export")
        Text("Contains version, platform, and closed redacted diagnostic facts only.")
          .font(.caption)
          .foregroundStyle(.secondary)
        if let outcome {
          Text(outcome)
            .font(.caption)
            .accessibilityIdentifier("settings.support.outcome")
            .accessibilityValue(outcome)
        }
      }
    }
    .formStyle(.grouped)
    .padding()
    .frame(width: 420)
  }

  private func exportSupportBundle() {
    guard let client = application.client else {
      outcome = "Support export unavailable"
      return
    }
    guard
      let url = application.dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Safe Support Bundle", prompt: "Export",
          suggestedFilename: "tablerock-support.txt", allowedExtensions: ["txt"]
        ))
    else { return }
    let destination =
      url.pathExtension.lowercased() == "txt" ? url : url.appendingPathExtension("txt")
    Task {
      let accessed = destination.startAccessingSecurityScopedResource()
      defer { if accessed { destination.stopAccessingSecurityScopedResource() } }
      do {
        let bytes = try await client.exportSupportBundle(path: destination.path)
        outcome = "Exported \(bytes) safe bytes to \(destination.lastPathComponent)"
      } catch {
        outcome = "Support export failed"
      }
    }
  }
}
