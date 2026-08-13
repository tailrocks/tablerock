import SwiftUI
import TableRockFeature

private struct ChangeReviewBadge: View {
  let text: String
  let destructive: Bool

  var body: some View {
    Label(text, systemImage: destructive ? "trash.fill" : "lock.shield.fill")
      .font(.caption2.weight(.bold))
      .foregroundStyle(destructive ? Color.red : .green)
      .padding(.horizontal, 10)
      .padding(.vertical, 4)
      .background((destructive ? Color.red : .green).opacity(0.1), in: Capsule())
      .accessibilityIdentifier(
        destructive ? "change.review.destructive" : "change.review.safe")
  }
}

private struct ChangeReviewHeader: View {
  let destructive: Bool
  let title: String
  let summary: String

  var body: some View {
    HStack(alignment: .top, spacing: 14) {
      Image(systemName: destructive ? "exclamationmark.triangle.fill" : "checkmark.shield.fill")
        .font(.title)
        .foregroundStyle(destructive ? Color.red : .green)
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 4) {
        Text(title)
          .font(.title2.weight(.semibold))
        Text(summary)
          .foregroundStyle(.secondary)
      }
      Spacer(minLength: 12)
      ChangeReviewBadge(
        text: destructive ? "DESTRUCTIVE" : "SAFE CHANGE",
        destructive: destructive
      )
    }
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("change.review.header")
  }
}

private struct ChangeReviewEntry: View {
  let kind: String
  let title: String
  let preview: String
  let destructive: Bool

  var body: some View {
    HStack(alignment: .top, spacing: 12) {
      Text(kind.uppercased())
        .font(.caption2.weight(.bold))
        .foregroundStyle(destructive ? Color.red : .orange)
        .padding(.horizontal, 9)
        .padding(.vertical, 4)
        .background((destructive ? Color.red : .orange).opacity(0.1), in: Capsule())
      VStack(alignment: .leading, spacing: 4) {
        Text(title)
          .fontWeight(.semibold)
        Text(preview)
          .font(.callout.monospaced())
          .textSelection(.enabled)
          .accessibilityLabel("Change preview")
          .accessibilityValue(preview)
          .accessibilityIdentifier("change.review.entry.preview")
      }
      Spacer(minLength: 0)
    }
    .padding(.vertical, 6)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("change.review.entry")
  }
}

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
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .top) {
        ChangeReviewEntry(
          kind: kindWord,
          title: title,
          preview: preview,
          destructive: destructive
        )
        .accessibilityIdentifier(previewAccessibilityId)
        Spacer(minLength: 8)
        if production {
          Text("PRODUCTION")
            .font(.caption2.weight(.bold))
            .foregroundStyle(.red)
            .accessibilityLabel("Production — writes need review")
        }
      }
      Text(metadataFact)
        .font(.caption2.monospaced())
        .foregroundStyle(.secondary)
        .textSelection(.enabled)
        .accessibilityIdentifier("change.review.metadata")
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
    .padding(12)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(Color(nsColor: .textBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("change.review.plane")
  }
}

