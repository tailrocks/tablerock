import Foundation
import SwiftUI
import TableRockFeature

struct StreamExportSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        VStack(alignment: .leading, spacing: 3) {
          Text("Export Full Result").font(.title2).bold()
          Text(
            "Rust replays the exact query or typed object browse in bounded pages and publishes atomically."
          )
          .foregroundStyle(.secondary)
        }
        Spacer()
        Button("Close") { model.closeStreamExport() }
          .disabled(
            model.streamExportProgress.map {
              ["running", "cancel_requested"].contains($0.phase)
            } ?? true
          )
          .accessibilityIdentifier("export.stream.close")
      }
      if let progress = model.streamExportProgress {
        ProgressView(value: progress.phase == "completed" ? 1 : nil) {
          Text("\(progress.completedRows) rows · \(progress.bytesWritten) bytes")
        }
        .accessibilityIdentifier("export.stream.progress")
        .accessibilityValue(
          "\(progress.phase), \(progress.completedRows) rows, \(progress.bytesWritten) bytes")
        Text(progress.summary)
          .textSelection(.enabled)
          .accessibilityIdentifier("export.stream.outcome")
        Text(URL(fileURLWithPath: progress.destination).lastPathComponent)
          .font(.caption).foregroundStyle(.secondary)
        if ["running", "cancel_requested"].contains(progress.phase) {
          Button("Cancel Export", role: .destructive) {
            Task { await model.cancelStreamExport() }
          }
          .disabled(progress.phase == "cancel_requested")
          .accessibilityIdentifier("export.stream.cancel")
        }
      } else {
        ProgressView("Starting full-result export…")
          .accessibilityIdentifier("export.stream.starting")
      }
      if let error = model.streamExportError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("export.stream.error")
      }
    }
    .padding(20)
    .frame(minWidth: 520, idealHeight: 260)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("export.stream.sheet")
    .interactiveDismissDisabled(
      model.streamExportProgress.map {
        ["running", "cancel_requested"].contains($0.phase)
      } ?? true)
  }
}

