import Foundation
import TableRockFeature

func connectedSessionLabel(_ session: String) -> String {
  "Connected · session \(session.prefix(16))…"
}

func counted(_ count: Int, _ singular: String) -> String {
  "\(count) \(singular)\(count == 1 ? "" : "s")"
}

func zeroizeTransientData(_ data: inout Data?) {
  guard var value = data else { return }
  value.resetBytes(in: 0..<value.count)
  data = value
}

extension Data {
  func hexEncodedString() -> String {
    map { String(format: "%02x", $0) }.joined()
  }
}

/// Mutable presentation form. The backend boundary receives a fresh immutable
/// value only when the operator saves or tests the form.
struct StartupActionEditorDraft: Identifiable {
  let id = UUID()
  var statement: String
  var safety: String
  var timeoutMs: UInt32
  var runOnReconnect: Bool

  init(_ value: WorkbenchStartupActionDraft) {
    statement = value.statement
    safety = value.safety
    timeoutMs = value.timeoutMs
    runOnReconnect = value.runOnReconnect
  }

  var workbench: WorkbenchStartupActionDraft {
    .init(
      statement: statement, safety: safety, timeoutMs: timeoutMs,
      runOnReconnect: runOnReconnect)
  }
}

struct ProfileEditorDraft {
  var idBytes: Data?
  var revision: UInt64
  var engine: String
  var name: String
  var group: String
  var environment: String
  var host: String
  var port: String
  var database: String
  var username: String
  var passwordSource: String
  var passwordValue: String
  var passwordReference: Data?
  var hasStoredPassword: Bool
  var plaintextAcknowledged: Bool
  var tlsMode: String
  var safetyMode: String
  var sshEnabled: Bool
  var sshHost: String
  var sshPort: String
  var sshUsername: String
  var sshAuthMode: String
  var sshPassword: String
  var sshPrivateKey: String
  var sshKnownHostsPath: String
  var sshHasStoredPassword: Bool
  var sshHasStoredPrivateKey: Bool
  var sshPlaintextAcknowledged: Bool
  var startupActions: [StartupActionEditorDraft]

  init(_ value: WorkbenchProfileDraft) {
    idBytes = value.idBytes
    revision = value.revision
    engine = value.engine
    name = value.name
    group = value.group
    environment = value.environment
    host = value.host
    port = value.port
    database = value.database
    username = value.username
    passwordSource = value.passwordSource
    passwordValue = value.passwordValue
    passwordReference = value.passwordReference
    hasStoredPassword = value.hasStoredPassword
    plaintextAcknowledged = value.plaintextAcknowledged
    tlsMode = value.tlsMode
    safetyMode = value.safetyMode
    sshEnabled = value.sshEnabled
    sshHost = value.sshHost
    sshPort = value.sshPort
    sshUsername = value.sshUsername
    sshAuthMode = value.sshAuthMode
    sshPassword = value.sshPassword
    sshPrivateKey = value.sshPrivateKey
    sshKnownHostsPath = value.sshKnownHostsPath
    sshHasStoredPassword = value.sshHasStoredPassword
    sshHasStoredPrivateKey = value.sshHasStoredPrivateKey
    sshPlaintextAcknowledged = value.sshPlaintextAcknowledged
    startupActions = value.startupActions.map(StartupActionEditorDraft.init)
  }

  var workbench: WorkbenchProfileDraft {
    .init(
      idBytes: idBytes, revision: revision, engine: engine, name: name,
      group: group, environment: environment, host: host, port: port,
      database: database, username: username, passwordSource: passwordSource,
      passwordValue: passwordValue, passwordReference: passwordReference,
      hasStoredPassword: hasStoredPassword,
      plaintextAcknowledged: plaintextAcknowledged,
      tlsMode: tlsMode, safetyMode: safetyMode,
      sshEnabled: sshEnabled, sshHost: sshHost, sshPort: sshPort,
      sshUsername: sshUsername, sshAuthMode: sshAuthMode, sshPassword: sshPassword,
      sshPrivateKey: sshPrivateKey, sshKnownHostsPath: sshKnownHostsPath,
      sshHasStoredPassword: sshHasStoredPassword,
      sshHasStoredPrivateKey: sshHasStoredPrivateKey,
      sshPlaintextAcknowledged: sshPlaintextAcknowledged,
      startupActions: startupActions.map(\.workbench)
    )
  }
}