struct MutationWorkflowSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    Group {
      if let review = model.activeObjectTab?.mutationReview {
        reviewSurface(review)
      } else if let draft = model.rowEditDraft {
        editorSurface(draft)
      } else {
        ContentUnavailableView("No staged row update", systemImage: "tablecells")
      }
    }
    .frame(minWidth: 650, minHeight: 520)
    .interactiveDismissDisabled(
      model.activeObjectTab?.mutationReview != nil
        || model.activeObjectTab?.mutationApplying == true
    )
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("mutation.workflow.sheet")
  }

  private func editorSurface(_ draft: NativeRowEditDraft) -> some View {
    @Bindable var draft = draft
    return VStack(alignment: .leading, spacing: 16) {
      HStack(alignment: .top, spacing: 12) {
        Image(systemName: "pencil.and.list.clipboard")
          .font(.title)
          .foregroundStyle(.blue)
        VStack(alignment: .leading, spacing: 3) {
          Text("Edit Selected Row")
            .font(.title2.weight(.semibold))
          Text(
            "\(draft.relation) · row \(draft.row + 1) · changes stay local until review and apply"
          )
          .foregroundStyle(.secondary)
        }
        Spacer()
      }
      Divider()
      Form {
        ForEach($draft.fields) { $field in
          if field.kind == "boolean" {
            Picker(field.column, selection: $field.value) {
              Text("true").tag("true")
              Text("false").tag("false")
            }
            .accessibilityIdentifier("mutation.field.\(field.column)")
          } else {
            TextField(field.column, text: $field.value)
              .textFieldStyle(.roundedBorder)
              .accessibilityLabel("\(field.column), \(field.kind)")
              .accessibilityIdentifier("mutation.field.\(field.column)")
          }
        }
      }
      .formStyle(.grouped)
      if let error = model.activeObjectTab?.mutationError {
        Text(error)
          .foregroundStyle(.red)
          .textSelection(.enabled)
          .accessibilityIdentifier("mutation.error")
      }
      Spacer(minLength: 0)
      HStack {
        Button("Discard") { Task { await model.discardRowUpdate() } }
          .keyboardShortcut(.cancelAction)
        Spacer()
        Button("Stage for Review") { Task { await model.stageRowUpdate() } }
          .buttonStyle(.glassProminent)
          .accessibilityIdentifier("mutation.stage-review")
      }
    }
    .padding(22)
  }

  private func reviewSurface(_ review: WorkbenchMutationReview) -> some View {
    VStack(spacing: 0) {
      ChangeReviewHeader(
        destructive: false,
        title: "Review staged changes",
        summary:
          "\(counted(review.lines.count, "safe update")) will apply in one PostgreSQL transaction."
      )
      .padding(22)
      Divider()
      List {
        ForEach(review.lines) { line in
          ChangeReviewEntry(
            kind: line.kind,
            title: review.target,
            preview: line.preview,
            destructive: false
          )
          .accessibilityIdentifier("mutation.review.entry")
          if !line.parameters.isEmpty {
            Text(
              line.parameters.enumerated()
                .map { "$\($0.offset + 1) = \($0.element)" }
                .joined(separator: " · ")
            )
            .font(.caption.monospaced())
            .foregroundStyle(.secondary)
            .textSelection(.enabled)
            .accessibilityIdentifier("mutation.review.parameters")
          }
        }
        Section("Review authority") {
          Text(
            ChangeReviewPresentation.metadataStrip(
              target: review.target, expiresAtMs: review.expiresAtMs,
              nowMs: model.nowMilliseconds(), destructive: false)
          )
          .font(.caption.monospaced())
          Text("Apply consumes this review once. Conflict rolls back the transaction.")
            .font(.caption)
            .foregroundStyle(.secondary)
        }
      }
      if let error = model.activeObjectTab?.mutationError {
        Text(error)
          .foregroundStyle(.red)
          .padding(.horizontal, 22)
          .textSelection(.enabled)
          .accessibilityIdentifier("mutation.error")
      }
      Divider()
      HStack {
        Button("Discard All", role: .destructive) {
          Task { await model.discardRowUpdate() }
        }
        Button("Back") { Task { await model.backToRowEditor() } }
        Spacer()
        if model.activeObjectTab?.mutationApplying == true {
          ProgressView("Applying…")
        }
        Button("Apply Updates") { Task { await model.applyRowUpdate() } }
          .buttonStyle(.glassProminent)
          .disabled(model.activeObjectTab?.mutationApplying == true)
          .accessibilityIdentifier("mutation.apply")
      }
      .padding(18)
    }
  }
}

struct DdlChangeSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @State private var destructiveConfirmation = ""

  private var needsDefinition: Bool {
    ["add_column", "create_index", "add_constraint"].contains(model.ddlChangeKind)
  }

  var body: some View {
    @Bindable var model = model
    Group {
      if let review = model.ddlChangeReview {
        reviewSurface(review)
      } else {
        editorSurface
      }
    }
    .frame(minWidth: 680, minHeight: 590)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("structure.change.sheet")
    .interactiveDismissDisabled(model.ddlChangeReview != nil || model.ddlChangeApplying)
    .onChange(of: model.ddlChangeReview != nil) { _, reviewing in
      if !reviewing { destructiveConfirmation = "" }
    }
  }

  private var editorSurface: some View {
    @Bindable var model = model
    return VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Structure change", systemImage: "tablecells.badge.ellipsis")
          .font(.title2.weight(.semibold))
        Spacer()
        Button("Close") { Task { await model.closeDdlChange() } }
          .disabled(model.ddlChangeApplying)
      }
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
        Spacer()
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
  }

  private func reviewSurface(_ review: WorkbenchDdlChangeReview) -> some View {
    VStack(spacing: 0) {
      ChangeReviewHeader(
        destructive: review.destructive,
        title: review.destructive ? "Review destructive change" : "Review staged change",
        summary: review.destructive
          ? "One frozen structure operation can remove database structure."
          : "One safe structure operation will use reviewed, one-time authority."
      )
      .padding(22)

      Divider()

      List {
        ChangeReviewEntry(
          kind: ChangeReviewPresentation.kindWord(
            preview: review.preview, destructive: review.destructive, fallback: "DDL"),
          title: model.selectedObjectTab?.title ?? "Selected relation",
          preview: review.preview,
          destructive: review.destructive
        )
        .accessibilityIdentifier("structure.change.preview")

        Section("Review authority") {
          Text(
            ChangeReviewPresentation.metadataStrip(
              target: nil,
              expiresAtMs: review.expiresAtMs,
              nowMs: model.nowMilliseconds(),
              destructive: review.destructive)
          )
          .font(.caption.monospaced())
          .textSelection(.enabled)
          Text(review.rollbackSummary)
            .font(.caption)
            .textSelection(.enabled)
            .accessibilityIdentifier("change.review.rollback")
        }
      }
      .listStyle(.inset)

      Divider()

      if review.destructive {
        VStack(alignment: .leading, spacing: 8) {
          Text("Type APPLY to enable the destructive action.")
            .font(.caption)
            .foregroundStyle(.secondary)
          TextField("APPLY", text: $destructiveConfirmation)
            .textFieldStyle(.roundedBorder)
            .accessibilityIdentifier("structure.change.confirmation")
        }
        .padding(.horizontal, 22)
        .padding(.top, 14)
      }

      HStack {
        Button("Discard Change", role: .destructive) {
          Task { await model.closeDdlChange() }
        }
        Spacer()
        Button("Back to Editing") {
          Task { await model.discardDdlChangeReview() }
        }
        Button(
          review.destructive
            ? (model.activeProductionWarning
              ? "Apply on PRODUCTION" : "Apply Destructive Change")
            : "Apply Change",
          role: review.destructive ? .destructive : nil
        ) {
          Task { await model.applyDdlChange() }
        }
        .disabled(review.destructive && destructiveConfirmation != "APPLY")
        .keyboardShortcut(.defaultAction)
        .accessibilityIdentifier("structure.change.apply-review")
      }
      .padding(22)
    }
  }
}

