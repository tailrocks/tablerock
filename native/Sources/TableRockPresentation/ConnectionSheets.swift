import SwiftUI
import TableRockFeature

struct ProfilePasswordSheet: View {
  @Environment(\.dismiss) private var dismiss
  let profile: WorkbenchProfileItem
  let onConnect: (String) async -> Bool
  @State private var password = ""
  @State private var connecting = false

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text("Connect to \(profile.name)").font(.title2).bold()
      Text("Password stays in memory for this connection attempt and is never saved.")
        .foregroundStyle(.secondary)
      SecureField("Password", text: $password)
        .textContentType(.password)
        .onSubmit { submit() }
      HStack {
        Spacer()
        Button("Cancel", role: .cancel) { dismiss() }
          .disabled(connecting)
        Button("Connect") { submit() }
          .buttonStyle(.glassProminent)
          .disabled(connecting)
      }
    }
    .padding(24)
    .frame(width: 420)
    .interactiveDismissDisabled(connecting)
  }

  private func submit() {
    guard !connecting else { return }
    connecting = true
    let transientPassword = password
    password = ""
    Task {
      if await onConnect(transientPassword) { dismiss() } else { connecting = false }
    }
  }
}

struct ProfileGroupEditorSheet: View {
  @Environment(\.dismiss) private var dismiss
  @State private var dialog: ProfileGroupDialog
  @State private var saving = false
  let onSave: (ProfileGroupDialog) async -> Bool

  init(
    initialDialog: ProfileGroupDialog,
    onSave: @escaping (ProfileGroupDialog) async -> Bool
  ) {
    _dialog = State(initialValue: initialDialog)
    self.onSave = onSave
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text(dialog.title).font(.title2).bold()
      TextField("Group name", text: $dialog.name)
        .textFieldStyle(.roundedBorder)
      HStack {
        Spacer()
        Button("Cancel") { dismiss() }
        Button("Save") {
          saving = true
          Task {
            if await onSave(dialog) { dismiss() }
            saving = false
          }
        }
        .keyboardShortcut(.defaultAction)
        .disabled(
          dialog.name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            || saving)
      }
    }
    .padding(24)
    .frame(width: 380)
    .interactiveDismissDisabled(saving)
  }
}

struct ConnectionUrlImportSheet: View {
  @Environment(\.dismiss) private var dismiss
  @State private var input: String
  @State private var error: String?
  @State private var parsing = false
  let onReview: (String) async -> String?

  init(initial: ConnectionUrlImport, onReview: @escaping (String) async -> String?) {
    _input = State(initialValue: initial.input)
    _error = State(initialValue: initial.error)
    self.onReview = onReview
  }

  var body: some View {
    NavigationStack {
      Form {
        Section("Database URL") {
          SecureField("postgresql://user:password@host/database", text: $input)
            .accessibilityIdentifier("profile.url-import.input")
          Text("Parsed fields are reviewed before saving. Passwords default to macOS Keychain.")
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        if let error {
          Section("Validation") {
            Text(error)
              .foregroundStyle(.red)
              .textSelection(.enabled)
              .accessibilityIdentifier("profile.url-import.error")
          }
        }
      }
      .formStyle(.grouped)
      .navigationTitle("Import Connection URL")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
        ToolbarItem(placement: .confirmationAction) {
          Button("Review") {
            parsing = true
            Task {
              error = await onReview(input)
              parsing = false
            }
          }
          .accessibilityIdentifier("profile.url-import.review")
          .keyboardShortcut(.defaultAction)
          .disabled(input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || parsing)
        }
      }
    }
    .frame(minWidth: 520, minHeight: 300)
    .interactiveDismissDisabled(parsing)
  }
}

struct ExternalUrlConfirmationSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss
  let review: ExternalUrlReview

  var body: some View {
    NavigationStack {
      Form {
        Section("Requested target") {
          Text(review.summary)
            .textSelection(.enabled)
            .accessibilityIdentifier("external-url.summary")
          Text("No connection or profile change occurs until you choose an action.")
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        if let profile = review.matchedProfile {
          Section("Saved match") {
            Text(profile.name)
            Button("Connect saved profile") {
              Task { await model.connectExternalSavedProfile() }
            }
            .accessibilityIdentifier("external-url.connect-saved")
          }
        }
        Section("Choose action") {
          Button("Connect Temporarily") {
            Task { await model.connectExternalTemporarily() }
          }
          .buttonStyle(.glassProminent)
          .accessibilityIdentifier("external-url.connect-temporary")

          Button("Review as New") { model.reviewExternalURLAsNewConnection() }
            .accessibilityIdentifier("external-url.review-new")
        }
      }
      .formStyle(.grouped)
      .navigationTitle("Open External Connection?")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") {
            model.externalUrlReview = nil
            dismiss()
          }
          .accessibilityIdentifier("external-url.cancel")
        }
      }
    }
    .frame(minWidth: 560, minHeight: 320)
  }
}

struct QuickSwitcherSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    NavigationStack {
      VStack(spacing: 0) {
        TextField("Connections, tabs, objects, queries", text: $model.quickSwitcherSearch)
          .textFieldStyle(.roundedBorder)
          .accessibilityIdentifier("quick-switch.search")
          .onSubmit {
            guard let first = model.quickSwitcherItems.first else { return }
            Task { await model.activateQuickSwitcherItem(first) }
          }
          .padding()
        Divider()
        List(model.quickSwitcherItems) { item in
          Button {
            Task { await model.activateQuickSwitcherItem(item) }
          } label: {
            HStack(spacing: 10) {
              Image(systemName: item.favorite ? "star.fill" : "arrow.right.circle")
                .foregroundStyle(item.favorite ? .yellow : .secondary)
              VStack(alignment: .leading, spacing: 2) {
                Text(item.title)
                Text(item.subtitle).font(.caption).foregroundStyle(.secondary)
              }
              Spacer()
            }
            .contentShape(Rectangle())
          }
          .buttonStyle(.plain)
          .accessibilityIdentifier("quick-switch.item.\(item.id)")
        }
        .overlay {
          if model.quickSwitcherItems.isEmpty {
            ContentUnavailableView.search(text: model.quickSwitcherSearch)
          }
        }
      }
      .onExitCommand {
        model.quickSwitcherPresented = false
        dismiss()
      }
      .navigationTitle("Quick Switcher")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") {
            model.quickSwitcherPresented = false
            dismiss()
          }
        }
      }
    }
    .frame(minWidth: 560, minHeight: 420)
  }
}
