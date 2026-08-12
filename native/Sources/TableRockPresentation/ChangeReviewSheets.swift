import SwiftUI
import TableRockFeature

struct ChangeReviewPlane: View {
  let kindWord: String
  let title: String
  let preview: String
  let metadataFact: String
  let destructive: Bool
  let production: Bool
  let rollbackSummary: String?
  let safetyNote: String?
  let previewAccessibilityId: String

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(kindWord)
          .font(.caption.weight(.bold).monospaced())
          .tracking(0.4)
          .accessibilityIdentifier("change.review.kind")
        VStack(alignment: .leading, spacing: 2) {
          Text(title)
            .font(.caption.weight(.bold))
            .tracking(0.4)
          Text(metadataFact)
            .font(.caption2.monospaced())
            .foregroundStyle(.secondary)
            .textSelection(.enabled)
            .accessibilityIdentifier("change.review.metadata")
        }
        Spacer(minLength: 4)
        if production {
          Text("HALO PRODUCTION")
            .font(.caption2.weight(.bold))
            .accessibilityLabel("Production — writes need review")
        }
        if destructive {
          Text("DESTRUCTIVE")
            .font(.caption2.weight(.bold).monospaced())
            .accessibilityIdentifier("change.review.destructive")
        }
      }
      Text(preview)
        .font(.system(.body, design: .monospaced))
        .textSelection(.enabled)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(Color(nsColor: .textBackgroundColor))
        .accessibilityLabel(preview)
        .accessibilityIdentifier(previewAccessibilityId)
      if let safetyNote {
        Text(safetyNote)
          .font(.caption2.weight(.semibold))
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("change.review.safety-note")
      }
      if let rollbackSummary, !rollbackSummary.isEmpty {
        Text(rollbackSummary)
          .font(.caption)
          .foregroundStyle(.secondary)
          .textSelection(.enabled)
          .accessibilityIdentifier("change.review.rollback")
      }
    }
    .padding(10)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(Color(nsColor: .controlBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("change.review.plane")
  }
}

struct DdlChangeSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @State private var applyConfirmationPresented = false

  private var needsDefinition: Bool {
    ["add_column", "create_index", "add_constraint"].contains(model.ddlChangeKind)
  }

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Text("CHANGE REVIEW")
          .font(.caption.weight(.bold))
          .tracking(0.6)
        Text("STRUCTURE")
          .font(.caption2.weight(.bold).monospaced())
          .foregroundStyle(.secondary)
        Spacer()
        Button("Close") { Task { await model.closeDdlChange() } }
          .disabled(model.ddlChangeApplying)
      }
      Text("Structure change")
        .font(.title3.weight(.semibold))
      Form {
        Picker("Operation", selection: $model.ddlChangeKind) {
          Text("Add column").tag("add_column")
          Text("Drop column").tag("drop_column")
          Text("Create index").tag("create_index")
          Text("Drop index").tag("drop_index")
          Text("Add constraint").tag("add_constraint")
          Text("Drop constraint").tag("drop_constraint")
        }
        .disabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
        TextField("Object name", text: $model.ddlChangeObjectName)
          .disabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
          .accessibilityIdentifier("structure.change.object")
        if needsDefinition {
          TextField(
            model.ddlChangeKind == "add_column"
              ? "Column type"
              : model.ddlChangeKind == "create_index"
                ? "Comma-separated columns" : "UNIQUE, PRIMARY KEY, or CHECK definition",
            text: $model.ddlChangeDefinition
          )
          .disabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
          .accessibilityIdentifier("structure.change.definition")
        }
      }
      .formStyle(.grouped)
      HStack {
        Button("Review Change…") { Task { await model.stageDdlChange() } }
          .buttonStyle(.glassProminent)
          .disabled(
            model.ddlChangeReview != nil || model.ddlChangeApplying
              || model.ddlChangeObjectName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              || (needsDefinition
                && model.ddlChangeDefinition.trimmingCharacters(in: .whitespacesAndNewlines)
                  .isEmpty)
          )
          .accessibilityIdentifier("structure.change.review")
        Button("Discard Review", role: .cancel) {
          Task { await model.discardDdlChangeReview() }
        }
        .disabled(model.ddlChangeReview == nil || model.ddlChangeApplying)
        Spacer()
      }
      if let review = model.ddlChangeReview {
        ChangeReviewPlane(
          kindWord: ChangeReviewPresentation.kindWord(
            preview: review.preview, destructive: review.destructive, fallback: "DDL"),
          title: "LEDGER · frozen structure plan",
          preview: review.preview,
          metadataFact: ChangeReviewPresentation.metadataStrip(
            target: nil,
            expiresAtMs: review.expiresAtMs,
            nowMs: model.nowMilliseconds(),
            destructive: review.destructive),
          destructive: review.destructive,
          production: model.activeProductionWarning,
          rollbackSummary: review.rollbackSummary,
          safetyNote: review.destructive
            ? "Removes structure — second confirmation required before apply." : nil,
          previewAccessibilityId: "structure.change.preview"
        )
        HStack {
          Button("Discard Review", role: .cancel) {
            Task { await model.discardDdlChangeReview() }
          }
          Spacer()
          Button("Apply Reviewed Change…") { applyConfirmationPresented = true }
            .buttonStyle(.glassProminent)
            .accessibilityIdentifier("structure.change.apply-review")
        }
      }
      if model.ddlChangeApplying { ProgressView("Applying structure change…") }
      if let outcome = model.ddlChangeOutcome {
        Label(outcome, systemImage: "checkmark.circle.fill")
          .foregroundStyle(.green)
          .accessibilityIdentifier("structure.change.outcome")
      }
      if let error = model.ddlChangeError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
      Spacer()
    }
    .padding(20)
    .frame(minWidth: 680, minHeight: 520)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
    .confirmationDialog(
      model.ddlChangeReview?.destructive == true
        ? "Apply destructive structure change?" : "Apply structure change?",
      isPresented: $applyConfirmationPresented,
      presenting: model.ddlChangeReview
    ) { review in
      if review.destructive {
        Button("Apply Destructive Change", role: .destructive) {
          Task { await model.applyDdlChange() }
        }
      } else {
        Button("Apply Structure Change") { Task { await model.applyDdlChange() } }
      }
      Button("Cancel", role: .cancel) {}
    } message: { review in
      Text("\(review.preview)\n\n\(review.rollbackSummary)")
    }
  }
}