struct TableOperationSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    Group {
      if let review = model.tableOperationReview {
        reviewSurface(review)
      } else {
        editorSurface
      }
    }
    .frame(minWidth: 680, minHeight: 590)
    .interactiveDismissDisabled(model.tableOperationReview != nil || model.tableOperationApplying)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("table-operation.sheet")
  }

  private var editorSurface: some View {
    @Bindable var model = model
    return VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Table operation", systemImage: "wrench.and.screwdriver")
          .font(.title2.weight(.semibold))
        Spacer()
        Button("Close") { Task { await model.closeTableOperation() } }
          .disabled(model.tableOperationApplying)
          .accessibilityIdentifier("table-operation.close")
      }
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
  }

  private func reviewSurface(_ review: WorkbenchTableOperationReview) -> some View {
    @Bindable var model = model
    return VStack(spacing: 0) {
      ChangeReviewHeader(
        destructive: review.destructive,
        title: review.destructive ? "Review destructive operation" : "Review table operation",
        summary: review.destructive
          ? "One frozen operation can remove or replace table data."
          : "One reviewed table operation will use one-time authority."
      )
      .padding(22)

      Divider()

      List {
        ChangeReviewEntry(
          kind: ChangeReviewPresentation.kindWord(
            preview: review.preview, destructive: review.destructive, fallback: "TABLE"),
          title: review.target,
          preview: review.preview,
          destructive: review.destructive
        )
        .accessibilityIdentifier("table-operation.preview")

        Section("Review authority") {
          Text(
            ChangeReviewPresentation.metadataStrip(
              target: review.target,
              expiresAtMs: review.expiresAtMs,
              nowMs: model.nowMilliseconds(),
              destructive: review.destructive,
              extra: "exact target required")
          )
          .font(.caption.monospaced())
          .textSelection(.enabled)
          Text(
            review.destructive
              ? "This can destroy table data. Rust consumes authority before database I/O."
              : "Rust consumes authority before database I/O."
          )
          .font(.caption)
          .textSelection(.enabled)
        }
      }
      .listStyle(.inset)

      Divider()

      VStack(alignment: .leading, spacing: 8) {
        Text("Type \(review.confirmation) to enable this operation.")
          .font(.caption)
          .foregroundStyle(.secondary)
        TextField("Exact table name", text: $model.tableOperationConfirmation)
          .textFieldStyle(.roundedBorder)
          .accessibilityIdentifier("table-operation.confirmation")
      }
      .padding(.horizontal, 22)
      .padding(.top, 14)

      HStack {
        Button("Discard Operation", role: .destructive) {
          Task { await model.closeTableOperation() }
        }
        Spacer()
        Button("Back to Editing") {
          Task { await model.resetTableOperationReview() }
        }
        Button(
          review.destructive
            ? (model.activeProductionWarning
              ? "Apply on PRODUCTION" : "Apply Destructive Operation")
            : "Apply Operation",
          role: review.destructive ? .destructive : nil
        ) {
          Task { await model.applyTableOperation() }
        }
        .disabled(model.tableOperationConfirmation != review.confirmation)
        .keyboardShortcut(.defaultAction)
        .accessibilityIdentifier("table-operation.apply")
      }
      .padding(22)
    }
  }
}
