import AppKit
import Observation
import SwiftUI
import TableRockBridge
import TableRockFeature

private enum PresentationStoreError: Error {
  case unavailable(String)
}

@MainActor
@Observable
final class WorkbenchPresentationStore {
  let windowId: UUID
  var status: String = "starting…"
  var bridgeError: String?
  var profiles: [WorkbenchProfileItem] = []
  var profileGroups: [WorkbenchProfileGroup] = []
  var collapsedProfileGroups: Set<String> = []
  var profileSearch = ""
  private(set) var profilesLoading = false
  private(set) var profilesError: String?
  private var profileSearchGeneration: UInt64 = 0
  var editorDraft: ProfileEditorDraft?
  var profileActionError: String?
  var profileActionOutcome: String?
  var pendingRemoval: WorkbenchProfileItem?
  var groupDialog: ProfileGroupDialog?
  var passwordPrompt: ProfilePasswordPrompt?
  var connectionUrlImport: ConnectionUrlImport?
  var externalUrlReview: ExternalUrlReview?
  var quickSwitcherPresented = false
  var quickSwitcherSearch = ""
  var explainPresented = false
  var externalUrlFixtureConsumed = false
  var pendingGroupRemoval: String?
  var profileSections: [ProfileSection] {
    var order = profileGroups.map(\.name)
    let alphabetical = Dictionary(
      uniqueKeysWithValues: profileGroups.map { ($0.name, $0.alphabetical) }
    )
    var grouped: [String: [WorkbenchProfileItem]] = [:]
    for profile in profiles {
      let group = profile.group ?? ""
      if !group.isEmpty && !order.contains(group) { order.append(group) }
      grouped[group, default: []].append(profile)
    }
    if grouped[""] != nil { order.append("") }
    if !profileSearch.isEmpty { order.removeAll { grouped[$0]?.isEmpty != false } }
    return order.map { group in
      var profiles = grouped[group] ?? []
      if alphabetical[group] == true {
        profiles.sort {
          if $0.favorite != $1.favorite { return $0.favorite && !$1.favorite }
          return $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending
        }
      }
      return ProfileSection(
        id: group.isEmpty ? "ungrouped" : group,
        title: group.isEmpty ? "Ungrouped" : group,
        profiles: profiles,
        alphabetical: alphabetical[group] ?? false
      )
    }
  }
  var sessionHex: String?
  var connectError: String?
  var connectingName: String?
  var sessionHealth: WorkbenchSessionHealth?
  var healthChecking = false
  var reconnectState: String?
  var reconnectGeneration: UInt64 = 0
  var historyPresented = false
  var historySearch = ""
  var historyItems: [WorkbenchHistoryItem] = []
  private(set) var historyLoading = false
  private(set) var historyError: String?
  var historyRetention = "full"
  private var historyGeneration: UInt64 = 0
  var savedQueriesPresented = false
  var savedQuerySearch = ""
  var savedQueryEngine = ""
  var savedQueries: [WorkbenchSavedQueryItem] = []
  private(set) var savedQueriesLoading = false
  private(set) var savedQueriesError: String?
  private var savedQueriesGeneration: UInt64 = 0
  var saveQueryDialog = false
  var savedQueryName = ""
  var pendingSavedQueryRemoval: WorkbenchSavedQueryItem?
  var csvImportPresented = false
  var csvImportPreview: WorkbenchCSVImportPreview?
  var csvImportMappedColumns: [String] = []
  var csvImportColumnTypes: [String] = []
  var csvImportReview: WorkbenchCSVImportReview?
  var csvImportError: String?
  var csvImportOutcome: String?
  var csvImportProgress: WorkbenchCSVImportProgress?
  var csvImportErrorCopyOutcome: String?
  var csvImportApplying = false
  private var csvImportUrl: URL?
  private var csvImportOperationId: Data?
  var streamExportPresented = false
  var streamExportProgress: WorkbenchStreamExportProgress?
  var streamExportError: String?
  private var streamExportOperationId: Data?
  var redisOverviewPresented = false
  var redisOverview: WorkbenchRedisOverview?
  private(set) var redisOverviewLoading = false
  private(set) var redisOverviewError: String?
  var redisSubscriptionPresented = false
  var redisSubscriptionSelector = ""
  var redisSubscriptionPattern = false
  private(set) var redisSubscriptionStatus: WorkbenchRedisSubscriptionStatus?
  private(set) var redisSubscriptionError: String?
  private(set) var redisSubscriptionStarting = false
  private var redisSubscriptionPollTask: Task<Void, Never>?
  var ddlChangePresented = false
  var ddlChangeKind = "add_column"
  var ddlChangeObjectName = ""
  var ddlChangeDefinition = ""
  var ddlChangeReview: WorkbenchDdlChangeReview?
  var ddlChangeOutcome: String?
  var ddlChangeError: String?
  private(set) var ddlChangeApplying = false
  private var ddlChangeCatalogNodeId: Data?
  /// Edit-safety probe held behind explicit Change Review (not silent apply).
  var probeChangeReview: ProbeChangeReviewState?
  var probeChangePresented = false
  var probeChangeOutcome: String?
  var probeChangeError: String?
  var probeChangeApplying = false
  var tableOperationPresented = false
  var tableOperationKind = "truncate"
  var tableOperationNewName = ""
  var tableOperationConfirmation = ""
  var tableOperationReview: WorkbenchTableOperationReview?
  var tableOperationStatus: WorkbenchTableOperationStatus?
  var tableOperationOutcome: String?
  var tableOperationError: String?
  private(set) var tableOperationApplying = false
  private var tableOperationCatalogNodeId: Data?
  private var tableOperationId: Data?
  var findReplacePresented = false
  var findPattern = ""
  var findReplacement = ""
  var findMode = "literal"
  var findScope = "document"
  var findStatus: String?
  var findError: String?
  var queryParametersPresented = false
  var queryParameterBindings: [WorkbenchQueryParameter] = []
  var queryParameterError: String?
  var parameterizedStatement: String?
  var postgresActivityPresented = false
  var postgresActivityRows: [WorkbenchPostgresActivityRow] = []
  private(set) var postgresActivityLoading = false
  private(set) var postgresActivityError: String?
  var postgresActivityOutcome: String?
  var postgresRelationshipsPresented = false
  var postgresRelationshipSnapshot: WorkbenchRelationshipSnapshot?
  private(set) var postgresRelationshipsLoading = false
  private(set) var postgresRelationshipsError: String?
  var postgresRolesPresented = false
  var postgresRoleSnapshot: WorkbenchRoleSnapshot?
  var postgresRoleSearch = ""
  private(set) var postgresRolesLoading = false
  private(set) var postgresRolesError: String?
  var postgresRoleChangeKind = "grant_membership"
  var postgresRoleChangeRole = ""
  var postgresRoleChangeSubject = ""
  var postgresRoleChangePrivilege = "SELECT"
  var postgresRoleChangeReview: WorkbenchRoleChangeReview?
  var postgresRoleChangeOutcome: String?
  var postgresToolsPresented = false
  var postgresToolKind = "dump"
  var postgresToolContent = "all"
  var postgresToolClean = false
  var postgresToolNoOwner = false
  var postgresToolExplicitPath = ""
  var postgresToolProbe: WorkbenchPostgresToolProbe?
  var postgresToolFileUrl: URL?
  var postgresToolStatus: WorkbenchPostgresToolStatus?
  var postgresToolError: String?
  var postgresToolReviewRequested = false
  private var postgresToolSecurityScopeActive = false
  var queryTabs: [NativeQueryTab]
  var selectedQueryTabId: UUID
  var objectTabs: [NativeObjectTab] = []
  var selectedObjectTabId: UUID?
  var selectedWorkbenchKind = "query"
  var pendingQueryTabClose: NativeQueryTab?
  var queryTabRename: NativeQueryTab?
  var queryTabRenameText = ""
  var activeProfileId: Data?
  var activeProfile: WorkbenchProfileItem? {
    guard let activeProfileId else { return nil }
    return profiles.first(where: { $0.idBytes == activeProfileId })
  }
  var activeEnvironmentLabel: String? {
    guard let environment = activeProfile?.environment, !environment.isEmpty else { return nil }
    return switch environment {
    case "production": "Production"
    case "staging": "Staging"
    case "development": "Development"
    case "testing": "Testing"
    default: environment
    }
  }
  var activeSafetyLabel: String? {
    guard let safety = activeProfile?.safetyMode else { return nil }
    return safety == "read_only" ? "Read only" : "Confirm writes"
  }
  var activeProductionWarning: Bool { activeProfile?.productionWarning == true }

  /// Presentation clock for expiry facts (Rust tokens still own authority).
  func nowMilliseconds() -> UInt64 {
    dependencies.clock.nowMilliseconds()
  }

  /// Pending ledger entries visible to presentation (probe review open counts as 1).
  var changeLedgerEntryCount: Int {
    var n = 0
    if probeChangeReview != nil { n += ChangeReviewPresentation.probeLedgerCount }
    if ddlChangeReview != nil { n += 1 }
    if tableOperationReview != nil { n += 1 }
    if csvImportReview != nil { n += 1 }
    if postgresRoleChangeReview != nil { n += 1 }
    return n
  }

