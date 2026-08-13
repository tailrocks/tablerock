import AppKit
import Observation
import SwiftUI
import TableRockFeature

@MainActor
@Observable
public final class WorkbenchPresentationStore {
  let windowId: UUID
  var status: String = "starting…"
  var bridgeError: String?
  var profiles: [WorkbenchProfileItem] = []
  var profileGroups: [WorkbenchProfileGroup] = []
  var connectionWorkspaceSurface: ConnectionWorkspaceSurface = .connections
  var profileEditorPresentation: ProfileEditorPresentation = .sheet
  var profileEditorSheetPresented = false
  var directConnectionPresented = false
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
  #if TABLEROCK_DEVELOPMENT_SUPPORT
    var externalUrlFixtureConsumed = false
  #endif
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
  var historyLoading = false
  var historyError: String?
  var historyRetention = "full"
  var historyGeneration: UInt64 = 0
  var savedQueriesPresented = false
  var savedQuerySearch = ""
  var savedQueryEngine = ""
  var savedQueries: [WorkbenchSavedQueryItem] = []
  var savedQueriesLoading = false
  var savedQueriesError: String?
  var savedQueriesGeneration: UInt64 = 0
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
  var csvImportUrl: URL?
  var csvImportOperationId: Data?
  var streamExportPresented = false
  var streamExportProgress: WorkbenchStreamExportProgress?
  var streamExportError: String?
  var streamExportOperationId: Data?
  var redisOverviewPresented = false
  var redisOverview: WorkbenchRedisOverview?
  var redisOverviewLoading = false
  var redisOverviewError: String?
  var redisSubscriptionPresented = false
  var redisSubscriptionSelector = ""
  var redisSubscriptionPattern = false
  var redisSubscriptionStatus: WorkbenchRedisSubscriptionStatus?
  var redisSubscriptionError: String?
  var redisSubscriptionStarting = false
  var redisSubscriptionPollTask: Task<Void, Never>?
  var ddlChangePresented = false
  var ddlChangeKind = "add_column"
  var ddlChangeObjectName = ""
  var ddlChangeDefinition = ""
  var ddlChangeReview: WorkbenchDdlChangeReview?
  var ddlChangeOutcome: String?
  var ddlChangeError: String?
  var ddlChangeApplying = false
  var ddlChangeCatalogNodeId: Data?
  var tableOperationPresented = false
  var tableOperationKind = "truncate"
  var tableOperationNewName = ""
  var tableOperationConfirmation = ""
  var tableOperationReview: WorkbenchTableOperationReview?
  var tableOperationStatus: WorkbenchTableOperationStatus?
  var tableOperationOutcome: String?
  var tableOperationError: String?
  var tableOperationApplying = false
  var tableOperationCatalogNodeId: Data?
  var tableOperationId: Data?
  var rowEditDraft: NativeRowEditDraft?
  var mutationReviewPresented = false
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
  var postgresActivityLoading = false
  var postgresActivityError: String?
  var postgresActivityOutcome: String?
  var postgresRelationshipsPresented = false
  var postgresRelationshipSnapshot: WorkbenchRelationshipSnapshot?
  var postgresRelationshipsLoading = false
  var postgresRelationshipsError: String?
  var postgresRolesPresented = false
  var postgresRoleSnapshot: WorkbenchRoleSnapshot?
  var postgresRoleSearch = ""
  var postgresRolesLoading = false
  var postgresRolesError: String?
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
  var postgresToolSecurityScopeActive = false
  var queryTabs: [NativeQueryTab]
  var selectedQueryTabId: UUID
  var objectTabs: [NativeObjectTab] = []
  var workspaceTabOrder: [NativeWorkspaceTabReference] = []
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

  /// Pending ledger entries visible to presentation.
  var changeLedgerEntryCount: Int {
    var n = objectTabs.reduce(0) { $0 + ($1.mutationReview == nil ? 0 : 1) }
    if ddlChangeReview != nil { n += 1 }
    if tableOperationReview != nil { n += 1 }
    if csvImportReview != nil { n += 1 }
    if postgresRoleChangeReview != nil { n += 1 }
    return n
  }

