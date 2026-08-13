import SwiftUI
import TableRockFeature

enum ProfileEditorLayout {
  case sheet
  case workspace
}

struct ProfileEditorSheet: View {
  @Environment(\.dismiss) private var dismiss
  @State private var draft: ProfileEditorDraft
  @State private var saving = false
  @State private var testing = false
  @State private var testResult: String?

  let presentation: ProfileEditorLayout
  let onCancel: (() -> Void)?
  let onTest: ((ProfileEditorDraft) async -> String)?
  let onSave: (ProfileEditorDraft) async -> Bool

  init(
    initialDraft: ProfileEditorDraft,
    presentation: ProfileEditorLayout = .sheet,
    onCancel: (() -> Void)? = nil,
    onTest: ((ProfileEditorDraft) async -> String)? = nil,
    onSave: @escaping (ProfileEditorDraft) async -> Bool
  ) {
    _draft = State(initialValue: initialDraft)
    self.presentation = presentation
    self.onCancel = onCancel
    self.onTest = onTest
    self.onSave = onSave
  }

  private var canSave: Bool {
    !draft.name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      && !draft.host.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      && UInt16(draft.port) != nil
      && (draft.passwordSource != "dangerous_plaintext"
        || (!draft.passwordValue.isEmpty && draft.plaintextAcknowledged))
      && (draft.passwordSource != "keychain"
        || draft.passwordReference != nil || !draft.passwordValue.isEmpty)
      && (!draft.sshEnabled
        || (!draft.sshHost.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          && UInt16(draft.sshPort).map { $0 > 0 } == true
          && !draft.sshKnownHostsPath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          && (draft.sshAuthMode == "agent"
            || (draft.sshPlaintextAcknowledged
              && (draft.sshAuthMode == "password"
                ? (!draft.sshPassword.isEmpty || draft.sshHasStoredPassword)
                : (!draft.sshPrivateKey.isEmpty || draft.sshHasStoredPrivateKey))))))
      && draft.startupActions.count <= 16
      && draft.startupActions.allSatisfy {
        !$0.statement.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
          && (100...120_000).contains($0.timeoutMs)
      }
  }

  private var title: String {
    let engine = ProfileEngineBadge.accessibilityName(draft.engine)
    return draft.idBytes == nil ? "New \(engine) Connection" : "Edit \(draft.name)"
  }

  var body: some View {
    Group {
      switch presentation {
      case .sheet: sheet
      case .workspace: workspace
      }
    }
    .interactiveDismissDisabled(saving || testing)
  }

  private var sheet: some View {
    NavigationStack {
      VStack(spacing: 0) {
        editorForm
        Divider()
        HStack {
          Button("Cancel") { cancel() }
            .accessibilityIdentifier("profile.editor.cancel")
          Spacer()
          if onTest != nil {
            Button("Test Connection") { test() }
              .accessibilityIdentifier("profile.editor.test")
              .disabled(!canSave || saving || testing)
          }
          Button("Save") { save() }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("profile.editor.save")
            .keyboardShortcut(.defaultAction)
            .disabled(!canSave || saving || testing)
        }
        .padding(.horizontal, 18)
        .frame(height: 52)
        .background(Color(nsColor: .windowBackgroundColor))
      }
      .navigationTitle(draft.idBytes == nil ? "New Connection" : "Edit Connection")
    }
    .frame(minWidth: 620, minHeight: 680)
  }

