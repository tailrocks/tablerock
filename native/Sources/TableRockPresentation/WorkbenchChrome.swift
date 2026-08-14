import SwiftUI
import TableRockFeature

/// Connected workbench: context strip · tabs · content · status bar.
/// Content dominates; chrome is dense and non-marketing.
struct WorkbenchShellView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    HStack(spacing: 0) {
      VStack(spacing: 0) {
        WorkbenchContextStrip()
          .background(.bar)
        Divider()
        QueryTabStrip()
        Group {
          if model.queryWorkbenchSelected {
            QueryWorkbenchView()
          } else {
            ObjectWorkbenchView()
          }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        Divider()
        WorkbenchStatusBar()
      }

      if let tab = model.selectedObjectTab,
        tab.selectedSection == "structure"
      {
        Divider()
        NativeStructureInspector(tab: tab)
          .frame(
            minWidth: WorkbenchResponsiveLayout.inspectorMinimumWidth,
            idealWidth: WorkbenchResponsiveLayout.inspectorIdealWidth,
            maxWidth: WorkbenchResponsiveLayout.inspectorIdealWidth
          )
      } else if let snapshot = model.selectedCellSnapshot {
        Divider()
        NativeValueInspector(
          column: snapshot.0,
          cell: snapshot.1,
          row: snapshot.2,
          columnIndex: snapshot.3
        )
        .frame(
          minWidth: WorkbenchResponsiveLayout.inspectorMinimumWidth,
          idealWidth: WorkbenchResponsiveLayout.inspectorIdealWidth,
          maxWidth: WorkbenchResponsiveLayout.inspectorIdealWidth
        )
      }
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
    HStack(spacing: 8) {
      Button {
        Task { await model.showQuickSwitcher() }
      } label: {
        HStack(spacing: 7) {
          Image(systemName: engineSymbol(model.connectedEngine))
            .foregroundStyle(.blue)
            .accessibilityHidden(true)
          VStack(alignment: .leading, spacing: 0) {
            Text(model.activeProfile?.name ?? model.connectedEngine)
              .font(.caption.weight(.semibold))
              .foregroundStyle(Color.white)
              .background(Color.black)
            Text(model.activeProfile?.context ?? model.connectedEngine.uppercased())
              .font(.caption.weight(.bold))
              .statusMetricStyle()
          }
          if model.activeProductionWarning {
            Text("PRODUCTION")
              .font(.caption2.weight(.bold))
              .foregroundStyle(.orange)
          }
        }
        .padding(.horizontal, 8)
        .frame(height: 32)
        .foregroundStyle(.primary)
      }
      .buttonStyle(.glass)
      .help("Switch database connection")
      .accessibilityIdentifier("workbench.context.connection")
      .accessibilityLabel(
        "Database context, \(model.activeProfile?.name ?? model.connectedEngine)"
      )

      if model.isCatalogRefreshing, model.catalogSnapshot != nil {
        ProgressView()
          .controlSize(.small)
          .accessibilityLabel("Refreshing catalog")
      }
      Spacer(minLength: 0)

      Button {
        Task { await model.browse() }
      } label: {
        Image(systemName: "arrow.clockwise")
      }
      .buttonStyle(.glass)
      .help("Refresh catalog")
      .disabled(model.isRunning || model.isCatalogRefreshing)
      .accessibilityIdentifier("workbench.catalog.refresh")

      Button {
        model.addQueryTab()
      } label: {
        Image(systemName: "plus.rectangle.on.rectangle")
      }
      .buttonStyle(.glass)
      .help("New query")
      .accessibilityLabel("New query")

      Button {
        Task { await model.presentHistory() }
      } label: {
        Image(systemName: "clock.arrow.circlepath")
      }
      .buttonStyle(.glass)
      .help("Query history")
      .accessibilityLabel("Query history")

      Divider().frame(height: 20)
      EnvironmentSafetyBadge(model: model)
    }
    .padding(.horizontal, 10)
    .frame(height: 48)
    .controlSize(.small)
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("workbench.context")
  }