  var changeReviewOpen: Bool {
    objectTabs.contains(where: { $0.mutationReview != nil }) || ddlChangeReview != nil
      || tableOperationReview != nil || csvImportReview != nil
      || postgresRoleChangeReview != nil
  }
  var activeQueryTab: NativeQueryTab {
    queryTabs.first(where: { $0.id == selectedQueryTabId }) ?? queryTabs[0]
  }
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
      relationContinuum = nil
      relationContinuumError = nil
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
    queryStateRevision &+= 1
  }

  func toggleValueInspector() {
    if selectedCell != nil {
      selectedCell = nil
    } else if let table = resultTable, !table.rows.isEmpty, !table.columns.isEmpty {
      selectCell(row: 0, column: 0)
    }
  }

  func presentActiveReview() {
    if activeObjectTab?.mutationReview != nil {
      mutationReviewPresented = true
    } else if ddlChangeReview != nil {
      ddlChangePresented = true
    } else if tableOperationReview != nil {
      tableOperationPresented = true
    } else if csvImportReview != nil {
      csvImportPresented = true
    } else if postgresRoleChangeReview != nil {
      postgresRolesPresented = true
    }
  }

  var relationContinuum: RelationContinuumState?
  var relationContinuumError: String?
  var relationContinuumLoading = false
  var queryWorkbenchSelected: Bool { selectedWorkbenchKind == "query" }
  var orderedWorkspaceTabs: [NativeWorkspaceTabReference] {
    let queryIds = Set(queryTabs.map(\.id))
    let objectIds = Set(objectTabs.map(\.id))
    var seen = Set<NativeWorkspaceTabReference>()
    var result = workspaceTabOrder.filter { reference in
      let exists =
        switch reference {
        case .query(let id): queryIds.contains(id)
        case .object(let id): objectIds.contains(id)
        }
      return exists && seen.insert(reference).inserted
    }
    result.append(
      contentsOf: queryTabs.compactMap { tab in
        let reference = NativeWorkspaceTabReference.query(tab.id)
        return seen.insert(reference).inserted ? reference : nil
      })
    result.append(
      contentsOf: objectTabs.compactMap { tab in
        let reference = NativeWorkspaceTabReference.object(tab.id)
        return seen.insert(reference).inserted ? reference : nil
      })
    return result
  }
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
  var sqlFileBaseline: String {
    get { activeQueryTab.sqlFileBaseline }
    set { activeQueryTab.sqlFileBaseline = newValue }
  }
  var confirmDiscardForOpen = false
  var confirmExternalOverwrite = false
  var sqlFileError: String? {
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
  #if TABLEROCK_DEVELOPMENT_SUPPORT
    let fixtures: NativeWorkbenchFixtureConfiguration
  #endif
  /// Operator data root used for sample SQLite + isolation.
  let dataRootPath: String
  var sessionData: Data?
  var queryStateRevision: UInt64 = 0

  #if TABLEROCK_DEVELOPMENT_SUPPORT
    init(
      client: (any WorkbenchBackend)? = nil,
      startupError: String? = nil,
      windowId: UUID? = nil,
      dependencies: AppDependencies = AppDependencies(),
      dataRootPath: String = FileManager.default.temporaryDirectory.path,
      fixtures: NativeWorkbenchFixtureConfiguration
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
      workspaceTabOrder = [.query(tab.id)]
      installPerformanceFixtureIfRequested()
    }

    public convenience init(
      client: (any WorkbenchBackend)? = nil,
      startupError: String? = nil,
      windowId: UUID? = nil,
      dependencies: AppDependencies = AppDependencies(),
      dataRootPath: String = FileManager.default.temporaryDirectory.path
    ) {
      self.init(
        client: client,
        startupError: startupError,
        windowId: windowId,
        dependencies: dependencies,
        dataRootPath: dataRootPath,
        fixtures: .current
      )
    }
  #else
    public init(
      client: (any WorkbenchBackend)? = nil,
      startupError: String? = nil,
      windowId: UUID? = nil,
      dependencies: AppDependencies = AppDependencies(),
      dataRootPath: String = FileManager.default.temporaryDirectory.path
    ) {
      self.client = client
      self.startupError = startupError
      self.dependencies = dependencies
      self.dataRootPath = dataRootPath
      self.windowId = windowId ?? dependencies.identifiers.next()
      let tab = NativeQueryTab(
        id: dependencies.identifiers.next(), title: "Query 1", statementText: "SELECT 1;"
      )
      queryTabs = [tab]
      selectedQueryTabId = tab.id
      workspaceTabOrder = [.query(tab.id)]
    }
  #endif

  public func initialize() async {
    #if TABLEROCK_DEVELOPMENT_SUPPORT
      if fixtures.nativeWorkbenchConnections || fixtures.nativeWorkbenchSetup {
        installNativeConnectionFixture(setup: fixtures.nativeWorkbenchSetup)
        return
      }
      if fixtures.nativeWorkbench || fixtures.nativeWorkbenchQuery
        || fixtures.nativeWorkbenchStructure || fixtures.nativeWorkbenchSafeEdit
        || fixtures.nativeWorkbenchSafeReview
        || fixtures.nativeWorkbenchDestructiveReview || fixtures.nativeWorkbenchEngine != nil
        || fixtures.nativeWorkbenchState != nil
      {
        installNativeWorkbenchFixture(
          selectsQuery: fixtures.nativeWorkbenchQuery,
          engine: fixtures.nativeWorkbenchEngine ?? .postgresql,
          state: fixtures.nativeWorkbenchState ?? .populated)
        if fixtures.nativeWorkbenchStructure {
          installNativeWorkbenchStructureFixture()
        } else if fixtures.nativeWorkbenchSafeEdit {
          installNativeWorkbenchSafeEditFixture()
          await presentNativeWorkbenchFixtureReview(destructive: false)
        } else if fixtures.nativeWorkbenchSafeReview {
          installNativeWorkbenchReviewFixture(destructive: false)
          await presentNativeWorkbenchFixtureReview(destructive: false)
        } else if fixtures.nativeWorkbenchDestructiveReview {
          installNativeWorkbenchReviewFixture(destructive: true)
          await presentNativeWorkbenchFixtureReview(destructive: true)
        }
        return
      }
      if fixtures.multiWindow {
        let other = WorkbenchPresentationStore(
          client: client, dependencies: dependencies, fixtures: fixtures)
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
        sessionData = Data(repeating: 5, count: 16)
        sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
        connectedEngine = "postgresql"
        selectedWorkbenchKind = "query"
        activeQueryTab.statementText = "SELECT "
        status = "Preparing IME fixture"
        try? await Task.sleep(for: .milliseconds(500))
        guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView
        else {
          status = "IME fixture failed: no window"
          activeQueryTab.querySummary = status
          return
        }
        func descendants(of view: NSView) -> [NSView] {
          [view] + view.subviews.flatMap(descendants)
        }
        guard let editor = descendants(of: root).compactMap({ $0 as? NSTextView }).first else {
          status = "IME fixture failed: no editor"
          activeQueryTab.querySummary = status
          return
        }
        editor.window?.makeFirstResponder(editor)
        editor.setSelectedRange(NSRange(location: editor.string.utf16.count, length: 0))
        editor.setMarkedText(
          "かな", selectedRange: NSRange(location: 2, length: 0),
          replacementRange: NSRange(location: NSNotFound, length: 0))
        guard editor.hasMarkedText() else {
          status = "IME fixture failed: no marked text"
          activeQueryTab.querySummary = status
          return
        }
        let composed = editor.string
        activeQueryTab.statementText = "model update must not replace composition"
        try? await Task.sleep(for: .milliseconds(250))
        guard editor.hasMarkedText(), editor.string == composed else {
          status = "IME fixture failed: composition replaced"
          activeQueryTab.querySummary = status
          writePerformanceMetric("IME_PROOF_FAILED composition_replaced=true")
          return
        }
        status = "IME composition preserved"
        activeQueryTab.querySummary = status
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
          guard let structure = tab.structure,
            structure.columns.count == 3,
            structure.indexes.contains(where: { $0.name == "structure_probe_pkey" }),
            structure.constraints.contains(where: { $0.name == "structure_probe_name_check" })
          else {
            writePerformanceMetric(
              "STRUCTURE_PROOF_FAILED \(tab.structureError ?? "snapshot mismatch")"
            )
            return
          }
          copyStructureDdl(structure.ddl)
          try? await Task.sleep(for: .milliseconds(500))
          runNativeStructureAudit(structure: structure)
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
          guard let structure = tab.structure,
            structure.engine == "clickhouse",
            structure.columns.count == 3,
            structure.columns.first(where: { $0.name == "id" })?.primaryKey == true,
            structure.columns.first(where: { $0.name == "id" })?.sortingKey == true,
            structure.facts.contains(where: {
              $0.name == "Engine" && $0.value == "MergeTree"
            })
          else {
            writePerformanceMetric(
              "CLICKHOUSE_STRUCTURE_PROOF_FAILED \(tab.structureError ?? "snapshot mismatch")"
            )
            return
          }
          copyStructureDdl(structure.ddl)
          try? await Task.sleep(for: .milliseconds(500))
          runNativeClickHouseStructureAudit(structure: structure)
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
          guard let redisView = activeObjectTab?.redisView,
            redisView.kind == "hash", redisView.lines.count > 34
          else {
            writePerformanceMetric(
              "REDIS_KEY_VIEW_PROOF_FAILED native view kind=\(activeObjectTab?.redisView?.kind ?? "nil") lines=\(activeObjectTab?.redisView?.lines.count ?? 0) next=\(String(describing: activeObjectTab?.redisView?.nextSkip))"
            )
            return
          }
          try? await Task.sleep(for: .milliseconds(500))
          runNativeRedisKeyViewAudit(view: redisView)
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
    #endif
    guard let client else {
      bridgeError = startupError ?? "Bridge unavailable"
      status = "error"
      return
    }
    #if TABLEROCK_DEVELOPMENT_SUPPORT
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
    #endif
    do {
      historyRetention = try await client.historyRetention()
      await refreshProfiles()
      await restoreWindowIntentOnLaunch()
    } catch {
      bridgeError = "Bridge init failed: \(error)"
      status = "error"
    }
  }

  #if TABLEROCK_DEVELOPMENT_SUPPORT
    private func installNativeConnectionFixture(setup: Bool) {
      profiles = [
        WorkbenchProfileItem(
          idBytes: Data(repeating: 4, count: 16), revision: 1,
          name: "Northstar Analytics", engine: "postgresql", group: "Production",
          favorite: true, savedOrder: 0, host: "analytics.internal", port: "5432",
          context: "analytics", safetyMode: "confirm_writes", environment: "production",
          productionWarning: true, dangerousPlaintext: false, connected: false
        ),
        WorkbenchProfileItem(
          idBytes: Data(repeating: 5, count: 16), revision: 1,
          name: "Atlas Events", engine: "clickhouse", group: "Staging",
          favorite: true, savedOrder: 1, host: "events.cluster", port: "9440",
          context: "events", safetyMode: "read_only", environment: "staging",
          productionWarning: false, dangerousPlaintext: false, connected: false
        ),
        WorkbenchProfileItem(
          idBytes: Data(repeating: 6, count: 16), revision: 1,
          name: "Arbor Cache", engine: "redis", group: "Local",
          favorite: true, savedOrder: 2, host: "127.0.0.1", port: "6379",
          context: "0", safetyMode: "read_only", environment: "development",
          productionWarning: false, dangerousPlaintext: false, connected: false
        ),
      ]
      profileGroups = [
        WorkbenchProfileGroup(name: "Production", alphabetical: true),
        WorkbenchProfileGroup(name: "Staging", alphabetical: true),
        WorkbenchProfileGroup(name: "Local", alphabetical: true),
      ]
      status = "Native Workbench connection fixture"
      guard setup else {
        connectionWorkspaceSurface = .connections
        return
      }
      profileEditorPresentation = .workspace
      profileEditorSheetPresented = false
      connectionWorkspaceSurface = .setup
      editorDraft = ProfileEditorDraft(
        WorkbenchProfileDraft(
          idBytes: nil, revision: 0, engine: "postgresql",
          name: "Northstar Analytics", group: "Production", environment: "production",
          host: "analytics.internal", port: "5432", database: "analytics",
          username: "table_operator", passwordSource: "prompt", passwordValue: "",
          passwordReference: nil, hasStoredPassword: false, plaintextAcknowledged: false,
          tlsMode: "verify_full", safetyMode: "confirm_writes"
        )
      )
    }

    private func installNativeWorkbenchFixture(
      selectsQuery: Bool,
      engine: NativeWorkbenchFixtureEngine = .postgresql,
      state: NativeWorkbenchFixtureState = .populated
    ) {
      if engine == .redis {
        installNativeWorkbenchRedisFixture(selectsQuery: selectsQuery, state: state)
        return
      }

      let clickHouse = engine == .clickhouse
      let objectName =
        state == .longIdentifiers
        ? "customer_engagement_retention_cohort_materialized_rollup_by_billing_region"
        : "customers"
      let tableKind = clickHouse ? "clickhouse_table" : "postgresql_table"
      let viewKind = clickHouse ? "clickhouse_view" : "postgresql_view"
      let materializedViewKind =
        clickHouse ? "clickhouse_materialized_view" : "postgresql_materialized_view"
      let specializedKind = clickHouse ? "clickhouse_dictionary" : "postgresql_function"
      let rootId = Data(repeating: 6, count: 16)
      let customersNode = WorkbenchCatalogNode(
        idBytes: Data(repeating: 7, count: 16), parentIdBytes: rootId,
        depth: 1, name: objectName, kind: tableKind,
        childrenState: "not_applicable", expandable: false)
      let objects = [
        customersNode,
        WorkbenchCatalogNode(
          idBytes: Data(repeating: 8, count: 16), parentIdBytes: rootId,
          depth: 1, name: "invoices", kind: tableKind,
          childrenState: "not_applicable", expandable: false),
        WorkbenchCatalogNode(
          idBytes: Data(repeating: 9, count: 16), parentIdBytes: rootId,
          depth: 1, name: "orders", kind: tableKind,
          childrenState: "not_applicable", expandable: false),
        WorkbenchCatalogNode(
          idBytes: Data(repeating: 10, count: 16), parentIdBytes: rootId,
          depth: 1, name: "products", kind: tableKind,
          childrenState: "not_applicable", expandable: false),
        WorkbenchCatalogNode(
          idBytes: Data(repeating: 12, count: 16), parentIdBytes: rootId,
          depth: 1, name: "regions", kind: viewKind,
          childrenState: "not_applicable", expandable: false),
        WorkbenchCatalogNode(
          idBytes: Data(repeating: 13, count: 16), parentIdBytes: rootId,
          depth: 1, name: "revenue_summary", kind: materializedViewKind,
          childrenState: "not_applicable", expandable: false),
        WorkbenchCatalogNode(
          idBytes: Data(repeating: 14, count: 16), parentIdBytes: rootId,
          depth: 1, name: clickHouse ? "customer_segments" : "calculate_retention(integer)",
          kind: specializedKind,
          childrenState: "not_applicable", expandable: false),
      ]
      catalogSnapshot =
        [
          WorkbenchCatalogNode(
            idBytes: rootId, parentIdBytes: nil, depth: 0,
            name: "analytics", kind: clickHouse ? "clickhouse_database" : "postgresql_schema",
            childrenState: "loaded_complete", expandable: true)
        ] + objects
      catalogRefreshState = .loaded
      catalogSelection = catalogNodeKey(customersNode.idBytes)

      let profile = WorkbenchProfileItem(
        idBytes: Data(repeating: 4, count: 16), revision: 1,
        name: clickHouse ? "Atlas Events" : "Northstar Analytics", engine: engine.rawValue,
        group: nil, favorite: true, savedOrder: 0,
        host: clickHouse ? "events.cluster" : "db.internal",
        port: clickHouse ? "9440" : "5432",
        context: clickHouse ? "analytics.events" : "analytics.public",
        safetyMode: clickHouse ? "read_only" : "confirm_writes", environment: "development",
        productionWarning: false, dangerousPlaintext: false, connected: true)
      profiles = [profile]
      activeProfileId = profile.idBytes
      sessionData = Data(repeating: 1, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      sessionHealth = WorkbenchSessionHealth(
        state: "healthy", serverReachable: true, elapsedMillis: 18,
        authenticationStopped: false)
      connectedEngine = engine.rawValue

      let customers = NativeObjectTab(
        id: dependencies.identifiers.next(), node: customersNode, pinned: true)
      customers.resultTable = WorkbenchTable(
        columns: [
          "customer_id", "company_name", "region", "plan", "seats",
          "monthly_revenue", "active", "updated_at",
        ],
        rows: [
          ["…8f31", "Fieldstone", "AMER", "Scale", "208", "$31,200", "true", "2026-08-12"],
          ["…77c0", "Ion Foundry", "AMER", "Scale", "156", "$23,400", "true", "2026-08-12"],
          ["…a91f", "Aster Works", "APAC", "Scale", "124", "$18,600", "true", "2026-08-11"],
          ["…c773", "Cedar Systems", "AMER", "Scale", "92", "$13,800", "true", "2026-08-11"],
          ["…09d6", "Juniper Cloud", "APAC", "Team", "64", "$8,000", "true", "2026-08-10"],
          ["…339a", "Lumen River", "AMER", "Team", "51", "$6,375", "true", "2026-08-10"],
          ["…a602", "Ember Labs", "EMEA", "Team", "46", "$5,750", "true", "2026-08-09"],
          ["…28cb", "Beacon & Co.", "EMEA", "Team", "38", "$4,750", "true", "2026-08-09"],
          ["…1a0d", "Grove Digital", "APAC", "Team", "27", "$3,375", "true", "2026-08-08"],
          ["…ef24", "Kitehouse", "EMEA", "Starter", "16", "$720", "false", "2026-08-08"],
          ["…4de2", "Driftline", "APAC", "Starter", "12", "$540", "false", "2026-08-07"],
          ["…b8e4", "Harbor North", "EMEA", "Starter", "8", "$360", "true", "2026-08-07"],
        ],
        columnMetadata: [
          WorkbenchColumn(name: "customer_id", engine: 0, engineType: "uuid", nullable: false),
          WorkbenchColumn(
            name: "company_name", engine: 0, engineType: clickHouse ? "String" : "text",
            nullable: false),
          WorkbenchColumn(
            name: "region", engine: 0,
            engineType: clickHouse ? "LowCardinality(String)" : "text", nullable: false),
          WorkbenchColumn(
            name: "plan", engine: 0,
            engineType: clickHouse ? "LowCardinality(String)" : "text", nullable: false),
          WorkbenchColumn(
            name: "seats", engine: 0, engineType: clickHouse ? "UInt32" : "int4",
            nullable: false),
          WorkbenchColumn(
            name: "monthly_revenue", engine: 0,
            engineType: clickHouse ? "Decimal(12,2)" : "numeric", nullable: false),
          WorkbenchColumn(
            name: "active", engine: 0, engineType: clickHouse ? "Bool" : "bool",
            nullable: false),
          WorkbenchColumn(
            name: "updated_at", engine: 0,
            engineType: clickHouse ? "DateTime64(3)" : "timestamptz", nullable: true),
        ])
      customers.resultIdData = Data(repeating: 8, count: 16)
      customers.resultRevision = 1
      customers.mutationEditability = WorkbenchMutationEditability(
        editable: !clickHouse, reason: clickHouse ? "engine edit flow unavailable" : nil,
        identityColumns: clickHouse ? [] : ["customer_id"])
      customers.summary = "48,224 rows · 41 ms"
      customers.filters = [
        WorkbenchBrowseFilter(
          id: dependencies.identifiers.next(), column: "active", operatorName: "eq", value: "true"),
        WorkbenchBrowseFilter(
          id: dependencies.identifiers.next(), column: "region", operatorName: "ne", value: "LATAM"),
      ]
      customers.sort = [WorkbenchBrowseSort(column: "monthly_revenue", descending: true)]
      customers.selectedCell = NativeCellSelection(row: 2, column: 1)

      let ordersNode = objects[2]
      let orders = NativeObjectTab(
        id: dependencies.identifiers.next(), node: ordersNode, pinned: true)
      orders.resultTable = WorkbenchTable(
        columns: ["order_id", "status"], rows: [["1001", "open"]])
      objectTabs = [customers, orders]
      activeQueryTab.title = "Revenue by region"
      activeQueryTab.statementText =
        """
        SELECT
          company_name,
          region,
          plan,
          monthly_revenue
        FROM analytics.customers
        WHERE active = true
          AND monthly_revenue >= 5_000
        ORDER BY monthly_revenue DESC
        LIMIT 100;
        """
      activeQueryTab.resultTable = customers.resultTable
      activeQueryTab.resultIdData = Data(repeating: 15, count: 16)
      activeQueryTab.resultRevision = 1
      activeQueryTab.querySummary = "100 rows · 126 ms · success"
      workspaceTabOrder = [.object(customers.id), .query(activeQueryTab.id), .object(orders.id)]
      selectedObjectTabId = customers.id
      selectedWorkbenchKind = selectsQuery ? "query" : "object"
      installNativeWorkbenchStateFixture(state)
      status = "Native Workbench \(engine.rawValue) \(state.rawValue) fixture"
    }

    private func installNativeWorkbenchStateFixture(_ state: NativeWorkbenchFixtureState) {
      guard let tab = selectedObjectTab else { return }
      switch state {
      case .populated, .longIdentifiers:
        break
      case .empty:
        if let table = tab.resultTable {
          let empty = WorkbenchTable(
            columns: table.columns, rows: [], columnMetadata: table.columnMetadata, cells: [])
          tab.resultTable = empty
          activeQueryTab.resultTable = empty
        }
        tab.selectedCell = nil
        tab.summary = "0 rows · 12 ms"
      case .loading:
        tab.resultTable = nil
        tab.selectedCell = nil
        tab.isRunning = true
        tab.summary = nil
        catalogSnapshot = nil
        catalogRefreshState = .loading(nodeKey: nil)
      case .connectionError:
        tab.resultTable = nil
        tab.selectedCell = nil
        tab.summary = nil
        tab.error = "Connection lost while loading this object. Reconnect, then retry."
        catalogSnapshot = nil
        catalogRefreshState = .failed(
          message: "Server unavailable. Check the host and TLS settings.")
        sessionHealth = WorkbenchSessionHealth(
          state: "unavailable", serverReachable: false, elapsedMillis: nil,
          authenticationStopped: false)
      case .largeResult:
        guard let table = tab.resultTable else { return }
        let rows = (0..<1_500).map { index in
          let region = ["AMER", "APAC", "EMEA"][index % 3]
          let plan = ["Scale", "Team", "Starter"][index % 3]
          let active = index.isMultiple(of: 7) ? "false" : "true"
          let day = String(format: "%02d", 1 + index % 12)
          return [
            String(format: "…%04x", index), "Customer \(index + 1)", region, plan,
            String(8 + index % 240), "$\((index + 1) * 125)", active, "2026-08-\(day)",
          ]
        }
        let large = WorkbenchTable(
          columns: table.columns, rows: rows, columnMetadata: table.columnMetadata)
        tab.resultTable = large
        activeQueryTab.resultTable = large
        tab.selectedCell = NativeCellSelection(row: 2, column: 1)
        tab.summary = "1,500 of 48,224 rows · 86 ms"
      case .queryError:
        selectedWorkbenchKind = "query"
        activeQueryTab.resultTable = nil
        activeQueryTab.statementText =
          """
          SELECT
            company_name,
            region,
            plan,
            monthly_revenue_total
          FROM analytics.customers
          WHERE active = true
            AND monthly_revenue >= 5_000
          ORDER BY monthly_revenue DESC
          LIMIT 100;
          """
        activeQueryTab.queryError =
          "Column monthly_revenue_total does not exist at line 5, column 3."
        activeQueryTab.querySummary = nil
        activeQueryTab.selectedResultSection = "messages"
      }
    }

    private func installNativeWorkbenchRedisFixture(
      selectsQuery: Bool,
      state: NativeWorkbenchFixtureState
    ) {
      let databaseId = Data(repeating: 6, count: 16)
      let namespaceId = Data(repeating: 16, count: 16)
      let keyName =
        state == .longIdentifiers
        ? "customer:session:regional-rollout:experiment-assignment:2026-08-13:apac"
        : "customer:session:10482"
      let keyNode = WorkbenchCatalogNode(
        idBytes: Data(repeating: 7, count: 16), parentIdBytes: namespaceId,
        depth: 2, name: keyName, kind: "redis_key_hash",
        childrenState: "not_applicable", expandable: false)
      let secondKey = WorkbenchCatalogNode(
        idBytes: Data(repeating: 8, count: 16), parentIdBytes: namespaceId,
        depth: 2, name: "customer:feature-flags", kind: "redis_key_set",
        childrenState: "not_applicable", expandable: false)
      catalogSnapshot = [
        WorkbenchCatalogNode(
          idBytes: databaseId, parentIdBytes: nil, depth: 0,
          name: "db0", kind: "redis_logical_database",
          childrenState: "loaded_complete", expandable: true),
        WorkbenchCatalogNode(
          idBytes: namespaceId, parentIdBytes: databaseId, depth: 1,
          name: "customer", kind: "redis_namespace",
          childrenState: "loaded_complete", expandable: true),
        keyNode,
        secondKey,
      ]
      catalogRefreshState = .loaded
      catalogSelection = catalogNodeKey(keyNode.idBytes)

      let profile = WorkbenchProfileItem(
        idBytes: Data(repeating: 6, count: 16), revision: 1,
        name: "Arbor Cache", engine: "redis", group: nil,
        favorite: true, savedOrder: 0, host: "127.0.0.1", port: "6379",
        context: "database 0", safetyMode: "read_only", environment: "development",
        productionWarning: false, dangerousPlaintext: false, connected: true)
      profiles = [profile]
      activeProfileId = profile.idBytes
      sessionData = Data(repeating: 3, count: 16)
      sessionHex = sessionData?.map { String(format: "%02x", $0) }.joined()
      sessionHealth = WorkbenchSessionHealth(
        state: "healthy", serverReachable: true, elapsedMillis: 3,
        authenticationStopped: false)
      connectedEngine = "redis"

      let keyTab = NativeObjectTab(
        id: dependencies.identifiers.next(), node: keyNode, pinned: true)
      keyTab.redisView = WorkbenchRedisKeyView(
        kind: "hash",
        lines: [
          "type: Hash",
          "ttl: ExpiresIn(3600s)",
          "HSCAN page skip=0 take=7 (end)",
          "customer_id = …a91f",
          "company_name = Aster Works",
          "region = APAC",
          "plan = Scale",
          "active = true",
          "last_seen_at = 2026-08-13T09:42:18Z",
          "feature_flags = billing-v2, insights",
        ],
        nextSkip: nil)
      keyTab.summary = "hash · 7 fields · TTL 1h"
      objectTabs = [keyTab]
      selectedObjectTabId = keyTab.id

      activeQueryTab.title = "Redis command"
      activeQueryTab.statementText = "SCAN 0 MATCH customer:* COUNT 100"
      activeQueryTab.resultTable = WorkbenchTable(
        columns: ["cursor", "key"],
        rows: [["0", keyName], ["0", secondKey.name]],
        columnMetadata: [
          WorkbenchColumn(name: "cursor", engine: 2, engineType: "integer", nullable: false),
          WorkbenchColumn(name: "key", engine: 2, engineType: "bytes", nullable: false),
        ])
      activeQueryTab.querySummary = "2 keys · 4 ms · complete"
      workspaceTabOrder = [.object(keyTab.id), .query(activeQueryTab.id)]
      selectedWorkbenchKind = selectsQuery ? "query" : "object"

      switch state {
      case .populated, .longIdentifiers:
        break
      case .empty:
        keyTab.redisView = WorkbenchRedisKeyView(
          kind: "hash",
          lines: [
            "type: Hash", "ttl: Persistent", "HSCAN page skip=0 take=0 (end)", "HSCAN: empty",
          ],
          nextSkip: nil)
        keyTab.summary = "hash · empty"
      case .loading:
        keyTab.redisView = nil
        keyTab.isRunning = true
        catalogSnapshot = nil
        catalogRefreshState = .loading(nodeKey: nil)
      case .connectionError:
        keyTab.redisView = nil
        keyTab.summary = nil
        keyTab.error = "Redis connection closed while reading this key. Reconnect, then retry."
        catalogSnapshot = nil
        catalogRefreshState = .failed(
          message: "Redis server unavailable at 127.0.0.1:6379.")
        sessionHealth = WorkbenchSessionHealth(
          state: "unavailable", serverReachable: false, elapsedMillis: nil,
          authenticationStopped: false)
      case .largeResult:
        keyTab.redisView = WorkbenchRedisKeyView(
          kind: "hash",
          lines: ["type: Hash", "ttl: Persistent", "HSCAN page skip=0 take=32 (more)"]
            + (0..<32).map { "field-\($0) = value-\($0)" },
          nextSkip: 32)
        keyTab.summary = "hash · 32+ fields"
      case .queryError:
        selectedWorkbenchKind = "query"
        activeQueryTab.statementText = "SCNA 0 MATCH customer:* COUNT 100"
        activeQueryTab.resultTable = nil
        activeQueryTab.queryError = "ERR unknown command 'SCNA'. Use SCAN for bounded key browsing."
        activeQueryTab.querySummary = nil
        activeQueryTab.selectedResultSection = "messages"
      }
      status = "Native Workbench redis \(state.rawValue) fixture"
    }

    private func installNativeWorkbenchStructureFixture() {
      guard let tab = selectedObjectTab else { return }
      tab.structure = WorkbenchRelationStructure(
        engine: "postgresql",
        namespace: "analytics.public",
        relation: "customers",
        columns: [
          WorkbenchRelationColumn(
            name: "customer_id", dataType: "UUID", nullable: false,
            defaultExpression: nil, comment: nil, primaryKey: true, sortingKey: false),
          WorkbenchRelationColumn(
            name: "company_name", dataType: "TEXT", nullable: false,
            defaultExpression: nil, comment: nil, primaryKey: false, sortingKey: false),
          WorkbenchRelationColumn(
            name: "region", dataType: "TEXT", nullable: false,
            defaultExpression: nil, comment: nil, primaryKey: false, sortingKey: false),
          WorkbenchRelationColumn(
            name: "plan", dataType: "TEXT", nullable: false,
            defaultExpression: nil, comment: nil, primaryKey: false, sortingKey: false),
          WorkbenchRelationColumn(
            name: "seats", dataType: "INT4", nullable: false,
            defaultExpression: nil, comment: nil, primaryKey: false, sortingKey: false),
          WorkbenchRelationColumn(
            name: "monthly_revenue", dataType: "NUMERIC", nullable: false,
            defaultExpression: nil, comment: nil, primaryKey: false, sortingKey: false),
          WorkbenchRelationColumn(
            name: "active", dataType: "BOOL", nullable: false,
            defaultExpression: nil, comment: nil, primaryKey: false, sortingKey: false),
          WorkbenchRelationColumn(
            name: "updated_at", dataType: "TIMESTAMPTZ", nullable: true,
            defaultExpression: "now()", comment: nil, primaryKey: false, sortingKey: false),
        ],
        indexes: [
          WorkbenchRelationIndex(
            kind: "PRIMARY", name: "customers_pkey",
            definition: "PRIMARY KEY (customer_id)"),
          WorkbenchRelationIndex(
            kind: "BTREE", name: "customers_region_idx",
            definition: "CREATE INDEX ON customers (region)"),
        ],
        constraints: [
          WorkbenchRelationConstraint(
            kind: "PRIMARY KEY", name: "customers_pkey",
            definition: "PRIMARY KEY (customer_id)"),
          WorkbenchRelationConstraint(
            kind: "CHECK", name: "customers_seats_check",
            definition: "CHECK (seats >= 0)"),
        ],
        facts: [
          WorkbenchRelationFact(name: "Persistence", value: "permanent"),
          WorkbenchRelationFact(name: "Access method", value: "heap"),
        ],
        ddl: "CREATE TABLE analytics.public.customers (…)"
      )
      tab.selectedSection = "structure"
      status = "Native Workbench structure fixture"
    }

    private func installNativeWorkbenchReviewFixture(destructive: Bool) {
      guard let tab = selectedObjectTab else { return }
      if destructive {
        let profile = WorkbenchProfileItem(
          idBytes: Data(repeating: 4, count: 16), revision: 1,
          name: "Northstar Analytics", engine: "postgresql", group: "Production",
          favorite: true, savedOrder: 0, host: "db.internal", port: "5432",
          context: "analytics.public", safetyMode: "confirm_writes",
          environment: "production", productionWarning: true,
          dangerousPlaintext: false, connected: true)
        profiles = [profile]
        activeProfileId = profile.idBytes
      }
      if destructive {
        ddlChangeCatalogNodeId = tab.catalogNodeId
        ddlChangeReview = WorkbenchDdlChangeReview(
          tokenId: Data(repeating: 19, count: 16),
          preview: "ALTER TABLE analytics.public.customers DROP COLUMN legacy_status",
          destructive: true,
          rollbackSummary: "Dropped data cannot be reconstructed automatically.",
          expiresAtMs: dependencies.clock.nowMilliseconds() + 60_000)
      } else {
        installNativeWorkbenchSafeEditFixture()
        tab.mutationReview = WorkbenchMutationReview(
          tokenId: Data(repeating: 18, count: 16), target: "analytics.customers",
          expiresAtMs: dependencies.clock.nowMilliseconds() + 60_000,
          lines: [
            WorkbenchMutationReviewLine(
              kind: "update",
              preview:
                "UPDATE \"analytics\".\"customers\" SET \"plan\" = $1, \"seats\" = $2 WHERE \"customer_id\" = $3",
              parameters: ["Enterprise", "148", "…a91f"])
          ])
      }
      status =
        destructive
        ? "Native Workbench destructive review fixture"
        : "Native Workbench safe review fixture"
    }

    private func installNativeWorkbenchSafeEditFixture() {
      guard let tab = selectedObjectTab else { return }
      rowEditDraft = NativeRowEditDraft(
        row: 2, relation: tab.title,
        fields: [
          NativeMutationField(column: "plan", kind: "text", original: "Scale", value: "Enterprise"),
          NativeMutationField(column: "seats", kind: "signed", original: "124", value: "148"),
        ])
      status = "Native Workbench safe edit fixture"
    }

    private func presentNativeWorkbenchFixtureReview(destructive: Bool) async {
      // Let SwiftUI attach and activate the owning workbench window before
      // presenting a development-only fixture sheet.
      try? await Task.sleep(for: .milliseconds(500))
      if destructive {
        ddlChangePresented = true
      } else {
        mutationReviewPresented = true
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
  #endif

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

  func refreshProfilesForConnectionSearch() async {
    #if TABLEROCK_DEVELOPMENT_SUPPORT
      if fixtures.nativeWorkbenchConnections || fixtures.nativeWorkbenchSetup { return }
    #endif
    await refreshProfiles()
  }

}

/// Sidebar profile list: groups, search, Sample/New chrome, empty/loading/error.