struct TableOperationSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Text("CHANGE REVIEW")
          .font(.caption.weight(.bold))
          .tracking(0.6)
        Text("TABLE OP")
          .font(.caption2.weight(.bold).monospaced())
          .foregroundStyle(.secondary)
        Spacer()
        Button("Close") { Task { await model.closeTableOperation() } }
          .disabled(model.tableOperationApplying)
          .accessibilityIdentifier("table-operation.close")
      }
      Text("Table operation")
        .font(.title3.weight(.semibold))
      Picker("Operation", selection: $model.tableOperationKind) {
        if model.connectedEngine == "postgresql" {
          Text("Rename table").tag("rename")
          Text("Truncate all rows").tag("truncate")
          Text("Drop table").tag("drop")
          Text("Vacuum").tag("vacuum")
          Text("Analyze").tag("analyze")
        } else if model.connectedEngine == "clickhouse" {
          Text("Optimize table").tag("optimize")
        }
      }
      .disabled(model.tableOperationReview != nil || model.tableOperationApplying)
      .accessibilityIdentifier("table-operation.kind")
      .onChange(of: model.tableOperationKind) {
        Task { await model.resetTableOperationReview() }
      }
      if model.tableOperationKind == "rename" {
        TextField("New table name", text: $model.tableOperationNewName)
          .disabled(model.tableOperationReview != nil || model.tableOperationApplying)
          .accessibilityIdentifier("table-operation.new-name")
      }
      Button("Review Operation…") { Task { await model.stageTableOperation() } }
        .buttonStyle(.glassProminent)
        .disabled(
          model.tableOperationReview != nil || model.tableOperationApplying
            || (model.tableOperationKind == "rename"
              && model.tableOperationNewName.trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty)
        )
        .accessibilityIdentifier("table-operation.review")
      if let review = model.tableOperationReview {
        ChangeReviewPlane(
          kindWord: ChangeReviewPresentation.kindWord(
            preview: review.preview, destructive: review.destructive, fallback: "TABLE"),
          title: "LEDGER · frozen table operation",
          preview: review.preview,
          metadataFact: ChangeReviewPresentation.metadataStrip(
            target: review.target,
            expiresAtMs: review.expiresAtMs,
            nowMs: model.nowMilliseconds(),
            destructive: review.destructive,
            extra: "type \(review.confirmation) to authorize"),
          destructive: review.destructive,
          production: model.activeProductionWarning,
          rollbackSummary: review.destructive
            ? "Destroys table data — exact target name required."
            : "Exact target name required before apply.",
          safetyNote: nil,
          previewAccessibilityId: "table-operation.preview"
        )
        TextField("Exact table name", text: $model.tableOperationConfirmation)
          .accessibilityIdentifier("table-operation.confirmation")
        HStack {
          Button("Discard Review", role: .cancel) {
            Task { await model.resetTableOperationReview() }
          }
          Spacer()
          Button(review.destructive ? "Apply Destructive Operation" : "Apply Operation") {
            Task { await model.applyTableOperation() }
          }
          .buttonStyle(.glassProminent)
          .disabled(model.tableOperationConfirmation != review.confirmation)
          .accessibilityIdentifier("table-operation.apply")
        }
      }
      if model.tableOperationApplying {
        ProgressView(model.tableOperationStatus?.summary ?? "Starting table operation…")
          .accessibilityIdentifier("table-operation.progress")
        if model.tableOperationStatus?.cancellable == false {
          Text("Cancellation is unavailable for this engine operation.")
            .font(.caption)
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("table-operation.cancel-unavailable")
        }
      }
      if let outcome = model.tableOperationOutcome {
        Label(outcome, systemImage: "checkmark.circle.fill")
          .foregroundStyle(.green)
          .accessibilityIdentifier("table-operation.outcome")
      }
      if let error = model.tableOperationError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("table-operation.error")
      }
      Spacer()
    }
    .padding(20)
    .frame(minWidth: 680, minHeight: 500)
    .interactiveDismissDisabled(model.tableOperationReview != nil || model.tableOperationApplying)
    .accessibilityElement(children: .contain)
  }
}
