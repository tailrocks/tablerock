import SwiftUI

struct FindReplaceSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Find and Replace", systemImage: "text.magnifyingglass")
          .font(.title2.bold())
        Spacer()
        Button("Done") { dismiss() }
          .accessibilityIdentifier("find-replace.dismiss")
      }
      TextField("Find", text: $model.findPattern)
        .accessibilityIdentifier("find-replace.pattern")
        .onChange(of: model.findPattern) { model.resetFindTraversal() }
      TextField("Replace with", text: $model.findReplacement)
        .accessibilityIdentifier("find-replace.replacement")
      Picker("Mode", selection: $model.findMode) {
        Text("Literal · Ignore Case").tag("literal")
        Text("Literal · Match Case").tag("case_sensitive")
        Text("Whole Word · Ignore Case").tag("whole_word")
        Text("Regular Expression").tag("regular_expression")
      }
      .accessibilityIdentifier("find-replace.mode")
      .onChange(of: model.findMode) { model.resetFindTraversal() }
      Picker("Scope", selection: $model.findScope) {
        Text("Document").tag("document")
        Text("Current Selection").tag("selection")
      }
      .pickerStyle(.segmented)
      .accessibilityIdentifier("find-replace.scope")
      .onChange(of: model.findScope) { _, scope in model.setFindScope(scope) }
      HStack {
        Button("Previous") { model.findEditorMatch(backwards: true) }
          .accessibilityIdentifier("find-replace.previous")
        Button("Next") { model.findEditorMatch(backwards: false) }
          .keyboardShortcut(.return, modifiers: [])
          .accessibilityIdentifier("find-replace.next")
        Spacer()
        Button("Replace") { model.replaceEditorMatch() }
          .accessibilityIdentifier("find-replace.replace")
        Button("Replace All") { model.replaceAllEditorMatches() }
          .accessibilityIdentifier("find-replace.replace-all")
      }
      .disabled(model.findPattern.isEmpty)
      if let status = model.findStatus {
        Label(status, systemImage: "checkmark.circle")
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("find-replace.status")
      }
      if let error = model.findError {
        Label(error, systemImage: "exclamationmark.triangle")
          .foregroundStyle(.red)
          .textSelection(.enabled)
          .accessibilityIdentifier("find-replace.error")
      }
    }
    .padding(20)
    .frame(width: 520)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("find-replace.sheet")
  }
}

struct QueryParametersSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      Label("Query Parameters", systemImage: "list.bullet.rectangle")
        .font(.title2.bold())
      Text("Values cross the Rust boundary separately from SQL text.")
        .foregroundStyle(.secondary)
      ForEach($model.queryParameterBindings) { $binding in
        HStack(alignment: .firstTextBaseline) {
          Text(":\(binding.name)")
            .font(.system(.body, design: .monospaced))
            .frame(width: 130, alignment: .leading)
          Picker("Type", selection: $binding.kind) {
            Text("Text").tag("text")
            Text("Integer").tag("integer")
            Text("Float").tag("float")
            Text("Boolean").tag("boolean")
            Text("NULL").tag("null")
          }
          .frame(width: 130)
          .accessibilityIdentifier("query-parameters.type.\(binding.name)")
          .onChange(of: binding.kind) { _, kind in
            if kind == "boolean" && !["true", "false"].contains(binding.value) {
              binding.value = "true"
            } else if kind == "null" {
              binding.value = ""
            }
          }
          if binding.kind == "boolean" {
            Picker("Value", selection: $binding.value) {
              Text("True").tag("true")
              Text("False").tag("false")
            }
            .accessibilityIdentifier("query-parameters.value.\(binding.name)")
          } else if binding.kind == "null" {
            Text("NULL").foregroundStyle(.secondary).frame(maxWidth: .infinity)
          } else {
            TextField("Value", text: $binding.value)
              .accessibilityIdentifier("query-parameters.value.\(binding.name)")
          }
        }
      }
      if let error = model.queryParameterError {
        Label(error, systemImage: "exclamationmark.triangle")
          .foregroundStyle(.red)
          .textSelection(.enabled)
          .accessibilityIdentifier("query-parameters.error")
      }
      HStack {
        Spacer()
        Button(model.isRunning ? "Cancel Query" : "Cancel", role: .cancel) {
          if model.isRunning {
            Task { await model.cancel() }
          } else {
            model.cancelQueryParameters()
          }
        }
        .accessibilityIdentifier("query-parameters.cancel")
        Button("Run") { Task { await model.runParameterizedQuery() } }
          .buttonStyle(.glassProminent)
          .disabled(model.isRunning)
          .accessibilityIdentifier("query-parameters.run")
      }
    }
    .padding(20)
    .frame(minWidth: 620)
    .interactiveDismissDisabled(model.isRunning)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("query-parameters.sheet")
  }
}