  private func engineSymbol(_ engine: String) -> String {
    switch engine.lowercased() {
    case "postgresql": "cylinder.split.1x2"
    case "clickhouse": "chart.bar.xaxis"
    case "redis": "square.stack.3d.up.fill"
    case "sqlite": "internaldrive"
    default: "cylinder"
    }
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
    Group {
      if !model.queryWorkbenchSelected, let tab = model.selectedObjectTab,
        !tab.kind.hasPrefix("redis_key_")
      {
        objectStatus(tab)
      } else {
        HStack(spacing: 8) {
          Text(operationWord)
            .font(.caption2.weight(.bold).monospaced())
            .statusMetricStyle()
            .accessibilityIdentifier("workbench.status.operation")
          Label("Details", systemImage: "info.circle")
            .font(.title3.weight(.heavy).monospacedDigit())
            .statusMetricStyle()
            .lineLimit(1)
            .help(factLine)
            .accessibilityIdentifier("workbench.status.facts")
            .accessibilityLabel("Workbench status details")
            .accessibilityValue(factLine)
          Spacer(minLength: 0)
          changeStatus
          if model.activeProductionWarning {
            Text("HALO PRODUCTION")
              .font(.caption2.weight(.bold))
              .statusMetricStyle()
              .accessibilityLabel("Production environment")
          }
        }
      }
    }
    .font(.caption)
    .padding(.horizontal, 10)
    .frame(height: 38)
    .background(Color(nsColor: .windowBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("workbench.status")
    .accessibilityLabel("Workbench status \(operationWord), \(factLine)")
  }

  private func objectStatus(_ tab: NativeObjectTab) -> some View {
    HStack(spacing: 12) {
      Picker(
        "Object section",
        selection: Binding(
          get: { tab.selectedSection },
          set: { section in
            tab.selectedSection = section
            if section == "structure" {
              Task { await model.loadObjectStructure() }
            }
          }
        )
      ) {
        Text("Data").tag("data")
        Text("Structure").tag("structure")
      }
      .labelsHidden()
      .pickerStyle(.segmented)
      .frame(width: 136)
      .accessibilityIdentifier("object.section")

      Divider().frame(height: 16)
      Label("\(tab.resultTable?.rows.count ?? 0) rows", systemImage: "list.number")
        .font(.title3.weight(.heavy))
        .statusMetricStyle()
        .monospacedDigit()
      if let summary = tab.summary {
        Label("Details", systemImage: "info.circle")
          .font(.title3.weight(.heavy).monospacedDigit())
          .statusMetricStyle()
          .lineLimit(1)
          .help(summary)
          .accessibilityLabel("Object summary")
          .accessibilityValue(summary)
      }
      if tab.isRunning, tab.resultTable != nil || tab.redisView != nil {
        ProgressView()
          .controlSize(.small)
          .accessibilityLabel("Loading rows")
      }
      Spacer(minLength: 0)
      Label(
        "\(tab.filters.count + (tab.rawWhere == nil ? 0 : 1)) filters",
        systemImage: "line.3.horizontal.decrease"
      )
      .font(.title3.weight(.heavy))
      .statusMetricStyle()
      Label("\(tab.resultTable?.columns.count ?? 0) columns", systemImage: "rectangle.split.3x1")
        .font(.title3.weight(.heavy))
        .statusMetricStyle()
      HStack(spacing: 3) {
        Button(action: {}) { Image(systemName: "chevron.left") }
          .buttonStyle(.plain)
          .disabled(true)
          .accessibilityLabel("Previous page")
        Text(tab.nextStartRow == nil ? "1 / 1" : "1 / …")
          .font(.title3.weight(.heavy))
          .statusMetricStyle()
          .monospacedDigit()
        Button {
          Task { await model.loadMoreObjectRows() }
        } label: {
          Image(systemName: "chevron.right")
        }
        .buttonStyle(.plain)
        .disabled(tab.nextStartRow == nil || tab.isRunning)
        .accessibilityLabel("Load next page")
        .accessibilityIdentifier("object.next-page")
      }
      changeStatus
    }
  }

  @ViewBuilder
  private var changeStatus: some View {
    if model.changeReviewOpen {
      Button {
        model.presentActiveReview()
      } label: {
        Label(
          counted(max(model.changeLedgerEntryCount, 1), "change").uppercased(),
          systemImage: "circle.fill"
        )
      }
      .buttonStyle(.plain)
      .foregroundStyle(.orange)
      .font(.caption2.weight(.semibold))
      .accessibilityIdentifier("workbench.status.ledger")
      .accessibilityLabel(
        "Change Ledger \(max(model.changeLedgerEntryCount, 1)) entries, review open")
    } else {
      Label("NO CHANGES", systemImage: "checkmark")
        .font(.caption2.weight(.semibold))
        .statusMetricStyle()
    }
  }
}

struct StatusMetricStyle: ViewModifier {
  func body(content: Content) -> some View {
    content
      .foregroundStyle(Color(nsColor: .windowBackgroundColor))
      .padding(.horizontal, 6)
      .padding(.vertical, 2)
      .background(Color(nsColor: .labelColor), in: Capsule())
  }
}

extension View {
  func statusMetricStyle() -> some View {
    modifier(StatusMetricStyle())
  }
}

/// Environment Halo: production, staging, and development remain unmistakable
/// without relying on color alone.
struct EnvironmentSafetyBadge: View {
  @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiateWithoutColor
  @Environment(\.colorSchemeContrast) private var colorSchemeContrast

  let model: WorkbenchPresentationStore

  var body: some View {
    if let environment = model.activeEnvironmentLabel,
      let safety = model.activeSafetyLabel
    {
      let isProduction =
        model.activeProductionWarning
        || environment.caseInsensitiveCompare("production") == .orderedSame
      let isStaging = environment.caseInsensitiveCompare("staging") == .orderedSame
      let haloWord: String = {
        if isProduction { return "PRODUCTION" }
        if isStaging { return "STAGING" }
        return environment.uppercased()
      }()
      let safetyWord = safety == "Read only" ? "SAFE MODE" : "CONFIRM WRITES"
      HStack(spacing: 4) {
        Image(
          systemName: isProduction
            ? "exclamationmark.triangle.fill"
            : isStaging ? "flag.fill" : safety == "Read only" ? "lock.fill" : "shield")
        Text(safetyWord)
      }
      .font(.caption2.weight(.semibold))
      .foregroundStyle(.primary)
      .padding(.horizontal, 7)
      .padding(.vertical, 3)
      .background(
        (isProduction ? Color.orange : Color.green).opacity(highContrast ? 0.18 : 0.11),
        in: .capsule
      )
      .overlay {
        if highContrast {
          Capsule().stroke(.primary, lineWidth: 1.5)
        }
      }
      .accessibilityElement(children: .combine)
      .accessibilityLabel(
        "Environment halo \(haloWord), \(environment), safety \(safety)"
      )
      .accessibilityIdentifier("environment.halo")
    }
  }

  private var highContrast: Bool {
    differentiateWithoutColor || colorSchemeContrast == .increased
  }
}
