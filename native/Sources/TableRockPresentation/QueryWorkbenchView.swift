import Foundation
import SwiftUI
import TableRockFeature

struct QueryWorkbenchView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    @Bindable var tab = model.activeQueryTabForPresentation
    let queryStatus = tab.queryError ?? tab.cancelOutcome ?? tab.querySummary ?? "Idle"
    let caretChip = SqlEditorMetrics.statusChip(
      text: model.queryText,
      selection: model.queryEditorSelection,
      isRunning: model.isRunning,
      hasError: tab.queryError != nil
    )
    VStack(alignment: .leading, spacing: 0) {
      HStack(spacing: 8) {
        Text("SQL")
          .font(.subheadline.weight(.semibold))
        if let file = model.sqlFile {
          Text(URL(fileURLWithPath: file.path).lastPathComponent)
            .font(.caption.monospaced())
            .foregroundStyle(.secondary)
            .lineLimit(1)
        }
        Spacer(minLength: 0)
        Text(caretChip)
          .font(.caption2.monospaced())
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("query.editor.metrics")
          .accessibilityValue(caretChip)
        if model.activeProductionWarning {
          Text("HALO PRODUCTION")
            .font(.caption2.weight(.bold))
            .accessibilityLabel("Production — writes need review")
        }
      }
      .padding(.bottom, 4)

      VSplitView {
        SqlTextEditor(
          text: $model.queryText,
          selection: $model.queryEditorSelection,
          isRunning: model.isRunning
        )
        .frame(minHeight: 96)
        .task(id: model.queryText) {
          try? await Task.sleep(for: .milliseconds(300))
          guard !Task.isCancelled else { return }
          await model.persistSessionIntent()
        }

        VStack(alignment: .leading, spacing: 4) {
          GlassEffectContainer {
            HStack(spacing: 8) {
              Button("Run") { Task { await model.runQuery() } }
                .accessibilityIdentifier("query.run")
                .buttonStyle(.glassProminent)
                .keyboardShortcut("r", modifiers: .command)
                .disabled(model.isRunning || model.isCatalogRefreshing)
                .help("Run selection or whole buffer")
              Button("Cancel") { Task { await model.cancel() } }
                .buttonStyle(.glass)
                .accessibilityIdentifier("query.cancel")
                .disabled(!model.isRunning)
              Button("Find…") { model.findReplacePresented = true }
                .buttonStyle(.glass)
                .keyboardShortcut("f", modifiers: .command)
                .accessibilityIdentifier("query.find")
              if model.connectedEngine == "redis" {
                Button("Redis Overview") { Task { await model.showRedisOverview() } }
                  .buttonStyle(.glass)
                  .disabled(model.redisOverviewLoading)
              }
              Button("Review probe…") { Task { await model.stageProbeChangeReview() } }
                .buttonStyle(.glass)
                .disabled(
                  model.isRunning || model.isCatalogRefreshing || model.probeChangeApplying
                    || model.probeChangeReview != nil
                )
                .accessibilityIdentifier("query.review-probe")
                .help("Stage edit-safety probe and open Change Review")
              Spacer(minLength: 0)
              if model.changeReviewOpen {
                Text("LEDGER")
                  .font(.caption2.weight(.bold).monospaced())
                  .accessibilityIdentifier("query.ledger.chip")
              }
              Text(queryStatus)
                .font(.caption.monospacedDigit())
                .foregroundStyle(model.queryError == nil ? Color.secondary : Color.primary)
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .accessibilityIdentifier("query.status")
                .accessibilityValue(queryStatus)
            }
            .controlSize(.small)
          }
          if let value = model.reviewOutcome {
            Text(value)
              .foregroundStyle(.secondary)
              .font(.caption2.monospaced())
              .accessibilityIdentifier("query.review.outcome")
          }
          if let value = model.reviewError {
            Text(value)
              .font(.caption2.monospaced())
              .textSelection(.enabled)
              .accessibilityIdentifier("query.review.error")
          }
          if let value = model.sqlFileError {
            Text(value).foregroundStyle(.red).font(.caption2).textSelection(.enabled)
          }
          if let table = model.resultTable {
            ResultGridWithInspector(
              table: table, minimumHeight: 140, exposesResultPaging: true
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
          } else {
            ContentUnavailableView(
              "No result yet",
              systemImage: "tablecells",
              description: Text("Run a query to fill the workbench grid.")
            )
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .accessibilityIdentifier("workbench.query.empty-result")
          }
        }
        .frame(minHeight: 160)
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    .accessibilityIdentifier("query.workbench")
  }
}
