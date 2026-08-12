import SwiftUI
import TableRockFeature

/// Connected workbench: context strip · tabs · content · status bar.
/// Content dominates; chrome is dense and non-marketing.
struct WorkbenchShellView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    VStack(spacing: 0) {
      WorkbenchContextStrip()
      Divider()
      QueryTabStrip()
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
      Divider()
      Group {
        if model.queryWorkbenchSelected {
          QueryWorkbenchView()
        } else {
          ObjectWorkbenchView()
        }
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
      .padding(.horizontal, 10)
      .padding(.top, 8)
      Divider()
      WorkbenchStatusBar()
    }
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("workbench.shell")
    .accessibilityLabel("Workbench")
  }
}

/// Disconnected detail: quiet invite + direct connect (not a workbench chrome).
struct WorkbenchWelcomeView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 12) {
      Text("TableRock")
        .font(.title2.weight(.semibold))
        .tracking(-0.2)
      Text(model.status)
        .font(.subheadline)
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("app.status")
        .accessibilityValue(model.status)
      if let outcome = model.profileActionOutcome {
        Text(outcome)
          .foregroundStyle(.secondary)
          .font(.callout)
          .accessibilityIdentifier("profile.action.outcome")
      }
      if let bridgeError = model.bridgeError {
        Text(bridgeError)
          .foregroundStyle(.red)
          .font(.callout)
          .textSelection(.enabled)
      }
      GroupBox("New connection") {
        Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 6) {
          GridRow {
            Text("Engine")
            Picker("", selection: $model.formEngine) {
              Text("PostgreSQL").tag("postgresql")
              Text("ClickHouse").tag("clickhouse")
              Text("Redis").tag("redis")
              Text("SQLite").tag("sqlite")
            }
            .labelsHidden()
          }
          GridRow {
            Text(model.formEngine == "sqlite" ? "Path" : "Host")
            TextField(
              model.formEngine == "sqlite" ? "/absolute/path.db" : "127.0.0.1",
              text: $model.formHost)
          }
          GridRow {
            Text("Port")
            TextField(model.formEngine == "sqlite" ? "1" : "5432", text: $model.formPort)
              .disabled(model.formEngine == "sqlite")
          }
          GridRow {
            Text(model.formEngine == "sqlite" ? "File" : "Database")
            TextField(
              model.formEngine == "sqlite" ? "main" : "postgres",
              text: $model.formDatabase)
          }
          GridRow {
            Text("User")
            TextField("postgres", text: $model.formUser)
          }
          GridRow {
            Text("Password")
            SecureField("", text: $model.formPassword)
          }
        }
        HStack {
          Button("Connect") { Task { await model.connectByParams() } }
            .buttonStyle(.glassProminent)
            .accessibilityIdentifier("connection.direct.connect")
          Spacer()
        }
        .padding(.top, 4)
      }
      if let name = model.connectingName {
        Text("Connecting to \(name)…").foregroundStyle(.secondary)
      }
      if let connectError = model.connectError {
        Text(connectError)
          .foregroundStyle(.red)
          .font(.callout)
          .textSelection(.enabled)
      }
      Spacer(minLength: 0)
      Text("Select a connection or try Sample — connect → catalog → query")
        .font(.caption)
        .foregroundStyle(.tertiary)
    }
    .padding(20)
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    .accessibilityIdentifier("workbench.welcome")
  }
}

/// Dense context facts (spec context bar). Halo stays non-color text.
struct WorkbenchContextStrip: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    HStack(spacing: 10) {
      Label {
        Text(model.activeProfile?.name ?? model.connectedEngine)
          .font(.subheadline.weight(.semibold))
      } icon: {
        Image(systemName: "bolt.horizontal.fill")
      }
      .accessibilityIdentifier("workbench.context.connection")
      Text(model.connectedEngine.uppercased())
        .font(.caption.weight(.semibold).monospaced())
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("workbench.context.engine")
      if let session = model.sessionHex {
        Text(connectedSessionLabel(session))
          .font(.caption.monospaced())
          .foregroundStyle(.secondary)
          .textSelection(.enabled)
          .accessibilityIdentifier("connection.status")
          .accessibilityValue(connectedSessionLabel(session))
      }
      EnvironmentSafetyBadge(model: model)
      if model.isCatalogRefreshing {
        ProgressView()
          .controlSize(.small)
          .accessibilityLabel("Refreshing catalog")
      }
      Spacer(minLength: 0)
      Button {
        Task { await model.browse() }
      } label: {
        Label("Catalog", systemImage: "arrow.clockwise")
      }
      .buttonStyle(.glass)
      .controlSize(.small)
      .disabled(model.isRunning || model.isCatalogRefreshing)
      .accessibilityIdentifier("workbench.catalog.refresh")
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 6)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("workbench.context")
  }
}

/// Permanent status bar: operation · rows · health words (never color alone).
struct WorkbenchStatusBar: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  private var operationWord: String {
    if model.isRunning { return "RUNNING" }
    if model.isCatalogRefreshing { return "CATALOG" }
    if model.connectError != nil || model.catalogError != nil { return "ERROR" }
    if model.queryError != nil { return "QUERY ERROR" }
    return "READY"
  }

  private var factLine: String {
    WorkbenchStatusFacts.line(
      operation: operationWord,
      engine: model.connectedEngine,
      querySummary: model.querySummary,
      queryError: model.queryError,
      cancelOutcome: model.cancelOutcome,
      catalogSummary: model.catalogSummary,
      catalogError: model.catalogError,
      resultRowCount: model.resultTable?.rows.count,
      production: model.activeProductionWarning,
      ledgerEntryCount: model.changeLedgerEntryCount,
      reviewOpen: model.changeReviewOpen
    )
  }

  var body: some View {
    HStack(spacing: 8) {
      Text(operationWord)
        .font(.caption2.weight(.bold).monospaced())
        .accessibilityIdentifier("workbench.status.operation")
      Text(factLine)
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .lineLimit(1)
        .truncationMode(.middle)
        .textSelection(.enabled)
        .accessibilityIdentifier("workbench.status.facts")
        .accessibilityValue(factLine)
      Spacer(minLength: 0)
      if model.changeReviewOpen {
        Text("LEDGER \(max(model.changeLedgerEntryCount, 1))")
          .font(.caption2.weight(.bold).monospaced())
          .accessibilityIdentifier("workbench.status.ledger")
          .accessibilityLabel(
            "Change Ledger \(max(model.changeLedgerEntryCount, 1)) entries, review open")
      }
      if model.activeProductionWarning {
        Text("HALO PRODUCTION")
          .font(.caption2.weight(.bold))
          .accessibilityLabel("Production environment")
      }
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 5)
    .accessibilityElement(children: .combine)
    .accessibilityIdentifier("workbench.status")
    .accessibilityLabel("Workbench status \(operationWord), \(factLine)")
  }
}
