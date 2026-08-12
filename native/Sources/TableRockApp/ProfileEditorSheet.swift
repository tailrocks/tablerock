import SwiftUI
import TableRockFeature

struct ProfileEditorSheet: View {
  @Environment(\.dismiss) private var dismiss
  @State private var draft: ProfileEditorDraft
  @State private var saving = false
  let onSave: (ProfileEditorDraft) async -> Bool

  init(
    initialDraft: ProfileEditorDraft,
    onSave: @escaping (ProfileEditorDraft) async -> Bool
  ) {
    _draft = State(initialValue: initialDraft)
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

  var body: some View {
    NavigationStack {
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
        Section("Connection") {
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
        Section("TLS") {
          Picker("Mode", selection: $draft.tlsMode) {
            Text("Off").tag("off")
            Text("Verify CA").tag("verify_ca")
            Text("Verify full").tag("verify_full")
          }
        }
        Section("SSH Tunnel") {
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
                  safety: "read_only", timeoutMs: 5_000, runOnReconnect: false)))
          }
          .accessibilityIdentifier("profile.editor.startup.add")
          Text(
            "Commands run in listed order. Read-only commands may auto-run. Write and dangerous commands always wait for separate review."
          )
          .font(.caption)
          .foregroundStyle(.secondary)
        }
      }
      .formStyle(.grouped)
      .navigationTitle(draft.idBytes == nil ? "New Connection" : "Edit Connection")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") { dismiss() }
        }
        ToolbarItem(placement: .confirmationAction) {
          Button("Save") {
            saving = true
            Task {
              if await onSave(draft) { dismiss() }
              saving = false
            }
          }
          .accessibilityIdentifier("profile.editor.save")
          .keyboardShortcut(.defaultAction)
          .disabled(!canSave || saving)
        }
      }
    }
    .frame(minWidth: 520, minHeight: 620)
    .interactiveDismissDisabled(saving)
  }
}