struct CsvImportSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Import CSV", systemImage: "tablecells.badge.ellipsis")
          .font(.title2.bold())
        Spacer()
        Button("Close") { Task { await model.closeCsvImport() } }
          .disabled(model.csvImportApplying)
      }
      HStack {
        Button("Stage Reviewed Import") { Task { await model.stageCsvImport() } }
          .buttonStyle(.glassProminent)
          .disabled(
            model.csvImportPreview == nil || model.csvImportReview != nil
              || model.csvImportOutcome != nil || model.csvImportApplying
          )
          .accessibilityIdentifier("import.csv.stage")
        Button("Apply Import") { Task { await model.applyCsvImport() } }
          .buttonStyle(.glassProminent)
          .disabled(model.csvImportReview == nil || model.csvImportApplying)
          .accessibilityIdentifier("import.csv.apply")
        Button("Discard Review", role: .cancel) {
          Task { await model.discardCsvImportReview() }
        }
        .disabled(model.csvImportReview == nil || model.csvImportApplying)
        .accessibilityIdentifier("import.csv.discard")
        Spacer()
      }
      .fixedSize(horizontal: false, vertical: true)
      ScrollView {
        VStack(alignment: .leading, spacing: 14) {
          if let preview = model.csvImportPreview {
            Text(
              "\(URL(fileURLWithPath: preview.path).lastPathComponent) · \(preview.totalRows) rows · \(preview.headers.count) columns"
            )
            .foregroundStyle(.secondary)
            if preview.formulaLikeCells > 0 {
              Label(
                "\(preview.formulaLikeCells) formula-like cells will be inserted as literal text",
                systemImage: "exclamationmark.triangle.fill"
              )
              .foregroundStyle(.orange)
            }
            GroupBox("Column mapping") {
              Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 6) {
                ForEach(preview.headers.indices, id: \.self) { index in
                  GridRow {
                    Text(preview.headers[index]).textSelection(.enabled)
                    Image(systemName: "arrow.right")
                      .foregroundStyle(.secondary)
                    TextField(
                      "Target column",
                      text: $model.csvImportMappedColumns[index]
                    )
                    .disabled(model.csvImportReview != nil)
                    Picker(
                      "Value type",
                      selection: $model.csvImportColumnTypes[index]
                    ) {
                      Text("Text").tag("text")
                      Text("Integer").tag("signed")
                      Text("Float").tag("float64")
                      Text("Boolean").tag("boolean")
                    }
                    .labelsHidden()
                    .disabled(model.csvImportReview != nil)
                  }
                }
              }
              .padding(6)
            }
            GroupBox("Preview — first \(preview.rows.count) rows") {
              ScrollView([.horizontal, .vertical]) {
                Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 5) {
                  GridRow {
                    ForEach(preview.headers, id: \.self) { header in
                      Text(header).bold()
                    }
                  }
                  Divider()
                  ForEach(preview.rows.indices, id: \.self) { rowIndex in
                    GridRow {
                      ForEach(preview.rows[rowIndex].cells.indices, id: \.self) { column in
                        Text(preview.rows[rowIndex].cells[column])
                          .lineLimit(1)
                          .textSelection(.enabled)
                      }
                    }
                  }
                }
                .padding(6)
              }
              .frame(minHeight: 150, maxHeight: 260)
            }
          }
          if let review = model.csvImportReview {
            ChangeReviewPlane(
              kindWord: "INSERT",
              title: "LEDGER · frozen CSV import",
              preview:
                "INSERT \(review.rowCount) rows · \(review.columnCount) mapped columns → \(review.target)",
              metadataFact: ChangeReviewPresentation.metadataStrip(
                target: review.target,
                expiresAtMs: review.expiresAtMs,
                nowMs: model.nowMilliseconds(),
                destructive: false,
                extra: review.formulaLikeCells > 0
                  ? "\(review.formulaLikeCells) formula-like cells as literals" : nil),
              destructive: false,
              production: model.activeProductionWarning,
              rollbackSummary:
                "Plan frozen 60s. Authority is consumed before database I/O and cannot be retried after failure.",
              safetyNote: review.formulaLikeCells > 0
                ? "Formula-like cells insert as literal text (never formulas)." : nil,
              previewAccessibilityId: "import.csv.review.preview"
            )
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }
      if let progress = model.csvImportProgress {
        VStack(alignment: .leading, spacing: 6) {
          ProgressView(
            value: Double(progress.completedRows),
            total: Double(max(progress.totalRows, 1))
          ) {
            Text("\(progress.completedRows) of \(progress.totalRows) rows")
          }
          .accessibilityIdentifier("import.csv.progress")
          .accessibilityValue(
            "\(progress.phase), \(progress.completedRows) of \(progress.totalRows) rows")
          HStack {
            Text(progress.phase.replacingOccurrences(of: "_", with: " ").capitalized)
              .foregroundStyle(.secondary)
            Spacer()
            if ["running", "cancel_requested"].contains(progress.phase) {
              Button("Cancel Import", role: .destructive) {
                Task { await model.cancelCsvImport() }
              }
              .disabled(progress.phase == "cancel_requested")
              .accessibilityIdentifier("import.csv.cancel")
            }
          }
          if !progress.errors.isEmpty {
            GroupBox("Import errors") {
              VStack(alignment: .leading, spacing: 5) {
                ForEach(progress.errors.indices, id: \.self) { index in
                  Text(progress.errors[index]).textSelection(.enabled)
                }
                if progress.errorsTruncated { Text("Additional errors omitted").italic() }
                Button("Copy Errors") { model.copyCsvImportErrors() }
                  .accessibilityIdentifier("import.csv.copy-errors")
                if let copied = model.csvImportErrorCopyOutcome {
                  Text(copied).foregroundStyle(.secondary)
                }
              }
              .padding(6)
            }
            .accessibilityIdentifier("import.csv.errors")
          }
        }
      } else if model.csvImportApplying {
        ProgressView("Starting reviewed import…")
      }
      if let outcome = model.csvImportOutcome {
        Label(
          outcome,
          systemImage: model.csvImportProgress?.phase == "completed"
            ? "checkmark.circle.fill" : "exclamationmark.circle.fill"
        )
        .foregroundStyle(model.csvImportProgress?.phase == "completed" ? .green : .orange)
        .accessibilityIdentifier("import.csv.outcome")
        .accessibilityValue(outcome)
      }
      if let error = model.csvImportError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .padding(20)
    .frame(minWidth: 720, idealHeight: 560)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(model.csvImportReview != nil || model.csvImportApplying)
  }
}