  private var workspace: some View {
    VStack(spacing: 0) {
      HStack {
        VStack(alignment: .leading, spacing: 2) {
          Text(title)
            .font(.title2.weight(.semibold))
            .accessibilityIdentifier("connection.setup")
          Text("Configure identity, endpoint, credentials, and connection safety")
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        Spacer()
        Label(
          draft.idBytes == nil ? "NOT SAVED" : "SAVED PROFILE",
          systemImage: draft.idBytes == nil ? "circle.dashed" : "checkmark.circle"
        )
        .font(.caption2.weight(.semibold))
        .foregroundStyle(.secondary)
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(.quaternary.opacity(0.5), in: .capsule)
      }
      .padding(.horizontal, 24)
      .frame(height: 76)
      Divider()

      HSplitView {
        editorForm
          .frame(minWidth: 520)
        ProfileEditorSummary(draft: draft, testResult: testResult)
          .frame(minWidth: 270, idealWidth: 310, maxWidth: 360)
      }

      Divider()
      HStack {
        Button("Cancel") { cancel() }
        Spacer()
        Button("Test Connection", systemImage: "wave.3.right") { test() }
          .disabled(!canSave || saving || testing)
          .accessibilityIdentifier("profile.editor.test")
        Button("Save & Connect", systemImage: "arrow.right") { save() }
          .buttonStyle(.borderedProminent)
          .disabled(!canSave || saving || testing)
          .accessibilityIdentifier("profile.editor.save")
          .keyboardShortcut(.defaultAction)
      }
      .padding(.horizontal, 18)
      .frame(height: 52)
      .background(Color(nsColor: .windowBackgroundColor))
    }
    .background(Color(nsColor: .controlBackgroundColor))
  }

  private var editorForm: some View {
    Form {
      Section("General") {
        Picker("Engine", selection: $draft.engine) {
          Text("PostgreSQL").tag("postgresql")
          Text("ClickHouse").tag("clickhouse")
          Text("Redis").tag("redis")
        }
        .accessibilityIdentifier("profile.editor.engine")
        TextField("Name", text: $draft.name)
          .accessibilityIdentifier("profile.editor.name")
        TextField("Group", text: $draft.group)
        Picker("Environment", selection: $draft.environment) {
          Text("None").tag("")
          Text("Production").tag("production")
          Text("Staging").tag("staging")
          Text("Development").tag("development")
          Text("Testing").tag("testing")
        }
        Picker("Safety", selection: $draft.safetyMode) {
          Text("Read only").tag("read_only")
          Text("Confirm writes").tag("confirm_writes")
        }
      }

      Section("Server") {
        TextField("Host", text: $draft.host)
          .accessibilityIdentifier("profile.editor.host")
        TextField("Port", text: $draft.port)
          .accessibilityIdentifier("profile.editor.port")
        TextField(
          draft.engine == "redis" ? "Logical database" : "Default database",
          text: $draft.database
        )
        .accessibilityIdentifier("profile.editor.database")
        TextField("Username", text: $draft.username)
          .accessibilityIdentifier("profile.editor.username")
      }

      Section("Credentials") {
        Picker("Password storage", selection: $draft.passwordSource) {
          Text("Prompt on connect").tag("prompt")
          Text("Save locally (dangerous)").tag("dangerous_plaintext")
          Text("Environment variable").tag("environment")
          Text("1Password reference").tag("onepassword")
          Text("macOS Keychain").tag("keychain")
        }
        .accessibilityIdentifier("profile.editor.password-source")
        if draft.passwordSource == "dangerous_plaintext" {
          SecureField(
            draft.hasStoredPassword ? "Re-enter stored password" : "Password",
            text: $draft.passwordValue
          )
          Toggle(
            "I understand this stores the password as plaintext locally",
            isOn: $draft.plaintextAcknowledged
          )
          .foregroundStyle(.orange)
        } else if draft.passwordSource == "environment" {
          TextField("Environment variable name", text: $draft.passwordValue)
        } else if draft.passwordSource == "onepassword" {
          TextField("account vault item [section] field", text: $draft.passwordValue)
        } else if draft.passwordSource == "keychain" {
          SecureField(
            draft.hasStoredPassword ? "Replace Keychain password" : "Password",
            text: $draft.passwordValue
          )
        }
      }

      Section("Transport") {
        Picker("TLS", selection: $draft.tlsMode) {
          Text("Off").tag("off")
          Text("Verify CA").tag("verify_ca")
          Text("Verify full").tag("verify_full")
        }
        Toggle("Connect through SSH bastion", isOn: $draft.sshEnabled)
          .accessibilityIdentifier("profile.editor.ssh.enabled")
        if draft.sshEnabled {
          TextField("Bastion host", text: $draft.sshHost)
            .accessibilityIdentifier("profile.editor.ssh.host")
          TextField("SSH port", text: $draft.sshPort)
            .accessibilityIdentifier("profile.editor.ssh.port")
          TextField("SSH username", text: $draft.sshUsername)
            .accessibilityIdentifier("profile.editor.ssh.username")
          Picker("Authentication", selection: $draft.sshAuthMode) {
            Text("SSH agent").tag("agent")
            Text("Password").tag("password")
            Text("OpenSSH private key").tag("private_key")
          }
          .accessibilityIdentifier("profile.editor.ssh.authentication")
          if draft.sshAuthMode == "password" {
            SecureField(
              draft.sshHasStoredPassword ? "Replace stored SSH password" : "SSH password",
              text: $draft.sshPassword
            )
            .accessibilityIdentifier("profile.editor.ssh.password")
          } else if draft.sshAuthMode == "private_key" {
            TextEditor(text: $draft.sshPrivateKey)
              .frame(minHeight: 90)
              .accessibilityLabel("OpenSSH private key")
              .accessibilityIdentifier("profile.editor.ssh.private-key")
            SecureField("Private-key passphrase (optional)", text: $draft.sshPassword)
          }
          LabeledContent("Host-key policy", value: "OpenSSH known_hosts · fail closed")
          TextField("Absolute known_hosts path", text: $draft.sshKnownHostsPath)
            .accessibilityIdentifier("profile.editor.ssh.known-hosts")
          if draft.sshAuthMode != "agent" {
            Toggle(
              "I understand SSH secrets are stored as acknowledged local plaintext",
              isOn: $draft.sshPlaintextAcknowledged
            )
            .foregroundStyle(.orange)
            Text(
              "Use SSH agent where available. Secret values never appear in logs or profile reads."
            )
            .font(.caption)
            .foregroundStyle(.secondary)
          }
        }
      }

      Section("Startup Commands") {
        ForEach($draft.startupActions) { $action in
          VStack(alignment: .leading, spacing: 8) {
            TextEditor(text: $action.statement)
              .font(.system(.body, design: .monospaced))
              .frame(minHeight: 60)
              .accessibilityLabel("Startup command")
              .accessibilityIdentifier("profile.editor.startup.statement")
            HStack {
              Picker("Safety", selection: $action.safety) {
                Text("Read only · auto-run").tag("read_only")
                Text("Write · review required").tag("write")
                Text("Dangerous · review required").tag("dangerous")
              }
              TextField("Timeout ms", value: $action.timeoutMs, format: .number)
                .frame(width: 150)
            }
            Toggle("Run again after reconnect", isOn: $action.runOnReconnect)
            HStack {
              Button("Move Up") {
                guard let index = draft.startupActions.firstIndex(where: { $0.id == action.id }),
                  index > 0
                else { return }
                draft.startupActions.swapAt(index, index - 1)
              }
              Button("Move Down") {
                guard let index = draft.startupActions.firstIndex(where: { $0.id == action.id }),
                  index + 1 < draft.startupActions.count
                else { return }
                draft.startupActions.swapAt(index, index + 1)
              }
              Button("Remove", role: .destructive) {
                draft.startupActions.removeAll { $0.id == action.id }
              }
            }
            if action.safety != "read_only" {
              Label("Never auto-runs; explicit review required", systemImage: "hand.raised.fill")
                .font(.caption)
                .foregroundStyle(.orange)
            }
          }
          .padding(.vertical, 4)
        }
        Button("Add Startup Command", systemImage: "plus") {
          guard draft.startupActions.count < 16 else { return }
          draft.startupActions.append(
            StartupActionEditorDraft(
              WorkbenchStartupActionDraft(
                statement: draft.engine == "redis" ? "PING" : "SELECT 1",
                safety: "read_only", timeoutMs: 5_000, runOnReconnect: false
              )
            )
          )
        }
        .accessibilityIdentifier("profile.editor.startup.add")
        Text(
          "Commands run in listed order. Read-only commands may auto-run. Write and dangerous commands always wait for separate review."
        )
        .font(.caption)
        .foregroundStyle(.secondary)
      }

      if let testResult {
        Section("Connection Test") {
          Label(testResult, systemImage: "info.circle")
            .textSelection(.enabled)
            .accessibilityIdentifier("profile.editor.test-result")
        }
      }
    }
    .formStyle(.grouped)
  }

  private func cancel() {
    if let onCancel { onCancel() } else { dismiss() }
  }

  private func test() {
    guard !testing, let onTest else { return }
    testing = true
    Task {
      testResult = await onTest(draft)
      testing = false
    }
  }

  private func save() {
    guard !saving else { return }
    saving = true
    Task {
      if await onSave(draft), presentation == .sheet { dismiss() }
      saving = false
    }
  }
}

private struct ProfileEditorSummary: View {
  let draft: ProfileEditorDraft
  let testResult: String?

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Image(systemName: connectionEngineSymbol(draft.engine))
        .font(.system(size: 34))
        .foregroundStyle(Color.accentColor)
        .frame(width: 64, height: 64)
        .background(Color.accentColor.opacity(0.09), in: .rect(cornerRadius: 12))
      VStack(alignment: .leading, spacing: 3) {
        Text(draft.name.isEmpty ? "Untitled Connection" : draft.name)
          .font(.headline)
        Text(ProfileEngineBadge.accessibilityName(draft.engine))
          .font(.caption)
          .foregroundStyle(.secondary)
      }
      Divider()
      LabeledContent("Endpoint", value: "\(draft.host):\(draft.port)")
      LabeledContent("Database", value: draft.database.isEmpty ? "Default" : draft.database)
      LabeledContent("TLS", value: tlsSummary)
      LabeledContent(
        "Safety",
        value: draft.safetyMode == "read_only" ? "Read only" : "Review writes"
      )
      Divider()
      if let testResult {
        Label(testResult, systemImage: "info.circle")
          .font(.caption)
          .foregroundStyle(.secondary)
      } else {
        Label("No connection test has run", systemImage: "info.circle")
          .font(.caption)
          .foregroundStyle(.secondary)
      }
      Spacer()
    }
    .padding(24)
    .background(Color(nsColor: .windowBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityLabel("Connection summary")
  }

  private var tlsSummary: String {
    switch draft.tlsMode {
    case "off": "Off"
    case "verify_ca": "Verify CA"
    default: "Verify full"
    }
  }
}