  var changeReviewOpen: Bool {
    probeChangeReview != nil || ddlChangeReview != nil || tableOperationReview != nil
      || csvImportReview != nil || postgresRoleChangeReview != nil
  }
  var activeQueryTab: NativeQueryTab {
    queryTabs.first(where: { $0.id == selectedQueryTabId }) ?? queryTabs[0]
  }
  var activeExplainPlan: String? { activeQueryTab.explainPlan }
  var activeQueryTabForPresentation: NativeQueryTab { activeQueryTab }
  var activeObjectTab: NativeObjectTab? {
    guard let selectedObjectTabId else { return nil }
    return objectTabs.first(where: { $0.id == selectedObjectTabId })
  }
  var selectedObjectTab: NativeObjectTab? { activeObjectTab }
  var sqlInsertCopyAvailable: Bool {
    guard let kind = activeObjectTab?.kind else { return false }
    return [
      "postgresql_table", "postgresql_foreign_table",
      "postgresql_partitioned_table", "clickhouse_table",
    ].contains(kind)
  }
  var canEditSelectedStructure: Bool {
    guard connectedEngine == "postgresql", activeObjectTab?.structure != nil,
      let kind = activeObjectTab?.kind
    else { return false }
    return ["postgresql_table", "postgresql_partitioned_table"].contains(kind)
  }
  var canOperateSelectedTable: Bool {
    guard let kind = activeObjectTab?.kind else { return false }
    return ["postgresql_table", "postgresql_partitioned_table", "clickhouse_table"]
      .contains(kind)
  }
  var selectedCell: NativeCellSelection? {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.selectedCell : activeQueryTab.selectedCell
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.selectedCell = newValue
      } else {
        activeQueryTab.selectedCell = newValue
      }
    }
  }
  var selectedCellSnapshot: (WorkbenchColumn, WorkbenchCell, Int, Int)? {
    _ = queryStateRevision
    guard let table = resultTable, let selection = selectedCell,
      table.columnMetadata.indices.contains(selection.column),
      table.cells.indices.contains(selection.row),
      table.cells[selection.row].indices.contains(selection.column)
    else { return nil }
    return (
      table.columnMetadata[selection.column],
      table.cells[selection.row][selection.column],
      selection.row, selection.column
    )
  }
  var loadedRowQuickFilter: String {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.quickFilter ?? "" : activeQueryTab.quickFilter
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.quickFilter = newValue
      } else {
        activeQueryTab.quickFilter = newValue
      }
      selectedCell = nil
    }
  }
  var resultSort: [WorkbenchBrowseSort] {
    selectedWorkbenchKind == "object" ? activeObjectTab?.sort ?? [] : []
  }
  var copyOutcome: String? {
    get {
      selectedWorkbenchKind == "object" ? activeObjectTab?.copyOutcome : activeQueryTab.copyOutcome
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.copyOutcome = newValue
      } else {
        activeQueryTab.copyOutcome = newValue
      }
    }
  }
  var copyError: String? {
    get {
      selectedWorkbenchKind == "object" ? activeObjectTab?.copyError : activeQueryTab.copyError
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.copyError = newValue
      } else {
        activeQueryTab.copyError = newValue
      }
    }
  }
  func selectCell(row: Int, column: Int) {
    selectedCell = NativeCellSelection(row: row, column: column)
    // Continuum stays closed until explicit open — selection alone must not navigate.
    queryStateRevision &+= 1
  }

  // MARK: - Row Continuum (presentation prototype; fixture neighbors until Rust contract)

  /// Presentation-only continuum plane. Never invents live DB truth beyond fixtures.
  struct RelationContinuumState: Equatable {
    var edgeTitle: String
    var directionWord: String
    var fromColumn: String
    var fromValue: String
    var relatedSchema: String
    var relatedTable: String
    var relatedColumn: String
    var columns: [String]
    var rows: [[String]]
    /// Always true until `relation_neighbors` ships in Rust.
    var usesFixtureData: Bool
    var statusWord: String
  }

  var relationContinuum: RelationContinuumState?
  var relationContinuumError: String?

  var canOpenRelationContinuum: Bool {
    guard sessionHex != nil, let snap = selectedCellSnapshot else { return false }
    return RelationContinuumFixtures.edge(forColumn: snap.0.name) != nil
  }

  func openRelationContinuumFromSelection() {
    relationContinuumError = nil
    guard sessionHex != nil else {
      relationContinuumError = "DISCONNECTED — connect before Continuum"
      relationContinuum = nil
      return
    }
    guard let snap = selectedCellSnapshot else {
      relationContinuumError = "Select a cell that participates in a relation"
      relationContinuum = nil
      return
    }
    let column = snap.0.name
    let value = snap.1.display
    guard let edge = RelationContinuumFixtures.edge(forColumn: column) else {
      relationContinuumError = "No Continuum edge for column \(column) (fixture map)"
      relationContinuum = nil
      return
    }
    let neighbors = RelationContinuumFixtures.neighbors(
      edge: edge, value: value)
    relationContinuum = RelationContinuumState(
      edgeTitle: "\(edge.fromTable).\(edge.fromColumn) → \(edge.toTable).\(edge.toColumn)",
      directionWord: "outbound",
      fromColumn: column,
      fromValue: value,
      relatedSchema: edge.toSchema,
      relatedTable: edge.toTable,
      relatedColumn: edge.toColumn,
      columns: neighbors.columns,
      rows: neighbors.rows,
      usesFixtureData: true,
      statusWord: neighbors.rows.isEmpty ? "EMPTY" : "READY"
    )
    queryStateRevision &+= 1
  }

  func closeRelationContinuum() {
    relationContinuum = nil
    relationContinuumError = nil
    queryStateRevision &+= 1
  }
  var queryWorkbenchSelected: Bool { selectedWorkbenchKind == "query" }
  var hasRunningWorkbench: Bool {
    queryTabs.contains(where: \.isRunning) || objectTabs.contains(where: \.isRunning)
      || redisSubscriptionIsActive
  }
  var redisSubscriptionIsActive: Bool {
    guard let phase = redisSubscriptionStatus?.phase else { return false }
    return phase == "connecting" || phase == "listening" || phase == "cancel_requested"
  }
  var sqlFile: WorkbenchSQLFile? {
    get { activeQueryTab.sqlFile }
    set { activeQueryTab.sqlFile = newValue }
  }
  private var sqlFileBaseline: String {
    get { activeQueryTab.sqlFileBaseline }
    set { activeQueryTab.sqlFileBaseline = newValue }
  }
  var confirmDiscardForOpen = false
  var confirmExternalOverwrite = false
  private(set) var sqlFileError: String? {
    get { activeQueryTab.sqlFileError }
    set { activeQueryTab.sqlFileError = newValue }
  }
  var catalogSummary: String?
  var catalogError: String?
  var catalogSnapshot: [WorkbenchCatalogNode]?
  var catalogRefreshState: CatalogRefreshState = .idle
  var isCatalogRefreshing: Bool {
    if case .loading = catalogRefreshState { true } else { false }
  }
  var resultTable: WorkbenchTable? {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.resultTable : activeQueryTab.resultTable
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.resultTable = newValue
      } else {
        activeQueryTab.resultTable = newValue
      }
    }
  }
  var catalogSelection: String?
  var writeOutcome: String? {
    get { activeQueryTab.writeOutcome }
    set { activeQueryTab.writeOutcome = newValue }
  }
  var isRunning: Bool {
    get {
      _ = queryStateRevision
      return selectedWorkbenchKind == "object"
        ? activeObjectTab?.isRunning == true : activeQueryTab.isRunning
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.isRunning = newValue
      } else {
        activeQueryTab.isRunning = newValue
      }
      queryStateRevision &+= 1
    }
  }
  var cancelOutcome: String? {
    get {
      _ = queryStateRevision
      return activeQueryTab.cancelOutcome
    }
    set {
      activeQueryTab.cancelOutcome = newValue
      queryStateRevision &+= 1
    }
  }
  // Pagination state for the current result (fetch_page).
  var resultIdData: Data? {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.resultIdData : activeQueryTab.resultIdData
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.resultIdData = newValue
      } else {
        activeQueryTab.resultIdData = newValue
      }
    }
  }
  var resultRevision: UInt64 {
    get {
      selectedWorkbenchKind == "object"
        ? activeObjectTab?.resultRevision ?? 0 : activeQueryTab.resultRevision
    }
    set {
      if selectedWorkbenchKind == "object" {
        activeObjectTab?.resultRevision = newValue
      } else {
        activeQueryTab.resultRevision = newValue
      }
    }
  }
  var nextStartRow: UInt64? {
    get { activeQueryTab.nextStartRow }
    set { activeQueryTab.nextStartRow = newValue }
  }
  var connectedEngine: String = ""
  var queryText: String {
    get { activeQueryTab.statementText }
    set { activeQueryTab.statementText = newValue }
  }
  var queryEditorSelection: NSRange {
    get { activeQueryTab.editorSelection }
    set { activeQueryTab.editorSelection = newValue }
  }
  var reviewOutcome: String? {
    get { activeQueryTab.reviewOutcome }
    set { activeQueryTab.reviewOutcome = newValue }
  }
  var reviewError: String? {
    get { activeQueryTab.reviewError }
    set { activeQueryTab.reviewError = newValue }
  }
  var querySummary: String? {
    get { activeQueryTab.querySummary }
    set { activeQueryTab.querySummary = newValue }
  }
  var queryError: String? {
    get { activeQueryTab.queryError }
    set { activeQueryTab.queryError = newValue }
  }
  // Direct-connect form (no saved profile required).
  var formEngine: String = "postgresql"
  var formHost: String = "127.0.0.1"
  var formPort: String = "5432"
  var formDatabase: String = "postgres"
  var formUser: String = "postgres"
  var formPassword: String = ""
  let client: (any WorkbenchBackend)?
  private let startupError: String?
  let dependencies: AppDependencies
  let fixtures: NativeWorkbenchFixtureConfiguration
  /// Operator data root used for sample SQLite + isolation.
  let dataRootPath: String
  var sessionData: Data?
  var queryStateRevision: UInt64 = 0

  init(
    client: (any WorkbenchBackend)? = nil,
    startupError: String? = nil,
    windowId: UUID? = nil,
    dependencies: AppDependencies = AppDependencies(),
    dataRootPath: String = FileManager.default.temporaryDirectory.path,
    fixtures: NativeWorkbenchFixtureConfiguration = .current
  ) {
    self.client = client
    self.startupError = startupError
    self.dependencies = dependencies
    self.fixtures = fixtures
    self.dataRootPath = dataRootPath
    self.windowId = windowId ?? dependencies.identifiers.next()
    let tab = NativeQueryTab(
      id: dependencies.identifiers.next(), title: "Query 1", statementText: "SELECT 1;"
    )
    queryTabs = [tab]
    selectedQueryTabId = tab.id
    installPerformanceFixtureIfRequested()
  }

  func initialize() async {
    if fixtures.multiWindow {
      let other = WorkbenchPresentationStore(client: client, dependencies: dependencies, fixtures: fixtures)
      other.queryText = "SELECT second_window;"
      other.sessionData = Data(repeating: 9, count: 16)
      guard other.windowId != windowId, sharesBridge(with: other),
        queryText == "SELECT 1;", other.queryText == "SELECT second_window;",
        sessionData == nil, other.sessionData != nil,
        queryTabs[0] !== other.queryTabs[0]
      else {
        writePerformanceMetric("MULTI_WINDOW_PROOF_FAILED ownership mismatch")
        return
      }
      status = "Multi-window fixture"
      return
    }
    if fixtures.objectTabs {
      let node = WorkbenchCatalogNode(
        idBytes: Data(repeating: 7, count: 16), parentIdBytes: Data(repeating: 6, count: 16),
        depth: 2, name: "users", kind: "postgresql_table",
        childrenState: "not_applicable", expandable: false
      )
      let first = NativeObjectTab(
        id: dependencies.identifiers.next(), node: node, pinned: true
      )
      first.resultTable = WorkbenchTable(columns: ["id"], rows: [["1"]])
      let preview = NativeObjectTab(id: dependencies.identifiers.next(), node: node)
      preview.resultTable = WorkbenchTable(columns: ["id"], rows: [["2"]])
      objectTabs = [first, preview]
      selectedObjectTabId = preview.id
      selectedWorkbenchKind = "object"
      sessionData = Data(repeating: 11, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      selectQueryTab(queryTabs[0])
      selectObjectTab(preview)
      guard preview.pinned, first.catalogNodeId == preview.catalogNodeId,
        first.resultTable?.rows == [["1"]], preview.resultTable?.rows == [["2"]]
      else {
        writePerformanceMetric("OBJECT_TABS_PROOF_FAILED isolation mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      await loadObjectFilterPresets(preview)
      runNativeObjectTabsAudit()
      return
    }
    if fixtures.dataMovementUI {
      sessionData = Data(repeating: 1, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      let node = WorkbenchCatalogNode(
        idBytes: Data(repeating: 7, count: 16),
        parentIdBytes: Data(repeating: 6, count: 16), depth: 1,
        name: "fixture_table", kind: "postgresql_table",
        childrenState: "not_applicable", expandable: false)
      let tab = NativeObjectTab(id: dependencies.identifiers.next(), node: node, pinned: true)
      tab.resultTable = WorkbenchTable(
        columns: ["id", "name"], rows: [["1", "Ada"]])
      tab.resultIdData = Data(repeating: 8, count: 16)
      tab.resultRevision = 1
      tab.summary = "1 row · 2 columns"
      objectTabs = [tab]
      selectedObjectTabId = tab.id
      selectedWorkbenchKind = "object"
      status = "Data movement fixture"
      return
    }
    if fixtures.valueInspector {
      sessionData = Data(repeating: 4, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      let raw = Data(#"{"ok":true}"#.utf8)
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["payload"], rows: [[#"{"ok":true}"#]],
        columnMetadata: [
          WorkbenchColumn(
            name: "payload", engine: 0, engineType: "jsonb", nullable: true
          )
        ],
        cells: [
          [
            WorkbenchCell(
              display: #"{"ok":true}"#, kind: 8, truncation: 2,
              originalByteCount: 128, bytes: raw
            )
          ]
        ]
      )
      activeQueryTab.selectedCell = NativeCellSelection(row: 0, column: 0)
      status = "Value inspector fixture"
      guard selectedCellSnapshot?.0.engineType == "jsonb",
        selectedCellSnapshot?.1.kindLabel == "Structured",
        selectedCellSnapshot?.1.originalByteCount == 128
      else {
        writePerformanceMetric("VALUE_INSPECTOR_PROOF_FAILED model projection mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeValueInspectorAudit()
      return
    }
    if fixtures.selectableInspector {
      sessionData = Data(repeating: 5, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      let raw = Data(#"{"selected":true}"#.utf8)
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["payload"], rows: [[#"{"selected":true}"#]],
        columnMetadata: [
          WorkbenchColumn(name: "payload", engine: 0, engineType: "jsonb", nullable: false)
        ],
        cells: [
          [
            WorkbenchCell(
              display: #"{"selected":true}"#, kind: 8, truncation: 0,
              originalByteCount: UInt64(raw.count), bytes: raw)
          ]
        ])
      activeQueryTab.selectedCell = nil
      status = "Selectable inspector fixture"
      return
    }
    if fixtures.resultPaging {
      sessionData = Data(repeating: 5, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["n"], rows: (1...500).map { [String($0)] })
      activeQueryTab.resultIdData = Data(repeating: 8, count: 16)
      activeQueryTab.resultRevision = 1
      activeQueryTab.nextStartRow = 500
      activeQueryTab.querySummary = "result · 1 column · 500 rows loaded"
      status = "Result paging fixture"
      return
    }
    if fixtures.quickFilter {
      sessionData = Data(repeating: 5, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "postgresql"
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["id", "name"],
        rows: [["1", "Ada"], ["2", "Grace"], ["3", "Linus"]])
      activeQueryTab.querySummary = "result · 2 columns · 3 rows loaded"
      status = "Quick filter fixture"
      return
    }
    if fixtures.inputMethodEditor {
      activeQueryTab.statementText = "SELECT "
      status = "Preparing IME fixture"
      try? await Task.sleep(for: .milliseconds(500))
      guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView
      else {
        status = "IME fixture failed: no window"
        return
      }
      func descendants(of view: NSView) -> [NSView] {
        [view] + view.subviews.flatMap(descendants)
      }
      guard let editor = descendants(of: root).compactMap({ $0 as? NSTextView }).first else {
        status = "IME fixture failed: no editor"
        return
      }
      editor.window?.makeFirstResponder(editor)
      editor.setSelectedRange(NSRange(location: editor.string.utf16.count, length: 0))
      editor.setMarkedText(
        "かな", selectedRange: NSRange(location: 2, length: 0),
        replacementRange: NSRange(location: NSNotFound, length: 0))
      guard editor.hasMarkedText() else {
        status = "IME fixture failed: no marked text"
        return
      }
      let composed = editor.string
      activeQueryTab.statementText = "model update must not replace composition"
      try? await Task.sleep(for: .milliseconds(250))
      guard editor.hasMarkedText(), editor.string == composed else {
        status = "IME fixture failed: composition replaced"
        writePerformanceMetric("IME_PROOF_FAILED composition_replaced=true")
        return
      }
      status = "IME composition preserved"
      writePerformanceMetric("IME_PROOF_PASSED marked_text_survived_model_update=true")
      return
    }
    if fixtures.structure {
      guard let client else {
        writePerformanceMetric("STRUCTURE_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 5433,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "postgresql"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first,
          let schema = try await client.refreshCatalog(
            session: session, parentNodeId: database.idBytes
          ).first(where: { $0.name == "public" })
        else {
          writePerformanceMetric("STRUCTURE_PROOF_FAILED catalog hierarchy missing")
          return
        }
        let objects = try await client.refreshCatalog(
          session: session, parentNodeId: schema.idBytes
        )
        guard let object = objects.first(where: { $0.name == "structure_probe" }) else {
          writePerformanceMetric("STRUCTURE_PROOF_FAILED target missing")
          return
        }
        let tab = NativeObjectTab(
          id: dependencies.identifiers.next(), node: object, pinned: true
        )
        objectTabs = [tab]
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
        await loadObjectStructure()
        guard tab.structure?.columns.count == 3,
          tab.structure?.indexes.contains(where: { $0.name == "structure_probe_pkey" }) == true,
          tab.structure?.constraints.contains(where: { $0.name == "structure_probe_name_check" })
            == true
        else {
          writePerformanceMetric(
            "STRUCTURE_PROOF_FAILED \(tab.structureError ?? "snapshot mismatch")"
          )
          return
        }
        copyStructureDdl(tab.structure!.ddl)
        try? await Task.sleep(for: .milliseconds(500))
        runNativeStructureAudit()
      } catch {
        writePerformanceMetric("STRUCTURE_PROOF_FAILED \(error)")
      }
      return
    }
    if fixtures.clickHouseStructure {
      guard let client else {
        writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "clickhouse", host: "127.0.0.1", port: 8122,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "clickhouse"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first(where: { $0.name == "db" })
        else {
          writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED database missing")
          return
        }
        let objects = try await client.refreshCatalog(
          session: session, parentNodeId: database.idBytes
        )
        guard let object = objects.first(where: { $0.name == "structure_probe" }) else {
          writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED target missing")
          return
        }
        let tab = NativeObjectTab(
          id: dependencies.identifiers.next(), node: object, pinned: true
        )
        objectTabs = [tab]
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
        await loadObjectStructure()
        guard tab.structure?.engine == "clickhouse",
          tab.structure?.columns.count == 3,
          tab.structure?.columns.first(where: { $0.name == "id" })?.primaryKey == true,
          tab.structure?.columns.first(where: { $0.name == "id" })?.sortingKey == true,
          tab.structure?.facts.contains(where: {
            $0.name == "Engine" && $0.value == "MergeTree"
          }) == true
        else {
          writePerformanceMetric(
            "CLICKHOUSE_STRUCTURE_PROOF_FAILED \(tab.structureError ?? "snapshot mismatch")"
          )
          return
        }
        copyStructureDdl(tab.structure!.ddl)
        try? await Task.sleep(for: .milliseconds(500))
        runNativeClickHouseStructureAudit()
      } catch {
        writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED \(error)")
      }
      return
    }
    if fixtures.redisOverview {
      guard let client else {
        writePerformanceMetric("REDIS_OVERVIEW_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "redis", host: "127.0.0.1", port: 6380,
            database: "0", user: "", password: "", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "redis"
        await showRedisOverview()
        guard redisOverview?.sampledAtMs ?? 0 > 0,
          redisOverview?.lines.contains(where: {
            $0.hasPrefix("redis_version: ")
          }) == true
        else {
          writePerformanceMetric(
            "REDIS_OVERVIEW_PROOF_FAILED \(redisOverviewError ?? "snapshot missing")"
          )
          return
        }
        try? await Task.sleep(for: .milliseconds(500))
        runNativeRedisOverviewAudit(sampledAtMs: redisOverview?.sampledAtMs ?? 0)
      } catch {
        writePerformanceMetric("REDIS_OVERVIEW_PROOF_FAILED \(error)")
      }
      return
    }
    if fixtures.redisPubSubUI {
      sessionData = Data(repeating: 1, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      connectedEngine = "redis"
      redisSubscriptionSelector = "updates:*"
      status = "Redis Pub/Sub fixture"
      return
    }
    if fixtures.redisKeyView {
      guard let client else {
        writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "redis", host: "127.0.0.1", port: 6380,
            database: "0", user: "", password: "", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "redis"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first(where: { $0.name == "db0" })
        else {
          writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED db0 missing")
          return
        }
        let keys = try await client.refreshCatalog(
          session: session, parentNodeId: database.idBytes
        )
        let expected = Set([
          "redis_key_string", "redis_key_hash", "redis_key_list",
          "redis_key_set", "redis_key_sorted_set", "redis_key_stream",
        ])
        guard expected.isSubset(of: Set(keys.map(\.kind))),
          let hash = keys.first(where: { $0.kind == "redis_key_hash" })
        else {
          writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED key kinds missing")
          return
        }
        catalogSnapshot = [database] + keys
        for key in keys where expected.contains(key.kind) {
          _ = try await client.redisKeyView(
            sessionId: session, catalogNodeId: key.idBytes, collectionSkip: 0
          )
        }
        await openCatalogObject(nodeKey: catalogNodeKey(hash.idBytes))
        await loadMoreRedisKey()
        guard activeObjectTab?.redisView?.kind == "hash",
          (activeObjectTab?.redisView?.lines.count ?? 0) > 34
        else {
          writePerformanceMetric(
            "REDIS_KEY_VIEW_PROOF_FAILED native view kind=\(activeObjectTab?.redisView?.kind ?? "nil") lines=\(activeObjectTab?.redisView?.lines.count ?? 0) next=\(String(describing: activeObjectTab?.redisView?.nextSkip))"
          )
          return
        }
        try? await Task.sleep(for: .milliseconds(500))
        runNativeRedisKeyViewAudit()
      } catch {
        writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED \(error)")
      }
      return
    }
    if let importPath = fixtures.csvImportPath {
      guard let client else {
        writePerformanceMetric("CSV_IMPORT_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 5433,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "postgresql"
        guard
          let database = try await client.refreshCatalog(
            session: session, parentNodeId: nil
          ).first,
          let schema = try await client.refreshCatalog(
            session: session, parentNodeId: database.idBytes
          ).first(where: { $0.name == "public" })
        else {
          writePerformanceMetric("CSV_IMPORT_PROOF_FAILED catalog hierarchy missing")
          return
        }
        let objects = try await client.refreshCatalog(
          session: session, parentNodeId: schema.idBytes
        )
        guard let object = objects.first(where: { $0.name == "import_probe" }) else {
          writePerformanceMetric(
            "CSV_IMPORT_PROOF_FAILED target missing objects=\(objects.map(\.name))"
          )
          return
        }
        let tab = NativeObjectTab(
          id: dependencies.identifiers.next(), node: object, pinned: true
        )
        objectTabs = [tab]
        selectedObjectTabId = tab.id
        selectedWorkbenchKind = "object"
        let url = URL(fileURLWithPath: importPath)
        csvImportUrl = url
        csvImportPreview = try await client.previewCsvImport(path: importPath)
        csvImportMappedColumns = csvImportPreview?.headers ?? []
        csvImportColumnTypes = ["signed", "text"]
        csvImportPresented = true
        await stageCsvImport()
        guard csvImportReview?.rowCount == 2 else {
          writePerformanceMetric(
            "CSV_IMPORT_PROOF_FAILED \(csvImportError ?? "review missing")"
          )
          return
        }
        await applyCsvImport()
        guard csvImportError == nil, csvImportOutcome?.contains("2 applied") == true else {
          writePerformanceMetric(
            "CSV_IMPORT_PROOF_FAILED \(csvImportError ?? csvImportOutcome ?? "apply missing")"
          )
          return
        }
        guard
          let verification = try await fetchPage(
            intent: "execute",
            statement: "SELECT count(*)::bigint AS n FROM import_probe",
            tab: activeQueryTab
          ), verification.rows == [["2"]]
        else {
          writePerformanceMetric("CSV_IMPORT_PROOF_FAILED server count mismatch")
          return
        }
        try? await Task.sleep(for: .milliseconds(500))
        runNativeCsvImportAudit()
      } catch {
        writePerformanceMetric("CSV_IMPORT_PROOF_FAILED \(error)")
      }
      return
    }
    if fixtures.resultCopy {
      guard let client else {
        writePerformanceMetric("RESULT_COPY_PROOF_FAILED no bridge")
        return
      }
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 5433,
            database: "db", user: "u", password: "secret", tlsMode: "off"
          ))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "postgresql"
        activeQueryTab.resultTable = try await fetchPage(
          intent: "execute",
          statement: "SELECT 7::bigint AS id, 'a,b'::text AS name",
          tab: activeQueryTab
        )
        activeQueryTab.selectedCell = NativeCellSelection(row: 0, column: 0)
        await copyResult(scope: "loaded", preferredFormat: "json")
        guard copyError == nil else {
          writePerformanceMetric("RESULT_COPY_PROOF_FAILED \(copyError ?? "unknown")")
          return
        }
        if let exportPath = fixtures.resultExportPath {
          let bytes = try await client.exportLoadedResult(
            resultId: activeQueryTab.resultIdData ?? Data(),
            revision: activeQueryTab.resultRevision,
            format: "json", path: exportPath
          )
          let exported = try String(contentsOfFile: exportPath, encoding: .utf8)
          guard bytes == exported.utf8.count, exported.contains(#""id":7"#) else {
            writePerformanceMetric("RESULT_EXPORT_PROOF_FAILED payload mismatch")
            return
          }
        }
        if let streamPath = fixtures.streamExportPath {
          let operationId = try await client.startStreamExport(
            sessionId: session,
            statement: "SELECT generate_series(1, 1200)::bigint AS id",
            format: "csv", path: streamPath)
          let outcome = try await pollStreamExport(client: client, operationId: operationId)
          let exported = try String(contentsOfFile: streamPath, encoding: .utf8)
          guard outcome.phase == "completed", outcome.completedRows == 1_200,
            exported.hasPrefix("id\n"), exported.contains("1200\n")
          else {
            writePerformanceMetric(
              "RESULT_EXPORT_PROOF_FAILED stream phase=\(outcome.phase) rows=\(outcome.completedRows)"
            )
            return
          }
          _ = try await client.dismissStreamExport(operationId: operationId)
        }
        runNativeResultCopyAudit()
      } catch {
        writePerformanceMetric("RESULT_COPY_PROOF_FAILED \(error)")
      }
      return
    }
    if fixtures.queryTabs {
      let first = NativeQueryTab(
        id: dependencies.identifiers.next(), title: "Users", statementText: "SELECT 1;"
      )
      first.resultTable = WorkbenchTable(columns: ["n"], rows: [["1"]])
      first.isRunning = true
      first.querySummary = "first result"
      let second = NativeQueryTab(
        id: dependencies.identifiers.next(), title: "Orders", statementText: "SELECT 2;"
      )
      second.resultTable = WorkbenchTable(columns: ["n"], rows: [["2"]])
      second.querySummary = "second result"
      queryTabs = [first, second]
      selectedQueryTabId = second.id
      sessionHex = String(repeating: "a", count: 32)
      connectedEngine = "postgresql"
      status = "Query tabs fixture"
      guard queryText == "SELECT 2;", resultTable?.rows == [["2"]], !isRunning,
        querySummary == "second result",
        first.statementText == "SELECT 1;", first.resultTable?.rows == [["1"]],
        first.isRunning, first.querySummary == "first result"
      else {
        writePerformanceMetric("QUERY_TABS_PROOF_FAILED isolation mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeQueryTabsAudit()
      return
    }
    if fixtures.sqlFiles {
      sqlFile = WorkbenchSQLFile(
        path: "/tmp/fixture.sql", statementText: "SELECT fixture_sql_file;",
        modifiedNanos: 1, len: 24
      )
      sqlFileBaseline = "SELECT fixture_sql_file;"
      queryText = "SELECT fixture_sql_file;"
      status = "SQL file fixture"
      try? await Task.sleep(for: .milliseconds(500))
      runNativeSqlFilesAudit()
      return
    }
    if fixtures.savedQueries {
      savedQueries = [
        WorkbenchSavedQueryItem(
          queryId: 1, name: "Recent users", engine: "postgresql",
          statementText: "SELECT id FROM users", updatedAt: "2026-07-19 05:00:00"
        ),
        WorkbenchSavedQueryItem(
          queryId: 2, name: "Scan keys", engine: "redis",
          statementText: "SCAN 0", updatedAt: "2026-07-19 04:00:00"
        ),
      ]
      savedQueriesPresented = true
      status = "Saved queries fixture"
      guard savedQueries.map(\.engine) == ["postgresql", "redis"],
        savedQueries[0].statementText == "SELECT id FROM users"
      else {
        writePerformanceMetric("SAVED_QUERIES_PROOF_FAILED projection mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeSavedQueriesAudit()
      return
    }
    if fixtures.history {
      historyItems = [
        WorkbenchHistoryItem(
          historyId: 2, engine: "postgresql", databaseName: "postgres",
          schemaName: "public", statementText: "SELECT fixture_history",
          outcome: "completed", createdAt: "2026-07-19 05:00:00"
        ),
        WorkbenchHistoryItem(
          historyId: 1, engine: "redis", databaseName: "0",
          schemaName: nil, statementText: nil,
          outcome: "failed", createdAt: "2026-07-19 04:00:00"
        ),
      ]
      historyPresented = true
      status = "History fixture"
      guard historyItems.count == 2,
        historyItems[0].statementText == "SELECT fixture_history",
        historyItems[1].statementText == nil
      else {
        writePerformanceMetric("HISTORY_PROOF_FAILED projection mismatch")
        return
      }
      try? await Task.sleep(for: .milliseconds(500))
      runNativeHistoryAudit()
      return
    }
    if fixtures.profileGroups {
      profileGroups = [
        WorkbenchProfileGroup(name: "Empty", alphabetical: false),
        WorkbenchProfileGroup(name: "Production", alphabetical: true),
      ]
      profiles = [
        WorkbenchProfileItem(
          idBytes: Data(repeating: 1, count: 16), revision: 0,
          name: "Zebra", engine: "postgresql", group: "Production",
          favorite: false, savedOrder: 0, host: "z.internal", port: "5432",
          context: "db", safetyMode: "confirm_writes", environment: "production",
          productionWarning: true, dangerousPlaintext: false, connected: true
        ),
        WorkbenchProfileItem(
          idBytes: Data(repeating: 2, count: 16), revision: 0,
          name: "Alpha", engine: "postgresql", group: "Production",
          favorite: false, savedOrder: 1, host: "a.internal", port: "5432",
          context: "db", safetyMode: "read_only", environment: "production",
          productionWarning: true, dangerousPlaintext: false, connected: false
        ),
      ]
      activeProfileId = profiles[0].idBytes
      sessionData = Data(repeating: 3, count: 16)
      sessionHealth = WorkbenchSessionHealth(
        state: "healthy", serverReachable: true,
        elapsedMillis: 12, authenticationStopped: false
      )
      status = "Profile group fixture"
      guard profileSections.count == 2,
        let connectedFixture = profileSections[1].profiles.last,
        profileSections.map(\.title) == ["Empty", "Production"],
        profileSections[0].profiles.isEmpty,
        profileSections[1].profiles.map(\.name) == ["Alpha", "Zebra"],
        connectedFixture.connected,
        connectionState(connectedFixture) == "Healthy · 12 ms",
        activeEnvironmentLabel == "Production",
        activeSafetyLabel == "Confirm writes",
        activeProductionWarning
      else {
        writePerformanceMetric("PROFILE_GROUP_PROOF_FAILED group projection mismatch")
        return
      }
      reconnectState = "Reconnecting · attempt 1"
      guard connectionState(connectedFixture) == "Reconnecting · attempt 1" else {
        writePerformanceMetric("PROFILE_GROUP_PROOF_FAILED reconnect projection mismatch")
        return
      }
      reconnectState = nil
      try? await Task.sleep(for: .milliseconds(500))
      runNativeProfileGroupAudit()
      return
    }
    guard let client else {
      bridgeError = startupError ?? "Bridge unavailable"
      status = "error"
      return
    }
    if fixtures.activeQuery {
      do {
        let session = try await client.open(
          params: WorkbenchOpenParams(
            engine: formEngine, host: formHost, port: 5432,
            database: formDatabase, user: formUser, password: formPassword,
            tlsMode: "off"))
        sessionData = session
        sessionHex = session.map { String(format: "%02x", $0) }.joined()
        connectedEngine = formEngine
        status = "Scripted query running"
        Task { [weak self] in await self?.runQuery() }
      } catch {
        bridgeError = "Scripted query setup failed: \(error)"
        status = "error"
      }
      return
    }
    do {
      historyRetention = try await client.historyRetention()
      await refreshProfiles()
      await restoreWindowIntentOnLaunch()
    } catch {
      bridgeError = "Bridge init failed: \(error)"
      status = "error"
    }
  }

  private func installPerformanceFixtureIfRequested() {
    guard let requested = fixtures.performanceGridRows, requested > 0
    else { return }
    let count = min(requested, 10_000)
    let columns = ["id", "engine", "schema", "object", "status", "rows", "bytes", "note"]
    let started = Date()
    var rows: [[String]] = []
    rows.reserveCapacity(count)
    for index in 0..<count {
      let status = index.isMultiple(of: 3) ? "ready" : "idle"
      rows.append([
        String(index), "PostgreSQL", "public", "fixture_\(index)", status,
        String(index * 10), String(index * 128), "resident snapshot",
      ])
    }
    resultTable = WorkbenchTable(columns: columns, rows: rows)
    let elapsed = Date().timeIntervalSince(started)
    catalogSummary =
      "Performance fixture · \(counted(count, "row")) · \(counted(columns.count, "column"))"
    writePerformanceMetric(
      "PERF_FIXTURE_READY rows=\(count) columns=\(columns.count) build_seconds=\(String(format: "%.6f", elapsed))"
    )
  }

  private func sharesBridge(with other: WorkbenchPresentationStore) -> Bool {
    guard let client, let otherClient = other.client else {
      return client == nil && other.client == nil
    }
    return client === otherClient
  }

  func refreshProfiles() async {
    guard let client else { return }
    profileSearchGeneration &+= 1
    let generation = profileSearchGeneration
    profilesLoading = true
    profilesError = nil
    do {
      let search = profileSearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.searchProfiles(search.isEmpty ? nil : search)
      let loadedGroups = try await client.listProfileGroups()
      guard generation == profileSearchGeneration else { return }
      profiles = loaded
      profileGroups = loadedGroups
      status =
        profiles.isEmpty
        ? "Bridge ready · no saved profiles"
        : "Bridge ready · \(profiles.count) profile\(profiles.count == 1 ? "" : "s")"
    } catch {
      guard generation == profileSearchGeneration else { return }
      profilesError = "List profiles failed: \(error)"
      status = "error"
    }
    if generation == profileSearchGeneration { profilesLoading = false }
  }

  func presentHistory() async {
    historyPresented = true
    await refreshHistory()
  }

  func refreshHistory() async {
    guard let client else { return }
    historyGeneration &+= 1
    let generation = historyGeneration
    historyLoading = true
    historyError = nil
    do {
      let search = historySearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.listHistory(search.isEmpty ? nil : search)
      guard generation == historyGeneration else { return }
      historyItems = loaded
    } catch {
      guard generation == historyGeneration else { return }
      historyError = "History failed: \(error)"
    }
    if generation == historyGeneration { historyLoading = false }
  }

  func setHistoryRetention(_ retention: String) async {
    guard let client else { return }
    do {
      try await client.setHistoryRetention(retention)
      historyRetention = retention
    } catch { historyError = "Retention change failed: \(error)" }
  }

  func restoreHistory(_ item: WorkbenchHistoryItem) {
    guard let statement = item.statementText else {
      historyError = "SQL text was not retained for this entry"
      return
    }
    queryText = statement
    historyPresented = false
    profileActionOutcome = "History restored to editor"
  }

  func presentSavedQueries() async {
    savedQueriesPresented = true
    await refreshSavedQueries()
  }

  func refreshSavedQueries() async {
    guard let client else { return }
    savedQueriesGeneration &+= 1
    let generation = savedQueriesGeneration
    savedQueriesLoading = true
    savedQueriesError = nil
    do {
      let search = savedQuerySearch.trimmingCharacters(in: .whitespacesAndNewlines)
      let loaded = try await client.listSavedQueries(
        engine: savedQueryEngine.isEmpty ? nil : savedQueryEngine,
        search: search.isEmpty ? nil : search
      )
      guard generation == savedQueriesGeneration else { return }
      savedQueries = loaded
    } catch {
      guard generation == savedQueriesGeneration else { return }
      savedQueriesError = "Saved queries failed: \(error)"
    }
    if generation == savedQueriesGeneration { savedQueriesLoading = false }
  }

  func beginSaveCurrentQuery() {
    guard !queryText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      profileActionError = "Cannot save an empty query"
      return
    }
    savedQueryName = ""
    saveQueryDialog = true
  }

  func saveCurrentQuery() async {
    guard let client else { return }
    let name = savedQueryName.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty else {
      savedQueriesError = "Query name is required"
      return
    }
    do {
      let engine = connectedEngine.isEmpty ? formEngine : connectedEngine
      _ = try await client.saveQuery(name: name, engine: engine, statement: queryText)
      saveQueryDialog = false
      savedQueryName = ""
      profileActionOutcome = "Saved query: \(name)"
      await refreshSavedQueries()
    } catch { savedQueriesError = "Save query failed: \(error)" }
  }

  func restoreSavedQuery(_ item: WorkbenchSavedQueryItem) {
    queryText = item.statementText
    savedQueriesPresented = false
    profileActionOutcome = "Saved query restored to editor"
  }

  func removePendingSavedQuery() async {
    guard let client, let item = pendingSavedQueryRemoval else { return }
    pendingSavedQueryRemoval = nil
    do {
      _ = try await client.deleteSavedQuery(item.queryId)
      await refreshSavedQueries()
    } catch { savedQueriesError = "Delete query failed: \(error)" }
  }

  func addQueryTab() {
    guard queryTabs.count + objectTabs.count < 64 else {
      profileActionError = "At most 64 workbench tabs are allowed"
      return
    }
    let tab = NativeQueryTab(
      id: dependencies.identifiers.next(),
      title: "Query \(queryTabs.count + 1)",
      statementText: ""
    )
    queryTabs.append(tab)
    selectedQueryTabId = tab.id
    selectedWorkbenchKind = "query"
    Task { await persistSessionIntent() }
  }

  func selectQueryTab(_ tab: NativeQueryTab) {
    if selectedWorkbenchKind == "object" { activeObjectTab?.pinned = true }
    selectedQueryTabId = tab.id
    selectedWorkbenchKind = "query"
    Task { await persistSessionIntent() }
  }

  func requestCloseQueryTab(_ tab: NativeQueryTab) {
    guard queryTabs.count > 1 else {
      profileActionError = "At least one query tab must remain open"
      return
    }
    guard !tab.isRunning else {
      profileActionError = "Cancel the running query before closing its tab"
      return
    }
    if tab.statementText != tab.sqlFileBaseline {
      pendingQueryTabClose = tab
    } else {
      closeQueryTab(tab)
    }
  }

  func closePendingQueryTab() {
    guard let tab = pendingQueryTabClose else { return }
    pendingQueryTabClose = nil
    closeQueryTab(tab)
  }

  private func closeQueryTab(_ tab: NativeQueryTab) {
    guard let index = queryTabs.firstIndex(where: { $0.id == tab.id }), queryTabs.count > 1 else {
      return
    }
    queryTabs.remove(at: index)
    if selectedQueryTabId == tab.id {
      selectedQueryTabId = queryTabs[min(index, queryTabs.count - 1)].id
    }
    Task { await persistSessionIntent() }
  }

  func beginRenameQueryTab(_ tab: NativeQueryTab) {
    queryTabRename = tab
    queryTabRenameText = tab.title
  }

  func renameQueryTab() {
    let title = queryTabRenameText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard let tab = queryTabRename, !title.isEmpty, title.utf8.count <= 256 else {
      profileActionError = "Tab title must be 1 to 256 bytes"
      return
    }
    tab.title = title
    queryTabRename = nil
    queryTabRenameText = ""
    Task { await persistSessionIntent() }
  }

  func openCatalogObject(nodeKey: String) async {
    guard let node = catalogSnapshot?.first(where: { catalogNodeKey($0.idBytes) == nodeKey })
    else { return }
    let browsableKinds: Set<String> = [
      "postgresql_table", "postgresql_view", "postgresql_materialized_view",
      "postgresql_foreign_table", "postgresql_partitioned_table", "postgresql_sequence",
      "clickhouse_table", "clickhouse_view", "clickhouse_materialized_view",
      "clickhouse_dictionary",
      "redis_key_unknown", "redis_key_string", "redis_key_hash",
      "redis_key_list", "redis_key_set", "redis_key_sorted_set",
      "redis_key_stream",
    ]
    guard browsableKinds.contains(node.kind) else {
      profileActionError = "\(node.name) is not a browsable table-like object"
      return
    }
    guard queryTabs.count + objectTabs.count < 64 else {
      profileActionError = "At most 64 workbench tabs are allowed"
      return
    }
    objectTabs.last(where: { !$0.pinned })?.pinned = true
    let tab = NativeObjectTab(id: dependencies.identifiers.next(), node: node)
    objectTabs.append(tab)
    selectedObjectTabId = tab.id
    selectedWorkbenchKind = "object"
    await loadObjectTab(tab)
    await loadObjectFilterPresets(tab)
  }

  func openCatalogObject(nodeId: Data) async {
    await openCatalogObject(nodeKey: catalogNodeKey(nodeId))
  }

  func selectObjectTab(_ tab: NativeObjectTab) {
    if selectedWorkbenchKind == "object", selectedObjectTabId != tab.id {
      activeObjectTab?.pinned = true
    }
    selectedObjectTabId = tab.id
    selectedWorkbenchKind = "object"
    Task { await loadObjectFilterPresets(tab) }
  }

  func pinObjectTab(_ tab: NativeObjectTab) {
    tab.pinned = true
  }

  func closeObjectTab(_ tab: NativeObjectTab) {
    guard !tab.isRunning else {
      profileActionError = "Cancel the running browse before closing its tab"
      return
    }
    guard let index = objectTabs.firstIndex(where: { $0.id == tab.id }) else { return }
    objectTabs.remove(at: index)
    if selectedObjectTabId == tab.id {
      if objectTabs.isEmpty {
        selectedObjectTabId = nil
        selectedWorkbenchKind = "query"
      } else {
        let next = objectTabs[min(index, objectTabs.count - 1)]
        selectedObjectTabId = next.id
        selectedWorkbenchKind = "object"
      }
    }
  }

  private func loadObjectTab(_ tab: NativeObjectTab) async {
    guard let client, let session = sessionData else { return }
    tab.error = nil
    tab.summary = nil
    do {
      if tab.kind.hasPrefix("redis_key_") {
        tab.isRunning = true
        defer { tab.isRunning = false }
        let view = try await client.redisKeyView(
          sessionId: session, catalogNodeId: tab.catalogNodeId,
          collectionSkip: 0
        )
        tab.redisView = view
        tab.summary = "Redis \(view.kind) · \(view.lines.count) lines"
        return
      }
      let operation = try await client.submitCatalogBrowse(
        session: session, nodeId: tab.catalogNodeId, sort: tab.sort, filters: tab.filters,
        rawWhere: tab.rawWhere
      )
      tab.activeOperationId = operation
      tab.isRunning = true
      defer {
        tab.activeOperationId = nil
        tab.isRunning = false
      }
      let projection = try await client.finish(operationId: operation)
      tab.resultTable = projection.table
      if let envelope = projection.envelope {
        tab.resultIdData = envelope.resultId
        tab.resultRevision = envelope.revision
        tab.nextStartRow =
          envelope.rowCount == 500
          ? envelope.startRow + UInt64(envelope.rowCount) : nil
      }
      if let table = projection.table {
        tab.summary =
          "\(counted(table.rows.count, "row")) · \(counted(table.columns.count, "column"))"
      } else {
        tab.summary = "No rows"
      }
    } catch { tab.error = "Object browse failed: \(error)" }
  }

  func loadMoreRedisKey() async {
    guard let tab = activeObjectTab, let client, let session = sessionData,
      let skip = tab.redisView?.nextSkip, !tab.isRunning
    else { return }
    tab.isRunning = true
    defer { tab.isRunning = false }
    do {
      let next = try await client.redisKeyView(
        sessionId: session, catalogNodeId: tab.catalogNodeId,
        collectionSkip: skip
      )
      let existing = tab.redisView?.lines ?? []
      tab.redisView = WorkbenchRedisKeyView(
        kind: next.kind,
        lines: existing + Array(next.lines.dropFirst(min(2, next.lines.count))),
        nextSkip: next.nextSkip
      )
      tab.summary = "Redis \(next.kind) · \(tab.redisView?.lines.count ?? 0) lines"
    } catch { tab.error = "Redis key page failed: \(error)" }
  }

  func reloadObjectTab() async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    await loadObjectTab(tab)
  }

  func addObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.sort.count < 16,
      !tab.sort.contains(where: { $0.column == column })
    else { return }
    tab.sort.append(WorkbenchBrowseSort(column: column))
    await loadObjectTab(tab)
  }

  func toggleObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning,
      let index = tab.sort.firstIndex(where: { $0.column == column })
    else { return }
    let current = tab.sort[index]
    tab.sort[index] = WorkbenchBrowseSort(
      column: current.column, descending: !current.descending)
    await loadObjectTab(tab)
  }

  func removeObjectSort(column: String) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.sort.removeAll(where: { $0.column == column })
    await loadObjectTab(tab)
  }

  func addObjectFilter() async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.filters.count < 32,
      !tab.filterColumn.isEmpty
    else { return }
    let value =
      ["is_null", "is_not_null"].contains(tab.filterOperator)
      ? nil : tab.filterValue
    tab.filters.append(
      WorkbenchBrowseFilter(
        id: dependencies.identifiers.next(), column: tab.filterColumn,
        operatorName: tab.filterOperator, value: value))
    tab.filterValue = ""
    await loadObjectTab(tab)
  }

  func removeObjectFilter(id: UUID) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.filters.removeAll(where: { $0.id == id })
    await loadObjectTab(tab)
  }

  func clearObjectFilters() async {
    guard let tab = activeObjectTab, !tab.isRunning, !tab.filters.isEmpty else { return }
    tab.filters.removeAll()
    await loadObjectTab(tab)
  }

  func applyObjectRawWhere() async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    let fragment = tab.rawWhereDraft.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !fragment.isEmpty, fragment.utf8.count <= 65_536 else { return }
    tab.rawWhere = fragment
    await loadObjectTab(tab)
  }

  func clearObjectRawWhere() async {
    guard let tab = activeObjectTab, !tab.isRunning, tab.rawWhere != nil else { return }
    tab.rawWhere = nil
    tab.rawWhereDraft = ""
    await loadObjectTab(tab)
  }

  private func loadObjectFilterPresets(_ tab: NativeObjectTab) async {
    guard let client, let session = sessionData, !tab.kind.hasPrefix("redis_key_") else { return }
    do {
      tab.filterPresets = try await client.listCatalogFilterPresets(
        session: session, nodeId: tab.catalogNodeId)
      tab.filterPresetError = nil
    } catch {
      tab.filterPresets = []
      tab.filterPresetError = "Could not load filter presets: \(error)"
    }
  }

  func saveObjectFilterPreset() async {
    guard let tab = activeObjectTab, let client, let session = sessionData, !tab.isRunning else {
      return
    }
    let name = tab.filterPresetName.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty, name.utf8.count <= 64 else { return }
    do {
      try await client.saveCatalogFilterPreset(
        session: session, nodeId: tab.catalogNodeId,
        preset: WorkbenchSavedFilterPreset(
          name: name, filters: tab.filters, rawWhere: tab.rawWhere))
      tab.filterPresetName = ""
      tab.filterPresetOutcome = "Saved filter preset \(name)"
      tab.filterPresetError = nil
      await loadObjectFilterPresets(tab)
    } catch {
      tab.filterPresetOutcome = nil
      tab.filterPresetError = "Could not save filter preset: \(error)"
    }
  }

  func applyObjectFilterPreset(_ preset: WorkbenchSavedFilterPreset) async {
    guard let tab = activeObjectTab, !tab.isRunning else { return }
    tab.filters = preset.filters.map {
      WorkbenchBrowseFilter(
        id: dependencies.identifiers.next(), column: $0.column,
        operatorName: $0.operatorName, value: $0.value)
    }
    tab.rawWhere = preset.rawWhere
    tab.rawWhereDraft = preset.rawWhere ?? ""
    tab.filterPresetOutcome = "Loaded filter preset \(preset.name)"
    tab.filterPresetError = nil
    await loadObjectTab(tab)
  }

  func loadObjectStructure() async {
    guard let tab = activeObjectTab, let client, let session = sessionData,
      !tab.structureLoading
    else { return }
    tab.selectedSection = "structure"
    tab.structureLoading = true
    tab.structureError = nil
    defer { tab.structureLoading = false }
    do {
      tab.structure = try await client.relationStructure(
        sessionId: session, catalogNodeId: tab.catalogNodeId
      )
    } catch {
      tab.structure = nil
      tab.structureError = "Structure unavailable: \(error)"
    }
  }

  func showFindReplace() {
    guard queryWorkbenchSelected else { return }
    findPattern = ""
    findReplacement = ""
    findMode = "literal"
    findScope = "document"
    findStatus = nil
    findError = nil
    activeQueryTab.findScopeRange = nil
    activeQueryTab.lastFindMatch = nil
    findReplacePresented = true
  }

  func setFindScope(_ scope: String) {
    findScope = scope
    activeQueryTab.findScopeRange = scope == "selection" ? activeQueryTab.editorSelection : nil
    activeQueryTab.lastFindMatch = nil
    findStatus = nil
    findError = nil
  }

  func resetFindTraversal() {
    activeQueryTab.lastFindMatch = nil
    findStatus = nil
    findError = nil
  }

  func findEditorMatch(backwards: Bool) {
    do {
      let match = try NativeFindReplaceEngine.find(
        in: queryText, pattern: findPattern, mode: findMode,
        scope: try effectiveFindScope(), selection: queryEditorSelection,
        previousMatch: activeQueryTab.lastFindMatch, backwards: backwards)
      guard let match else {
        findStatus = "No match"
        findError = nil
        activeQueryTab.lastFindMatch = nil
        return
      }
      queryEditorSelection = match
      activeQueryTab.lastFindMatch = match
      findStatus = "Match at character \(match.location + 1)"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  func replaceEditorMatch() {
    do {
      let outcome = try NativeFindReplaceEngine.replaceCurrent(
        in: queryText, pattern: findPattern, replacement: findReplacement,
        mode: findMode, scope: try effectiveFindScope(), selection: queryEditorSelection)
      guard let outcome else {
        findEditorMatch(backwards: false)
        return
      }
      queryText = outcome.text
      queryEditorSelection = outcome.selection
      updateFindScope(afterReplacing: outcome.replacedRange, delta: outcome.delta)
      activeQueryTab.lastFindMatch = nil
      findStatus = "Replaced 1 match"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  func replaceAllEditorMatches() {
    do {
      let outcome = try NativeFindReplaceEngine.replaceAll(
        in: queryText, pattern: findPattern, replacement: findReplacement,
        mode: findMode, scope: try effectiveFindScope())
      queryText = outcome.text
      queryEditorSelection = outcome.selection
      if findScope == "selection" { activeQueryTab.findScopeRange = outcome.selection }
      activeQueryTab.lastFindMatch = nil
      findStatus = "Replaced \(outcome.count) match\(outcome.count == 1 ? "" : "es")"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  private func effectiveFindScope() throws -> NSRange {
    let whole = NSRange(location: 0, length: (queryText as NSString).length)
    guard findScope == "selection", let selected = activeQueryTab.findScopeRange else {
      return whole
    }
    let location = min(selected.location, whole.length)
    let scope = NSRange(location: location, length: min(selected.length, whole.length - location))
    guard scope.length > 0 else { throw NativeFindReplaceError.invalidScope }
    return scope
  }

  private func updateFindScope(afterReplacing range: NSRange, delta: Int) {
    guard findScope == "selection", var scope = activeQueryTab.findScopeRange,
      range.location >= scope.location, NSMaxRange(range) <= NSMaxRange(scope)
    else { return }
    scope.length = max(0, scope.length + delta)
    activeQueryTab.findScopeRange = scope
  }

  func showDdlChange() {
    guard canEditSelectedStructure else { return }
    ddlChangeKind = "add_column"
    ddlChangeObjectName = ""
    ddlChangeDefinition = ""
    ddlChangeReview = nil
    ddlChangeOutcome = nil
    ddlChangeError = nil
    ddlChangeCatalogNodeId = activeObjectTab?.catalogNodeId
    ddlChangePresented = true
  }

  func stageDdlChange() async {
    guard let client, let session = sessionData, let nodeId = ddlChangeCatalogNodeId,
      ddlChangeReview == nil, !ddlChangeApplying
    else { return }
    ddlChangeError = nil
    ddlChangeOutcome = nil
    do {
      ddlChangeReview = try await client.stageDdlChange(
        sessionId: session, catalogNodeId: nodeId, kind: ddlChangeKind,
        objectName: ddlChangeObjectName.trimmingCharacters(in: .whitespacesAndNewlines),
        definition: ddlChangeDefinition.trimmingCharacters(in: .whitespacesAndNewlines),
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      ddlChangeReview = nil
      ddlChangeError = "Structure review rejected: \(error)"
    }
  }

  func applyDdlChange() async {
    guard let client, let session = sessionData, let review = ddlChangeReview else { return }
    let nodeId = ddlChangeCatalogNodeId
    ddlChangeReview = nil
    ddlChangeApplying = true
    ddlChangeError = nil
    defer { ddlChangeApplying = false }
    do {
      ddlChangeOutcome = try await client.applyDdlChange(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmed: true)
      if let nodeId,
        let tab = objectTabs.first(where: { $0.catalogNodeId == nodeId })
      {
        tab.structure = try await client.relationStructure(
          sessionId: session, catalogNodeId: nodeId)
        tab.structureError = nil
      }
    } catch {
      ddlChangeError = "Structure outcome unknown or failed; review consumed: \(error)"
    }
  }

  func discardDdlChangeReview() async {
    if let review = ddlChangeReview, let client {
      _ = try? await client.revokeDdlChange(tokenId: review.tokenId)
    }
    ddlChangeReview = nil
  }

  func closeDdlChange() async {
    await discardDdlChangeReview()
    ddlChangePresented = false
  }

  func showTableOperation() {
    guard canOperateSelectedTable else { return }
    tableOperationKind = connectedEngine == "clickhouse" ? "optimize" : "truncate"
    tableOperationNewName = ""
    tableOperationConfirmation = ""
    tableOperationReview = nil
    tableOperationStatus = nil
    tableOperationOutcome = nil
    tableOperationError = nil
    tableOperationCatalogNodeId = activeObjectTab?.catalogNodeId
    tableOperationPresented = true
  }

  func resetTableOperationReview() async {
    guard !tableOperationApplying else { return }
    if let review = tableOperationReview, let client {
      _ = try? await client.revokeTableOperation(tokenId: review.tokenId)
    }
    if let operationId = tableOperationId, let client {
      _ = try? await client.dismissTableOperation(operationId: operationId)
    }
    tableOperationReview = nil
    tableOperationStatus = nil
    tableOperationId = nil
    tableOperationConfirmation = ""
    tableOperationOutcome = nil
    tableOperationError = nil
  }

  func stageTableOperation() async {
    guard let client, let session = sessionData, let nodeId = tableOperationCatalogNodeId,
      tableOperationReview == nil, !tableOperationApplying
    else { return }
    tableOperationError = nil
    tableOperationOutcome = nil
    do {
      tableOperationReview = try await client.stageTableOperation(
        sessionId: session, catalogNodeId: nodeId, kind: tableOperationKind,
        newName: tableOperationNewName.trimmingCharacters(in: .whitespacesAndNewlines),
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      tableOperationError = "Table operation review rejected: \(error)"
    }
  }

  func applyTableOperation() async {
    guard let client, let session = sessionData, let review = tableOperationReview else { return }
    guard tableOperationConfirmation == review.confirmation else {
      tableOperationError = "Type the exact target table name to authorize this operation."
      return
    }
    let kind = tableOperationKind
    let nodeId = tableOperationCatalogNodeId
    tableOperationReview = nil
    tableOperationApplying = true
    tableOperationStatus = nil
    tableOperationError = nil
    defer { tableOperationApplying = false }
    do {
      let operationId = try await client.startTableOperation(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmation: tableOperationConfirmation)
      tableOperationId = operationId
      while true {
        let status = try await client.tableOperationStatus(operationId: operationId)
        tableOperationStatus = status
        if status.phase != "running" { break }
        try await Task.sleep(for: .milliseconds(100))
      }
      guard let status = tableOperationStatus else { return }
      if status.phase == "succeeded" {
        tableOperationOutcome = status.summary
      } else {
        tableOperationError = "Table operation \(status.phase): \(status.summary)"
        return
      }
      if ["rename", "drop"].contains(kind), let nodeId {
        objectTabs.removeAll(where: { $0.catalogNodeId == nodeId })
        selectedObjectTabId = nil
        selectedWorkbenchKind = "query"
        await browse()
      } else if kind == "truncate", let nodeId,
        let tab = objectTabs.first(where: { $0.catalogNodeId == nodeId })
      {
        await loadObjectTab(tab)
      }
    } catch {
      tableOperationError = "Table operation failed or outcome unknown; review consumed: \(error)"
    }
  }

  func closeTableOperation() async {
    await resetTableOperationReview()
    tableOperationPresented = false
  }

  func loadMoreObjectRows() async {
    guard let tab = activeObjectTab, let client, let resultId = tab.resultIdData,
      let start = tab.nextStartRow
    else { return }
    do {
      let (more, envelope) = try await client.fetchPage(
        resultId: resultId, startRow: start, revision: tab.resultRevision
      )
      if more.rows.isEmpty {
        tab.nextStartRow = nil
        return
      }
      if let table = tab.resultTable {
        guard let table = table.appending(more) else {
          tab.error = "Load more returned incompatible page metadata"
          return
        }
        tab.resultTable = table
        tab.summary =
          "\(counted(table.rows.count, "row")) · \(counted(table.columns.count, "column"))"
      }
      tab.nextStartRow =
        envelope.rowCount == 500
        ? envelope.startRow + UInt64(envelope.rowCount) : nil
    } catch { tab.error = "Load more failed: \(error)" }
  }

  func persistSessionIntent() async {
    guard let client, let profileId = activeProfileId,
      let selected = queryTabs.firstIndex(where: { $0.id == selectedQueryTabId })
    else { return }
    let intent = WorkbenchSessionIntent(
      database: formDatabase,
      schema: nil,
      selectedTab: UInt32(selected),
      tabs: queryTabs.map {
        WorkbenchWorkspaceTab(title: $0.title, statementText: $0.statementText)
      }
    )
    do {
      try await client.putNativeWindowIntent(
        windowId: windowId.uuidString.lowercased(), profileId: profileId, intent: intent
      )
    } catch { profileActionError = "Save workspace intent failed: \(error)" }
  }

  func copyResult(scope: String, preferredFormat: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "No resident result to copy"
      return
    }
    let selection = selectedCell
    if scope != "loaded", selection == nil {
      copyError = "Select a result cell first"
      return
    }
    copyOutcome = nil
    copyError = nil
    do {
      let row = selection.map { UInt64($0.row) }
      let column = selection.map { UInt32($0.column) }
      var payloads: [String: String] = [:]
      for format in ["csv", "tsv", "json", "markdown"] {
        payloads[format] = try await client.formatResultCopy(
          resultId: resultId, revision: resultRevision, scope: scope,
          row: row, column: column, format: format
        )
      }
      if preferredFormat == "sql_insert" {
        payloads[preferredFormat] = try await client.formatResultCopy(
          resultId: resultId, revision: resultRevision, scope: scope,
          row: row, column: column, format: preferredFormat
        )
      }
      let preferred = payloads[preferredFormat] ?? payloads["tsv"] ?? ""
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: preferred),
        AppPasteboardRepresentation(
          type: "public.comma-separated-values-text", value: payloads["csv"] ?? ""
        ),
        AppPasteboardRepresentation(
          type: "public.utf8-tab-separated-values-text", value: payloads["tsv"] ?? ""),
        AppPasteboardRepresentation(type: "public.json", value: payloads["json"] ?? ""),
        AppPasteboardRepresentation(
          type: "net.daringfireball.markdown", value: payloads["markdown"] ?? ""
        ),
      ])
      copyOutcome =
        "Copied \(scope) as \(preferredFormat.uppercased()) with CSV, TSV, JSON, and Markdown representations"
    } catch { copyError = "Copy failed: \(error)" }
  }

  func exportLoadedResult(format: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "No resident result to export"
      return
    }
    let fileExtension = format == "sql_insert" ? "sql" : format
    guard
      let selected = dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Loaded Result", prompt: "Export",
          suggestedFilename: "result.\(fileExtension)", allowedExtensions: [fileExtension]
        ))
    else { return }
    let url =
      selected.pathExtension.lowercased() == fileExtension
      ? selected : selected.appendingPathExtension(fileExtension)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    copyOutcome = nil
    copyError = nil
    do {
      let bytes = try await client.exportLoadedResult(
        resultId: resultId, revision: resultRevision, format: format, path: url.path
      )
      copyOutcome = "Exported \(bytes) bytes to \(url.lastPathComponent)"
    } catch { copyError = "Export failed: \(error)" }
  }

  func exportFullResult(format: String) async {
    guard let client, let resultId = resultIdData else {
      copyError = "Full-result export requires a resident result"
      return
    }
    let statement = queryText.trimmingCharacters(in: .whitespacesAndNewlines)
    if selectedWorkbenchKind == "query" && statement.isEmpty {
      copyError = "Query is empty"
      return
    }
    let fileExtension = format
    guard
      let selected = dependencies.filePanels.chooseSaveFile(
        AppFilePanelRequest(
          title: "Export Full Result", prompt: "Export",
          suggestedFilename: "result.\(fileExtension)", allowedExtensions: [fileExtension]
        ))
    else { return }
    let url =
      selected.pathExtension.lowercased() == fileExtension
      ? selected : selected.appendingPathExtension(fileExtension)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    copyOutcome = nil
    copyError = nil
    streamExportError = nil
    streamExportProgress = nil
    streamExportPresented = true
    do {
      let operationId: Data
      if selectedWorkbenchKind == "object" {
        operationId = try await client.startCatalogStreamExport(
          resultId: resultId, revision: resultRevision, format: format, path: url.path)
      } else {
        guard let session = sessionData else {
          throw PresentationStoreError.unavailable("stream-export-session")
        }
        operationId = try await client.startStreamExport(
          sessionId: session, statement: statement, format: format, path: url.path)
      }
      streamExportOperationId = operationId
      while streamExportOperationId == operationId {
        let progress = try await client.streamExportProgress(operationId: operationId)
        streamExportProgress = progress
        if !["running", "cancel_requested"].contains(progress.phase) {
          copyOutcome = progress.summary
          _ = try? await client.dismissStreamExport(operationId: operationId)
          streamExportOperationId = nil
          break
        }
        try await Task.sleep(for: .milliseconds(100))
      }
    } catch {
      streamExportOperationId = nil
      streamExportError = "Full-result export failed: \(error)"
    }
  }

  private func pollStreamExport(
    client: any WorkbenchBackend, operationId: Data
  ) async throws -> WorkbenchStreamExportProgress {
    while true {
      let progress = try await client.streamExportProgress(operationId: operationId)
      if !["running", "cancel_requested"].contains(progress.phase) { return progress }
      try await Task.sleep(for: .milliseconds(50))
    }
  }

  func cancelStreamExport() async {
    guard let client, let operationId = streamExportOperationId else { return }
    do {
      if try await client.cancelStreamExport(operationId: operationId) {
        streamExportProgress = try await client.streamExportProgress(operationId: operationId)
      }
    } catch { streamExportError = "Cancel export failed: \(error)" }
  }

  func closeStreamExport() {
    guard streamExportOperationId == nil else { return }
    streamExportPresented = false
    streamExportProgress = nil
    streamExportError = nil
  }

  func chooseCsvImport() async {
    guard let client, sqlInsertCopyAvailable else { return }
    guard
      let url = dependencies.filePanels.chooseOpenFile(
        AppFilePanelRequest(
          title: "Import CSV into Table", prompt: "Preview", allowedExtensions: ["csv"]
        ))
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let preview = try await client.previewCsvImport(path: url.path)
      csvImportUrl = url
      csvImportPreview = preview
      csvImportMappedColumns = preview.headers
      csvImportColumnTypes = Array(repeating: "text", count: preview.headers.count)
      csvImportReview = nil
      csvImportError = nil
      csvImportOutcome = nil
      csvImportProgress = nil
      csvImportErrorCopyOutcome = nil
      csvImportPresented = true
    } catch { csvImportError = "CSV preview failed: \(error)" }
  }

  func stageCsvImport() async {
    guard let client, let session = sessionData, let object = activeObjectTab,
      let url = csvImportUrl
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    csvImportError = nil
    do {
      csvImportReview = try await client.stageCsvImport(
        sessionId: session, catalogNodeId: object.catalogNodeId, path: url.path,
        mappedColumns: csvImportMappedColumns,
        mappedTypes: csvImportColumnTypes,
        expectedFingerprint: csvImportPreview?.fingerprint ?? "",
        nowMs: dependencies.clock.nowMilliseconds()
      )
    } catch { csvImportError = "Stage import failed: \(error)" }
  }

  func applyCsvImport() async {
    guard let client, let session = sessionData, let review = csvImportReview else { return }
    csvImportApplying = true
    csvImportError = nil
    defer { csvImportApplying = false }
    do {
      let operationId = try await client.startCsvImportApply(
        tokenId: review.tokenId,
        nowMs: dependencies.clock.nowMilliseconds(),
        sessionId: session
      )
      csvImportReview = nil
      csvImportOperationId = operationId
      while csvImportOperationId == operationId {
        let progress = try await client.csvImportProgress(operationId: operationId)
        csvImportProgress = progress
        if !["running", "cancel_requested"].contains(progress.phase) {
          csvImportOutcome = progress.summary
          _ = try? await client.dismissCsvImport(operationId: operationId)
          csvImportOperationId = nil
          if progress.phase == "completed" { await reloadObjectTab() }
          break
        }
        try await Task.sleep(for: .milliseconds(100))
      }
    } catch {
      csvImportReview = nil
      csvImportOperationId = nil
      csvImportError = "Import progress failed after authority was consumed: \(error)"
    }
  }

  func cancelCsvImport() async {
    guard let client, let operationId = csvImportOperationId else { return }
    do {
      if try await client.cancelCsvImport(operationId: operationId) {
        csvImportProgress = try await client.csvImportProgress(operationId: operationId)
      }
    } catch { csvImportError = "Cancel import failed: \(error)" }
  }

  func copyCsvImportErrors() {
    guard let progress = csvImportProgress, !progress.errors.isEmpty else { return }
    var text = progress.errors.joined(separator: "\n")
    if progress.errorsTruncated { text += "\n… additional errors omitted" }
    do {
      try dependencies.pasteboard.write([
        AppPasteboardRepresentation(type: "public.utf8-plain-text", value: text)
      ])
      csvImportErrorCopyOutcome = "Copied \(progress.errors.count) import errors"
    } catch { csvImportErrorCopyOutcome = "Copy errors failed: \(error)" }
  }

  func discardCsvImportReview() async {
    if let review = csvImportReview, let client {
      _ = try? await client.revokeReviewToken(tokenId: review.tokenId)
    }
    csvImportReview = nil
  }

  func closeCsvImport() async {
    await discardCsvImportReview()
    csvImportPresented = false
    csvImportPreview = nil
    csvImportMappedColumns = []
    csvImportColumnTypes = []
    csvImportUrl = nil
    csvImportProgress = nil
    csvImportErrorCopyOutcome = nil
  }

  func showRedisOverview() async {
    guard connectedEngine == "redis", let client, let session = sessionData,
      !redisOverviewLoading
    else { return }
    redisOverviewPresented = true
    redisOverviewLoading = true
    redisOverviewError = nil
    defer { redisOverviewLoading = false }
    do {
      redisOverview = try await client.redisOverview(sessionId: session)
    } catch {
      redisOverview = nil
      redisOverviewError = "Redis overview failed: \(error)"
    }
  }

  func showRedisSubscription() {
    guard connectedEngine == "redis", sessionData != nil else { return }
    redisSubscriptionPresented = true
    redisSubscriptionError = nil
  }

  func startRedisSubscription() async {
    guard let client, let session = sessionData, !redisSubscriptionStarting,
      !redisSubscriptionIsActive
    else { return }
    let selector = redisSubscriptionSelector.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !selector.isEmpty else {
      redisSubscriptionError = "Enter a channel or pattern"
      return
    }
    redisSubscriptionStarting = true
    redisSubscriptionError = nil
    defer { redisSubscriptionStarting = false }
    do {
      let operation = try await client.startRedisSubscription(
        sessionId: session, selector: selector, pattern: redisSubscriptionPattern)
      redisSubscriptionStatus = try await client.redisSubscriptionStatus(operationId: operation)
      beginRedisSubscriptionPolling(operation)
    } catch {
      redisSubscriptionStatus = nil
      redisSubscriptionError = "Subscription failed: \(error)"
    }
  }

  func refreshRedisSubscription() async {
    guard let client, let operation = redisSubscriptionStatus?.operationId else { return }
    do {
      let status = try await client.redisSubscriptionStatus(operationId: operation)
      redisSubscriptionStatus = status
      if !redisSubscriptionIsActive { redisSubscriptionPollTask?.cancel() }
    } catch {
      redisSubscriptionError = "Subscription status unavailable: \(error)"
      redisSubscriptionPollTask?.cancel()
    }
  }

  private func beginRedisSubscriptionPolling(_ operation: Data) {
    redisSubscriptionPollTask?.cancel()
    redisSubscriptionPollTask = Task { [weak self] in
      while !Task.isCancelled {
        try? await Task.sleep(for: .milliseconds(250))
        guard !Task.isCancelled, let self,
          self.redisSubscriptionStatus?.operationId == operation,
          self.redisSubscriptionPresented
        else { return }
        await self.refreshRedisSubscription()
        if !self.redisSubscriptionIsActive { return }
      }
    }
  }

  func cancelRedisSubscription() async {
    guard let client, let operation = redisSubscriptionStatus?.operationId else { return }
    do {
      _ = try await client.cancelRedisSubscription(operationId: operation)
      await refreshRedisSubscription()
    } catch {
      redisSubscriptionError = "Cancel failed: \(error)"
    }
  }

  func closeRedisSubscription() async {
    if redisSubscriptionIsActive { await cancelRedisSubscription() }
    redisSubscriptionPollTask?.cancel()
    redisSubscriptionPollTask = nil
    redisSubscriptionPresented = false
  }

  func showPostgresActivity() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresActivityPresented = true
    await refreshPostgresActivity()
  }

  func refreshPostgresActivity() async {
    guard let client, let session = sessionData, !postgresActivityLoading else { return }
    postgresActivityLoading = true
    postgresActivityError = nil
    defer { postgresActivityLoading = false }
    do {
      postgresActivityRows = try await client.postgresActivity(sessionId: session)
    } catch {
      postgresActivityRows = []
      postgresActivityError = "PostgreSQL activity failed: \(error)"
    }
  }

  func showPostgresRelationships() async {
    guard connectedEngine == "postgresql", activeObjectTab != nil else { return }
    postgresRelationshipsPresented = true
    await refreshPostgresRelationships()
  }

  func refreshPostgresRelationships() async {
    guard let client, let session = sessionData, let object = activeObjectTab,
      !postgresRelationshipsLoading
    else { return }
    postgresRelationshipsLoading = true
    postgresRelationshipsError = nil
    defer { postgresRelationshipsLoading = false }
    do {
      postgresRelationshipSnapshot = try await client.postgresRelationships(
        sessionId: session, catalogNodeId: object.catalogNodeId)
    } catch {
      postgresRelationshipSnapshot = nil
      postgresRelationshipsError = "Relationships unavailable: \(error)"
    }
  }

  func showPostgresRoles() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresRolesPresented = true
    await refreshPostgresRoles()
  }

  func refreshPostgresRoles() async {
    guard let client, let session = sessionData, !postgresRolesLoading else { return }
    postgresRolesLoading = true
    postgresRolesError = nil
    defer { postgresRolesLoading = false }
    do {
      postgresRoleSnapshot = try await client.postgresRoles(
        sessionId: session, catalogNodeId: activeObjectTab?.catalogNodeId)
    } catch {
      postgresRoleSnapshot = nil
      postgresRolesError = "Roles unavailable: \(error)"
    }
  }

  func stagePostgresRoleChange() async {
    guard let client, let session = sessionData else { return }
    postgresRolesError = nil
    postgresRoleChangeOutcome = nil
    do {
      postgresRoleChangeReview = try await client.stagePostgresRoleChange(
        sessionId: session, catalogNodeId: activeObjectTab?.catalogNodeId,
        kind: postgresRoleChangeKind,
        role: postgresRoleChangeRole.trimmingCharacters(in: .whitespacesAndNewlines),
        memberOrGrantee: postgresRoleChangeSubject.trimmingCharacters(in: .whitespacesAndNewlines),
        privilege: postgresRoleChangePrivilege,
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      postgresRoleChangeReview = nil
      postgresRolesError = "Role change rejected: \(error)"
    }
  }

  func applyPostgresRoleChange() async {
    guard let client, let session = sessionData, let review = postgresRoleChangeReview else {
      return
    }
    postgresRoleChangeReview = nil
    do {
      postgresRoleChangeOutcome = try await client.applyPostgresRoleChange(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmed: true)
      await refreshPostgresRoles()
    } catch {
      postgresRolesError = "Role change outcome unknown or failed; review consumed: \(error)"
    }
  }

  func discardPostgresRoleChange() async {
    if let review = postgresRoleChangeReview, let client {
      _ = try? await client.revokePostgresRoleChange(tokenId: review.tokenId)
    }
    postgresRoleChangeReview = nil
  }

  func openRelatedRelation(_ edge: WorkbenchRelationshipEdge) async {
    guard let snapshot = postgresRelationshipSnapshot, let nodes = catalogSnapshot else { return }
    let selectedIsSource =
      edge.fromSchema == snapshot.namespace && edge.fromTable == snapshot.relation
    let namespace = selectedIsSource ? edge.toSchema : edge.fromSchema
    let relation = selectedIsSource ? edge.toTable : edge.fromTable
    let node = nodes.first { candidate in
      guard candidate.name == relation, let parentId = candidate.parentIdBytes else { return false }
      return nodes.first(where: { $0.idBytes == parentId })?.name == namespace
    }
    guard let node else {
      postgresRelationshipsError = "Load \(namespace).\(relation) in the catalog before opening it."
      return
    }
    postgresRelationshipsPresented = false
    await openCatalogObject(nodeKey: catalogNodeKey(node.idBytes))
  }

  func signalPostgresBackend(kind: String, pid: Int32) async {
    guard let client, let session = sessionData else { return }
    postgresActivityError = nil
    postgresActivityOutcome = nil
    do {
      let outcome = try await client.signalPostgresBackend(
        sessionId: session, kind: kind, pid: pid)
      postgresActivityOutcome =
        outcome.acknowledged
        ? "\(kind.capitalized) acknowledged for PID \(pid)"
        : "PID \(pid) was not signalable"
      await refreshPostgresActivity()
    } catch {
      postgresActivityError = "\(kind.capitalized) failed: \(error)"
    }
  }

  func showPostgresTools() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresToolsPresented = true
    postgresToolError = nil
    await probePostgresTool()
  }

  func probePostgresTool() async {
    guard let client else { return }
    postgresToolError = nil
    let explicit = postgresToolExplicitPath.trimmingCharacters(in: .whitespacesAndNewlines)
    do {
      postgresToolProbe = try await client.probePostgresTool(
        kind: postgresToolKind,
        explicitPath: explicit.isEmpty ? nil : explicit)
    } catch {
      postgresToolProbe = nil
      postgresToolError = "Tool probe failed: \(error)"
    }
  }

  func choosePostgresToolFile() {
    let request = AppFilePanelRequest(
      title: postgresToolKind == "dump" ? "Choose Backup Destination" : "Choose Restore Archive",
      prompt: postgresToolKind == "dump" ? "Choose" : "Restore",
      suggestedFilename: postgresToolKind == "dump" ? "tablerock.dump" : nil,
      allowedExtensions: ["dump", "backup"])
    postgresToolFileUrl =
      postgresToolKind == "dump"
      ? dependencies.filePanels.chooseSaveFile(request)
      : dependencies.filePanels.chooseOpenFile(request)
    postgresToolStatus = nil
    postgresToolError = nil
  }

  func requestStartPostgresTool() {
    guard postgresToolProbe?.available == true, postgresToolFileUrl != nil else {
      postgresToolError = "Choose an available tool and archive file first"
      return
    }
    postgresToolReviewRequested = true
  }

  func startPostgresTool() async {
    postgresToolReviewRequested = false
    guard let client, let session = sessionData, let tool = postgresToolProbe?.path,
      let file = postgresToolFileUrl
    else { return }
    postgresToolError = nil
    postgresToolStatus = nil
    postgresToolSecurityScopeActive = file.startAccessingSecurityScopedResource()
    do {
      let operation = try await client.startPostgresTool(
        sessionId: session, kind: postgresToolKind, toolPath: tool, filePath: file.path,
        content: postgresToolContent, clean: postgresToolKind == "restore" && postgresToolClean,
        noOwner: postgresToolNoOwner)
      postgresToolStatus = WorkbenchPostgresToolStatus(
        operationId: operation, kind: postgresToolKind, phase: "running",
        summary: "Process started")
      await pollPostgresTool(operation)
    } catch {
      releasePostgresToolSecurityScope()
      postgresToolError = "PostgreSQL tool failed to start: \(error)"
    }
  }

  private func pollPostgresTool(_ operation: Data) async {
    guard let client else { return }
    while true {
      do {
        let status = try await client.postgresToolStatus(operationId: operation)
        postgresToolStatus = status
        if status.phase != "running" && status.phase != "cancel_requested" {
          releasePostgresToolSecurityScope()
          return
        }
      } catch {
        releasePostgresToolSecurityScope()
        postgresToolError = "PostgreSQL tool status failed: \(error)"
        return
      }
      try? await Task.sleep(for: .milliseconds(200))
    }
  }

  func cancelPostgresTool() async {
    guard let client, let operation = postgresToolStatus?.operationId else { return }
    do {
      if try await client.cancelPostgresTool(operationId: operation) {
        postgresToolStatus = WorkbenchPostgresToolStatus(
          operationId: operation, kind: postgresToolKind, phase: "cancel_requested",
          summary: "Cancellation requested")
      }
    } catch { postgresToolError = "PostgreSQL tool cancellation failed: \(error)" }
  }

  func closePostgresTools() {
    guard
      postgresToolStatus?.phase != "running"
        && postgresToolStatus?.phase != "cancel_requested"
    else { return }
    releasePostgresToolSecurityScope()
    postgresToolsPresented = false
  }

  private func releasePostgresToolSecurityScope() {
    if postgresToolSecurityScopeActive, let file = postgresToolFileUrl {
      file.stopAccessingSecurityScopedResource()
    }
    postgresToolSecurityScopeActive = false
  }


  private var hasUnsavedEditorText: Bool { queryText != sqlFileBaseline }

  func requestOpenSqlFile() {
    if hasUnsavedEditorText {
      confirmDiscardForOpen = true
    } else {
      Task { await openSqlFile() }
    }
  }

  func openSqlFile() async {
    confirmDiscardForOpen = false
    guard
      let url = dependencies.filePanels.chooseOpenFile(
        AppFilePanelRequest(
          title: "Open SQL File", prompt: "Open", allowedExtensions: ["sql"]
        )), let client
    else { return }
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let file = try await client.readSqlFile(path: url.path)
      sqlFile = file
      sqlFileBaseline = file.statementText
      queryText = file.statementText
      sqlFileError = nil
      profileActionOutcome = "Opened \(url.lastPathComponent)"
    } catch { sqlFileError = "Open SQL file failed: \(error)" }
  }

  func saveSqlFile(saveAs: Bool = false, overwriteExternalChange: Bool = false) async {
    guard let client else { return }
    var url = sqlFile.map { URL(fileURLWithPath: $0.path) }
    if saveAs || url == nil {
      guard
        let selected = dependencies.filePanels.chooseSaveFile(
          AppFilePanelRequest(
            title: "Save SQL File", prompt: "Save", suggestedFilename: "query.sql",
            allowedExtensions: ["sql"]
          ))
      else { return }
      url =
        selected.pathExtension == "sql"
        ? selected : selected.appendingPathExtension("sql")
    }
    guard let url else { return }
    let sameFile = !saveAs && sqlFile?.path == url.path
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let written = try await client.writeSqlFile(
        path: url.path,
        statement: queryText,
        expectedModifiedNanos: sameFile ? sqlFile?.modifiedNanos : nil,
        expectedLength: sameFile ? sqlFile?.len : nil,
        overwriteExternalChange: overwriteExternalChange
      )
      sqlFile = written
      sqlFileBaseline = queryText
      sqlFileError = nil
      confirmExternalOverwrite = false
      profileActionOutcome = "Saved \(url.lastPathComponent)"
    } catch let error as BridgeError {
      if case .Rejected(code: "sql-file-external-change", message: _) = error {
        confirmExternalOverwrite = true
      } else {
        sqlFileError = "Save SQL file failed: \(error)"
      }
    } catch { sqlFileError = "Save SQL file failed: \(error)" }
  }

  func reloadSqlFile() async {
    guard let file = sqlFile, let client else { return }
    let url = URL(fileURLWithPath: file.path)
    let accessed = url.startAccessingSecurityScopedResource()
    defer { if accessed { url.stopAccessingSecurityScopedResource() } }
    do {
      let loaded = try await client.readSqlFile(path: file.path)
      sqlFile = loaded
      sqlFileBaseline = loaded.statementText
      queryText = loaded.statementText
      sqlFileError = nil
      confirmExternalOverwrite = false
      profileActionOutcome = "Reloaded \(url.lastPathComponent)"
    } catch { sqlFileError = "Reload SQL file failed: \(error)" }
  }


}

/// Sidebar profile list: groups, search, Sample/New chrome, empty/loading/error.
