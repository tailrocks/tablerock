import Foundation

public protocol WorkbenchBackend: Actor, Sendable {
  func listProfiles() throws -> [WorkbenchProfileItem]
  func searchProfiles(_ search: String?) throws -> [WorkbenchProfileItem]
  func profileDraft(id: Data) throws -> WorkbenchProfileDraft
  func parseConnectionUrl(_ input: String) throws -> WorkbenchProfileDraft
  func saveProfile(_ draft: WorkbenchProfileDraft) throws -> Data
  func deleteProfile(id: Data, revision: UInt64) throws
  func testProfile(id: Data, secretOverride: Data?) throws -> WorkbenchConnectionTestReport
  func listProfileGroups() throws -> [WorkbenchProfileGroup]
  func createProfileGroup(_ name: String) throws
  func renameProfileGroup(_ oldName: String, _ newName: String) throws -> UInt32
  func deleteProfileGroup(_ name: String) throws -> UInt32
  func setGroupAlphabetical(_ name: String, _ alphabetical: Bool) throws
  func listHistory(_ search: String?) throws -> [WorkbenchHistoryItem]
  func setHistoryRetention(_ retention: String) throws
  func historyRetention() throws -> String
  func listSavedQueries(engine: String?, search: String?) throws -> [WorkbenchSavedQueryItem]
  func saveQuery(name: String, engine: String, statement: String) throws -> Int64
  func deleteSavedQuery(_ id: Int64) throws -> Bool
  func readSqlFile(path: String) throws -> WorkbenchSQLFile
  func writeSqlFile(
    path: String, statement: String, expectedModifiedNanos: UInt64?, expectedLength: UInt64?,
    overwriteExternalChange: Bool
  ) throws -> WorkbenchSQLFile
  func putSessionIntent(profileId: Data, intent: WorkbenchSessionIntent) throws
  func sessionIntent(profileId: Data) throws -> WorkbenchSessionIntent?
  func deleteSessionIntent(profileId: Data) throws
  func putNativeWindowIntent(windowId: String, profileId: Data, intent: WorkbenchSessionIntent)
    throws
  func nativeWindowIntent(windowId: String) throws -> WorkbenchNativeWindowIntent?
  func deleteNativeWindowIntent(windowId: String) throws
  func setProfileFavorite(_ item: WorkbenchProfileItem, _ favorite: Bool) throws
  func reorderProfiles(group: String?, profiles: [WorkbenchProfileItem]) throws
  func open(params: WorkbenchOpenParams) throws -> Data
  func openProfile(id: Data, secretOverride: Data?) throws -> Data
  func disconnect(session: Data) throws
  func checkHealth(session: Data) throws -> WorkbenchSessionHealth
  func planReconnect(session: Data, attempt: UInt32, authenticationStopped: Bool) throws
    -> WorkbenchReconnectPlan
  func reconnect(session: Data, secretOverride: Data?) throws -> WorkbenchReconnectAttempt
  func refreshCatalog(session: Data, parentNodeId: Data?) throws -> [WorkbenchCatalogNode]
  func submitCatalogBrowse(
    session: Data, nodeId: Data, sort: [WorkbenchBrowseSort], filters: [WorkbenchBrowseFilter],
    rawWhere: String?
  ) throws -> Data
  func listCatalogFilterPresets(session: Data, nodeId: Data) throws
    -> [WorkbenchSavedFilterPreset]
  func saveCatalogFilterPreset(
    session: Data, nodeId: Data, preset: WorkbenchSavedFilterPreset
  ) throws
  func submit(session: Data, intent: String, statement: String?) throws -> Data
  func inspectNamedParameters(statement: String) throws -> [String]
  func submitNamed(
    session: Data, statement: String, bindings: [WorkbenchQueryParameter]
  ) throws -> Data
  func finish(operationId: Data) async throws -> WorkbenchOperation
  func cancel(operationId: Data) throws -> WorkbenchCancelOutcome
  func fetchPage(resultId: Data, startRow: UInt64, revision: UInt64) async throws -> (
    WorkbenchTable, WorkbenchPageEnvelope
  )
  func formatResultCopy(
    resultId: Data, revision: UInt64, scope: String, row: UInt64?, column: UInt32?, format: String
  ) throws -> String
  func exportLoadedResult(resultId: Data, revision: UInt64, format: String, path: String) throws
    -> UInt64
  func startStreamExport(sessionId: Data, statement: String, format: String, path: String) throws
    -> Data
  func startCatalogStreamExport(resultId: Data, revision: UInt64, format: String, path: String)
    throws -> Data
  func streamExportProgress(operationId: Data) throws -> WorkbenchStreamExportProgress
  func cancelStreamExport(operationId: Data) throws -> Bool
  func dismissStreamExport(operationId: Data) throws -> Bool
  func exportSupportBundle(path: String) throws -> UInt64
  func previewCsvImport(path: String) throws -> WorkbenchCSVImportPreview
  func stageCsvImport(
    sessionId: Data, catalogNodeId: Data, path: String, mappedColumns: [String],
    mappedTypes: [String], expectedFingerprint: String, nowMs: UInt64
  ) throws -> WorkbenchCSVImportReview
  func startCsvImportApply(tokenId: Data, nowMs: UInt64, sessionId: Data) throws -> Data
  func csvImportProgress(operationId: Data) throws -> WorkbenchCSVImportProgress
  func cancelCsvImport(operationId: Data) throws -> Bool
  func dismissCsvImport(operationId: Data) throws -> Bool
  func relationStructure(sessionId: Data, catalogNodeId: Data) throws -> WorkbenchRelationStructure
  func redisKeyView(sessionId: Data, catalogNodeId: Data, collectionSkip: UInt64) throws
    -> WorkbenchRedisKeyView
  func redisOverview(sessionId: Data) throws -> WorkbenchRedisOverview
  func startRedisSubscription(sessionId: Data, selector: String, pattern: Bool) throws -> Data
  func redisSubscriptionStatus(operationId: Data) throws -> WorkbenchRedisSubscriptionStatus
  func cancelRedisSubscription(operationId: Data) throws -> Bool
  func stageDdlChange(
    sessionId: Data, catalogNodeId: Data, kind: String, objectName: String,
    definition: String, nowMs: UInt64
  ) throws -> WorkbenchDdlChangeReview
  func applyDdlChange(tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool) throws
    -> String
  func revokeDdlChange(tokenId: Data) throws -> Bool
  func stageTableOperation(
    sessionId: Data, catalogNodeId: Data, kind: String, newName: String, nowMs: UInt64
  ) throws -> WorkbenchTableOperationReview
  func applyTableOperation(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmation: String
  ) throws -> String
  func revokeTableOperation(tokenId: Data) throws -> Bool
  func postgresActivity(sessionId: Data) throws -> [WorkbenchPostgresActivityRow]
  func postgresRelationships(sessionId: Data, catalogNodeId: Data) throws
    -> WorkbenchRelationshipSnapshot
  func postgresRoles(sessionId: Data, catalogNodeId: Data?) throws -> WorkbenchRoleSnapshot
  func stagePostgresRoleChange(
    sessionId: Data, catalogNodeId: Data?, kind: String, role: String,
    memberOrGrantee: String, privilege: String, nowMs: UInt64
  ) throws -> WorkbenchRoleChangeReview
  func applyPostgresRoleChange(
    tokenId: Data, sessionId: Data, nowMs: UInt64, confirmed: Bool
  ) throws -> String
  func revokePostgresRoleChange(tokenId: Data) throws -> Bool
  func signalPostgresBackend(sessionId: Data, kind: String, pid: Int32) throws
    -> WorkbenchBackendSignalOutcome
  func probePostgresTool(kind: String, explicitPath: String?) throws -> WorkbenchPostgresToolProbe
  func startPostgresTool(
    sessionId: Data, kind: String, toolPath: String, filePath: String, content: String,
    clean: Bool, noOwner: Bool
  ) throws -> Data
  func postgresToolStatus(operationId: Data) throws -> WorkbenchPostgresToolStatus
  func cancelPostgresTool(operationId: Data) throws -> Bool
  func applyReviewToken(tokenId: Data, nowMs: UInt64, sessionId: Data) throws
    -> WorkbenchApplyOutcome
  func revokeReviewToken(tokenId: Data) throws -> Bool
  func stageAndApply(session: Data, now: UInt64) throws -> WorkbenchApplyOutcome
}

public struct WorkbenchQueryParameter: Sendable, Equatable, Identifiable {
  public let name: String
  public var kind: String
  public var value: String
  public var id: String { name }

  public init(name: String, kind: String = "text", value: String = "") {
    self.name = name
    self.kind = kind
    self.value = value
  }
}

public struct WorkbenchTableOperationReview: Sendable, Equatable {
  public let tokenId: Data
  public let target: String
  public let preview: String
  public let destructive: Bool
  public let confirmation: String
  public let expiresAtMs: UInt64

  public init(
    tokenId: Data, target: String, preview: String, destructive: Bool,
    confirmation: String, expiresAtMs: UInt64
  ) {
    self.tokenId = tokenId
    self.target = target
    self.preview = preview
    self.destructive = destructive
    self.confirmation = confirmation
    self.expiresAtMs = expiresAtMs
  }
}

// Immutable application facts crossing the presentation/backend seam. These
// deliberately know nothing about generated UniFFI records.
public struct WorkbenchBrowseSort: Sendable, Equatable, Identifiable {
  public let column: String
  public let descending: Bool
  public var id: String { column }
  public init(column: String, descending: Bool = false) {
    self.column = column
    self.descending = descending
  }
}

public func workbenchColumnHeaderTitle(
  column: String,
  sorts: [WorkbenchBrowseSort]
) -> String {
  guard let index = sorts.firstIndex(where: { $0.column == column }) else { return column }
  let direction = sorts[index].descending ? "↓" : "↑"
  return "\(column) \(direction) \(index + 1)"
}

public struct WorkbenchBrowseFilter: Sendable, Equatable, Identifiable {
  public let id: UUID
  public let column: String
  public let operatorName: String
  public let value: String?
  public init(id: UUID = UUID(), column: String, operatorName: String, value: String?) {
    self.id = id
    self.column = column
    self.operatorName = operatorName
    self.value = value
  }
}

public struct WorkbenchSavedFilterPreset: Sendable, Equatable, Identifiable {
  public let name: String
  public let filters: [WorkbenchBrowseFilter]
  public let rawWhere: String?
  public var id: String { name }
  public init(name: String, filters: [WorkbenchBrowseFilter], rawWhere: String?) {
    self.name = name
    self.filters = filters
    self.rawWhere = rawWhere
  }
}

public struct WorkbenchOperation: Sendable, Equatable {
  public let table: WorkbenchTable?
  public let envelope: WorkbenchPageEnvelope?
  public let outcome: String?
  public let historyFailed: Bool
  public init(
    table: WorkbenchTable?, envelope: WorkbenchPageEnvelope?, outcome: String?, historyFailed: Bool
  ) {
    self.table = table
    self.envelope = envelope
    self.outcome = outcome
    self.historyFailed = historyFailed
  }
}
public struct WorkbenchApplyOutcome: Sendable, Equatable {
  public let transaction: String
  public let changeCount: UInt32
  public let appliedCount: UInt32
  public let conflictCount: UInt32
  public let failedCount: UInt32
  public init(
    transaction: String, changeCount: UInt32, appliedCount: UInt32, conflictCount: UInt32,
    failedCount: UInt32
  ) {
    self.transaction = transaction
    self.changeCount = changeCount
    self.appliedCount = appliedCount
    self.conflictCount = conflictCount
    self.failedCount = failedCount
  }
}
public struct WorkbenchCancelOutcome: Sendable, Equatable {
  public let core: String
  public let runtime: String?
  public init(core: String, runtime: String?) {
    self.core = core
    self.runtime = runtime
  }
}
public struct WorkbenchOpenParams: Sendable, Equatable {
  public let engine: String
  public let host: String
  public let port: UInt16
  public let database: String
  public let user: String
  public let password: String
  public let tlsMode: String
  public init(
    engine: String, host: String, port: UInt16, database: String, user: String, password: String,
    tlsMode: String
  ) {
    self.engine = engine
    self.host = host
    self.port = port
    self.database = database
    self.user = user
    self.password = password
    self.tlsMode = tlsMode
  }
}

public struct WorkbenchProfileItem: Sendable, Equatable, Hashable {
  public let idBytes: Data
  public let revision: UInt64
  public let name: String
  public let engine: String
  public let group: String?
  public let favorite: Bool
  public let savedOrder: UInt32
  public let host: String?
  public let port: String?
  public let context: String?
  public let safetyMode: String
  public let environment: String?
  public let productionWarning: Bool
  public let dangerousPlaintext: Bool
  public let connected: Bool
  public init(
    idBytes: Data, revision: UInt64, name: String, engine: String, group: String?, favorite: Bool,
    savedOrder: UInt32, host: String?, port: String?, context: String?, safetyMode: String,
    environment: String?, productionWarning: Bool, dangerousPlaintext: Bool, connected: Bool
  ) {
    self.idBytes = idBytes
    self.revision = revision
    self.name = name
    self.engine = engine
    self.group = group
    self.favorite = favorite
    self.savedOrder = savedOrder
    self.host = host
    self.port = port
    self.context = context
    self.safetyMode = safetyMode
    self.environment = environment
    self.productionWarning = productionWarning
    self.dangerousPlaintext = dangerousPlaintext
    self.connected = connected
  }
}
public struct WorkbenchProfileDraft: Sendable, Equatable {
  public let idBytes: Data?
  public let revision: UInt64
  public let engine: String
  public let name: String
  public let group: String
  public let environment: String
  public let host: String
  public let port: String
  public let database: String
  public let username: String
  public let passwordSource: String
  public let passwordValue: String
  public let passwordReference: Data?
  public let hasStoredPassword: Bool
  public let plaintextAcknowledged: Bool
  public let tlsMode: String
  public let safetyMode: String
  public init(
    idBytes: Data?, revision: UInt64, engine: String, name: String, group: String,
    environment: String, host: String, port: String, database: String, username: String,
    passwordSource: String, passwordValue: String, passwordReference: Data?,
    hasStoredPassword: Bool, plaintextAcknowledged: Bool, tlsMode: String, safetyMode: String
  ) {
    self.idBytes = idBytes
    self.revision = revision
    self.engine = engine
    self.name = name
    self.group = group
    self.environment = environment
    self.host = host
    self.port = port
    self.database = database
    self.username = username
    self.passwordSource = passwordSource
    self.passwordValue = passwordValue
    self.passwordReference = passwordReference
    self.hasStoredPassword = hasStoredPassword
    self.plaintextAcknowledged = plaintextAcknowledged
    self.tlsMode = tlsMode
    self.safetyMode = safetyMode
  }
}
public struct WorkbenchProfileGroup: Sendable, Equatable {
  public let name: String
  public let alphabetical: Bool
  public init(name: String, alphabetical: Bool) {
    self.name = name
    self.alphabetical = alphabetical
  }
}
public struct WorkbenchConnectionTestReport: Sendable, Equatable {
  public let identity: String
  public let tlsOutcome: String
  public let elapsedMillis: UInt64
  public init(identity: String, tlsOutcome: String, elapsedMillis: UInt64) {
    self.identity = identity
    self.tlsOutcome = tlsOutcome
    self.elapsedMillis = elapsedMillis
  }
}
public struct WorkbenchCatalogNode: Sendable, Equatable, Hashable {
  public let idBytes: Data
  public let parentIdBytes: Data?
  public let depth: UInt16
  public let name: String
  public let kind: String
  public let childrenState: String
  public let expandable: Bool
  public init(
    idBytes: Data, parentIdBytes: Data?, depth: UInt16, name: String, kind: String,
    childrenState: String, expandable: Bool
  ) {
    self.idBytes = idBytes
    self.parentIdBytes = parentIdBytes
    self.depth = depth
    self.name = name
    self.kind = kind
    self.childrenState = childrenState
    self.expandable = expandable
  }
}

public struct WorkbenchHistoryItem: Sendable, Equatable {
  public let historyId: Int64
  public let engine: String
  public let databaseName: String
  public let schemaName: String?
  public let statementText: String?
  public let outcome: String
  public let createdAt: String
  public init(
    historyId: Int64, engine: String, databaseName: String, schemaName: String?,
    statementText: String?, outcome: String, createdAt: String
  ) {
    self.historyId = historyId
    self.engine = engine
    self.databaseName = databaseName
    self.schemaName = schemaName
    self.statementText = statementText
    self.outcome = outcome
    self.createdAt = createdAt
  }
}
public struct WorkbenchSavedQueryItem: Sendable, Equatable {
  public let queryId: Int64
  public let name: String
  public let engine: String
  public let statementText: String
  public let updatedAt: String
  public init(
    queryId: Int64, name: String, engine: String, statementText: String, updatedAt: String
  ) {
    self.queryId = queryId
    self.name = name
    self.engine = engine
    self.statementText = statementText
    self.updatedAt = updatedAt
  }
}
public struct WorkbenchSQLFile: Sendable, Equatable {
  public let path: String
  public let statementText: String
  public let modifiedNanos: UInt64?
  public let len: UInt64
  public init(path: String, statementText: String, modifiedNanos: UInt64?, len: UInt64) {
    self.path = path
    self.statementText = statementText
    self.modifiedNanos = modifiedNanos
    self.len = len
  }
}
public struct WorkbenchWorkspaceTab: Sendable, Equatable {
  public let title: String
  public let statementText: String
  public init(title: String, statementText: String) {
    self.title = title
    self.statementText = statementText
  }
}
public struct WorkbenchSessionIntent: Sendable, Equatable {
  public let database: String
  public let schema: String?
  public let selectedTab: UInt32
  public let tabs: [WorkbenchWorkspaceTab]
  public init(database: String, schema: String?, selectedTab: UInt32, tabs: [WorkbenchWorkspaceTab])
  {
    self.database = database
    self.schema = schema
    self.selectedTab = selectedTab
    self.tabs = tabs
  }
}
public struct WorkbenchNativeWindowIntent: Sendable, Equatable {
  public let profileId: Data
  public let intent: WorkbenchSessionIntent
  public init(profileId: Data, intent: WorkbenchSessionIntent) {
    self.profileId = profileId
    self.intent = intent
  }
}
public struct WorkbenchSessionHealth: Sendable, Equatable {
  public let state: String
  public let serverReachable: Bool
  public let elapsedMillis: UInt64?
  public let authenticationStopped: Bool
  public init(
    state: String, serverReachable: Bool, elapsedMillis: UInt64?, authenticationStopped: Bool
  ) {
    self.state = state
    self.serverReachable = serverReachable
    self.elapsedMillis = elapsedMillis
    self.authenticationStopped = authenticationStopped
  }
}
public struct WorkbenchReconnectPlan: Sendable, Equatable {
  public let action: String
  public let delayMillis: UInt64?
  public let restoreLastContext: Bool
  public init(action: String, delayMillis: UInt64?, restoreLastContext: Bool) {
    self.action = action
    self.delayMillis = delayMillis
    self.restoreLastContext = restoreLastContext
  }
}
public struct WorkbenchReconnectAttempt: Sendable, Equatable {
  public let state: String
  public let sessionId: Data?
  public init(state: String, sessionId: Data?) {
    self.state = state
    self.sessionId = sessionId
  }
}

public struct WorkbenchCSVRow: Sendable, Equatable {
  public let cells: [String]
  public init(cells: [String]) { self.cells = cells }
}
public struct WorkbenchCSVImportPreview: Sendable, Equatable {
  public let path: String
  public let headers: [String]
  public let rows: [WorkbenchCSVRow]
  public let totalRows: UInt32
  public let formulaLikeCells: UInt32
  public let fingerprint: String
  public init(
    path: String, headers: [String], rows: [WorkbenchCSVRow], totalRows: UInt32,
    formulaLikeCells: UInt32, fingerprint: String
  ) {
    self.path = path
    self.headers = headers
    self.rows = rows
    self.totalRows = totalRows
    self.formulaLikeCells = formulaLikeCells
    self.fingerprint = fingerprint
  }
}
public struct WorkbenchCSVImportReview: Sendable, Equatable {
  public let tokenId: Data
  public let target: String
  public let rowCount: UInt32
  public let columnCount: UInt32
  public let formulaLikeCells: UInt32
  public let expiresAtMs: UInt64
  public init(
    tokenId: Data, target: String, rowCount: UInt32, columnCount: UInt32, formulaLikeCells: UInt32,
    expiresAtMs: UInt64
  ) {
    self.tokenId = tokenId
    self.target = target
    self.rowCount = rowCount
    self.columnCount = columnCount
    self.formulaLikeCells = formulaLikeCells
    self.expiresAtMs = expiresAtMs
  }
}
public struct WorkbenchCSVImportProgress: Sendable, Equatable {
  public let operationId: Data
  public let phase: String
  public let completedRows: UInt64
  public let totalRows: UInt64
  public let appliedRows: UInt64
  public let conflictRows: UInt64
  public let failedRows: UInt64
  public let errors: [String]
  public let errorsTruncated: Bool
  public let summary: String
  public init(
    operationId: Data, phase: String, completedRows: UInt64, totalRows: UInt64,
    appliedRows: UInt64, conflictRows: UInt64, failedRows: UInt64, errors: [String],
    errorsTruncated: Bool, summary: String
  ) {
    self.operationId = operationId
    self.phase = phase
    self.completedRows = completedRows
    self.totalRows = totalRows
    self.appliedRows = appliedRows
    self.conflictRows = conflictRows
    self.failedRows = failedRows
    self.errors = errors
    self.errorsTruncated = errorsTruncated
    self.summary = summary
  }
}
public struct WorkbenchStreamExportProgress: Sendable, Equatable {
  public let operationId: Data
  public let phase: String
  public let completedRows: UInt64
  public let bytesWritten: UInt64
  public let destination: String
  public let summary: String
  public init(
    operationId: Data, phase: String, completedRows: UInt64, bytesWritten: UInt64,
    destination: String, summary: String
  ) {
    self.operationId = operationId
    self.phase = phase
    self.completedRows = completedRows
    self.bytesWritten = bytesWritten
    self.destination = destination
    self.summary = summary
  }
}
public struct WorkbenchRedisKeyView: Sendable, Equatable {
  public let kind: String
  public let lines: [String]
  public let nextSkip: UInt64?
  public init(kind: String, lines: [String], nextSkip: UInt64?) {
    self.kind = kind
    self.lines = lines
    self.nextSkip = nextSkip
  }
}
public struct WorkbenchRedisOverview: Sendable, Equatable {
  public let sampledAtMs: UInt64
  public let lines: [String]
  public init(sampledAtMs: UInt64, lines: [String]) {
    self.sampledAtMs = sampledAtMs
    self.lines = lines
  }
}
public struct WorkbenchRedisSubscriptionStatus: Sendable, Equatable {
  public let operationId: Data
  public let selector: String
  public let pattern: Bool
  public let phase: String
  public let messages: [String]
  public let totalReceived: UInt64
  public let discontinuities: UInt64
  public let summary: String
  public init(
    operationId: Data, selector: String, pattern: Bool, phase: String, messages: [String],
    totalReceived: UInt64, discontinuities: UInt64, summary: String
  ) {
    self.operationId = operationId
    self.selector = selector
    self.pattern = pattern
    self.phase = phase
    self.messages = messages
    self.totalReceived = totalReceived
    self.discontinuities = discontinuities
    self.summary = summary
  }
}
public struct WorkbenchDdlChangeReview: Sendable, Equatable {
  public let tokenId: Data
  public let preview: String
  public let destructive: Bool
  public let rollbackSummary: String
  public let expiresAtMs: UInt64
  public init(
    tokenId: Data, preview: String, destructive: Bool, rollbackSummary: String,
    expiresAtMs: UInt64
  ) {
    self.tokenId = tokenId
    self.preview = preview
    self.destructive = destructive
    self.rollbackSummary = rollbackSummary
    self.expiresAtMs = expiresAtMs
  }
}

public struct WorkbenchPostgresActivityRow: Sendable, Equatable, Identifiable {
  public let pid: Int32
  public let user: String
  public let application: String
  public let state: String
  public let queryPreview: String
  public var id: Int32 { pid }
  public init(pid: Int32, user: String, application: String, state: String, queryPreview: String) {
    self.pid = pid
    self.user = user
    self.application = application
    self.state = state
    self.queryPreview = queryPreview
  }
}

public struct WorkbenchRelationshipEdge: Sendable, Equatable, Identifiable {
  public let fromSchema: String
  public let fromTable: String
  public let fromColumn: String
  public let toSchema: String
  public let toTable: String
  public let toColumn: String
  public var id: String {
    "\(fromSchema).\(fromTable).\(fromColumn)->\(toSchema).\(toTable).\(toColumn)"
  }
  public init(
    fromSchema: String, fromTable: String, fromColumn: String,
    toSchema: String, toTable: String, toColumn: String
  ) {
    self.fromSchema = fromSchema
    self.fromTable = fromTable
    self.fromColumn = fromColumn
    self.toSchema = toSchema
    self.toTable = toTable
    self.toColumn = toColumn
  }
}

public struct WorkbenchRelationshipSnapshot: Sendable, Equatable {
  public let namespace: String
  public let relation: String
  public let edges: [WorkbenchRelationshipEdge]
  public let truncated: Bool
  public init(
    namespace: String, relation: String, edges: [WorkbenchRelationshipEdge], truncated: Bool
  ) {
    self.namespace = namespace
    self.relation = relation
    self.edges = edges
    self.truncated = truncated
  }
}

public struct WorkbenchRoleMembership: Sendable, Equatable, Identifiable {
  public let role: String
  public let member: String
  public let inheritOption: Bool
  public let adminOption: Bool
  public let setOption: Bool
  public var id: String { "\(role)<-\(member)" }
  public init(
    role: String, member: String, inheritOption: Bool, adminOption: Bool, setOption: Bool
  ) {
    self.role = role
    self.member = member
    self.inheritOption = inheritOption
    self.adminOption = adminOption
    self.setOption = setOption
  }
}

public struct WorkbenchRolePrivilege: Sendable, Equatable, Identifiable {
  public let grantee: String
  public let privilege: String
  public let object: String
  public let grantable: Bool
  public var id: String { "\(grantee):\(privilege):\(object)" }
  public init(grantee: String, privilege: String, object: String, grantable: Bool) {
    self.grantee = grantee
    self.privilege = privilege
    self.object = object
    self.grantable = grantable
  }
}

public struct WorkbenchRoleSnapshot: Sendable, Equatable {
  public let currentUser: String
  public let roles: [String]
  public let memberships: [WorkbenchRoleMembership]
  public let effectiveRoles: [String]
  public let cycleEdges: [String]
  public let privileges: [WorkbenchRolePrivilege]
  public let privilegeScope: String?
  public let privilegesUnavailable: Bool
  public let truncated: Bool
  public init(
    currentUser: String, roles: [String], memberships: [WorkbenchRoleMembership],
    effectiveRoles: [String], cycleEdges: [String], privileges: [WorkbenchRolePrivilege],
    privilegeScope: String?, privilegesUnavailable: Bool, truncated: Bool
  ) {
    self.currentUser = currentUser
    self.roles = roles
    self.memberships = memberships
    self.effectiveRoles = effectiveRoles
    self.cycleEdges = cycleEdges
    self.privileges = privileges
    self.privilegeScope = privilegeScope
    self.privilegesUnavailable = privilegesUnavailable
    self.truncated = truncated
  }
}

public struct WorkbenchRoleChangeReview: Sendable, Equatable {
  public let tokenId: Data
  public let summary: String
  public let expiresAtMs: UInt64
  public init(tokenId: Data, summary: String, expiresAtMs: UInt64) {
    self.tokenId = tokenId
    self.summary = summary
    self.expiresAtMs = expiresAtMs
  }
}

public struct WorkbenchBackendSignalOutcome: Sendable, Equatable {
  public let kind: String
  public let pid: Int32
  public let acknowledged: Bool
  public init(kind: String, pid: Int32, acknowledged: Bool) {
    self.kind = kind
    self.pid = pid
    self.acknowledged = acknowledged
  }
}

public struct WorkbenchPostgresToolProbe: Sendable, Equatable {
  public let kind: String
  public let available: Bool
  public let path: String?
  public let version: String?
  public let summary: String
  public init(kind: String, available: Bool, path: String?, version: String?, summary: String) {
    self.kind = kind
    self.available = available
    self.path = path
    self.version = version
    self.summary = summary
  }
}

public struct WorkbenchPostgresToolStatus: Sendable, Equatable {
  public let operationId: Data
  public let kind: String
  public let phase: String
  public let summary: String
  public init(operationId: Data, kind: String, phase: String, summary: String) {
    self.operationId = operationId
    self.kind = kind
    self.phase = phase
    self.summary = summary
  }
}

public struct WorkbenchRelationColumn: Sendable, Equatable {
  public let name: String
  public let dataType: String
  public let nullable: Bool
  public let defaultExpression: String?
  public let comment: String?
  public let primaryKey: Bool
  public let sortingKey: Bool
  public init(
    name: String, dataType: String, nullable: Bool, defaultExpression: String?, comment: String?,
    primaryKey: Bool, sortingKey: Bool
  ) {
    self.name = name
    self.dataType = dataType
    self.nullable = nullable
    self.defaultExpression = defaultExpression
    self.comment = comment
    self.primaryKey = primaryKey
    self.sortingKey = sortingKey
  }
}
public struct WorkbenchRelationIndex: Sendable, Equatable {
  public let kind: String
  public let name: String
  public let definition: String
  public init(kind: String, name: String, definition: String) {
    self.kind = kind
    self.name = name
    self.definition = definition
  }
}
public struct WorkbenchRelationConstraint: Sendable, Equatable {
  public let kind: String
  public let name: String
  public let definition: String
  public init(kind: String, name: String, definition: String) {
    self.kind = kind
    self.name = name
    self.definition = definition
  }
}
public struct WorkbenchRelationFact: Sendable, Equatable {
  public let name: String
  public let value: String
  public init(name: String, value: String) {
    self.name = name
    self.value = value
  }
}
public struct WorkbenchRelationStructure: Sendable, Equatable {
  public let engine: String
  public let namespace: String
  public let relation: String
  public let columns: [WorkbenchRelationColumn]
  public let indexes: [WorkbenchRelationIndex]
  public let constraints: [WorkbenchRelationConstraint]
  public let facts: [WorkbenchRelationFact]
  public let ddl: String
  public init(
    engine: String, namespace: String, relation: String, columns: [WorkbenchRelationColumn],
    indexes: [WorkbenchRelationIndex], constraints: [WorkbenchRelationConstraint],
    facts: [WorkbenchRelationFact], ddl: String
  ) {
    self.engine = engine
    self.namespace = namespace
    self.relation = relation
    self.columns = columns
    self.indexes = indexes
    self.constraints = constraints
    self.facts = facts
    self.ddl = ddl
  }
}

public struct WorkbenchPageEnvelope: Sendable, Equatable {
  public let encodingVersion: UInt16
  public let resultId: Data
  public let revision: UInt64
  public let engine: UInt8
  public let startRow: UInt64
  public let rowCount: UInt32
  public let columnCount: UInt32
  public let arenaByteLen: UInt64
  public let columnTextByteLen: UInt64
  public let delivery: UInt8
  public let warnings: UInt16
  public init(
    encodingVersion: UInt16, resultId: Data, revision: UInt64, engine: UInt8, startRow: UInt64,
    rowCount: UInt32, columnCount: UInt32, arenaByteLen: UInt64, columnTextByteLen: UInt64,
    delivery: UInt8, warnings: UInt16
  ) {
    self.encodingVersion = encodingVersion
    self.resultId = resultId
    self.revision = revision
    self.engine = engine
    self.startRow = startRow
    self.rowCount = rowCount
    self.columnCount = columnCount
    self.arenaByteLen = arenaByteLen
    self.columnTextByteLen = columnTextByteLen
    self.delivery = delivery
    self.warnings = warnings
  }
}
public struct WorkbenchColumn: Sendable, Equatable {
  public let name: String
  public let engine: UInt8
  public let engineType: String
  public let nullable: Bool
  public init(name: String, engine: UInt8, engineType: String, nullable: Bool) {
    self.name = name
    self.engine = engine
    self.engineType = engineType
    self.nullable = nullable
  }
}
public struct WorkbenchCell: Sendable, Equatable {
  public let display: String
  public let kind: UInt8
  public let truncation: UInt8
  public let originalByteCount: UInt64?
  public let bytes: Data
  public init(
    display: String, kind: UInt8, truncation: UInt8, originalByteCount: UInt64?, bytes: Data
  ) {
    self.display = display
    self.kind = kind
    self.truncation = truncation
    self.originalByteCount = originalByteCount
    self.bytes = bytes
  }
  public var kindLabel: String {
    switch kind {
    case 0: "NULL"
    case 1: "Boolean"
    case 2: "Signed integer"
    case 3: "Unsigned integer"
    case 4: "Float"
    case 5: "Decimal"
    case 6: "Temporal"
    case 7: "Text"
    case 8: "Structured"
    case 9: "Binary"
    case 10: "Invalid"
    case 11: "Unknown"
    default: "Kind \(kind)"
    }
  }
  public var isTruncated: Bool { truncation != 0 }
}
public struct WorkbenchTable: Sendable, Equatable {
  public let columns: [String]
  public let rows: [[String]]
  public let columnMetadata: [WorkbenchColumn]
  public let cells: [[WorkbenchCell]]
  public init(
    columns: [String], rows: [[String]], columnMetadata: [WorkbenchColumn]? = nil,
    cells: [[WorkbenchCell]]? = nil
  ) {
    self.columns = columns
    self.rows = rows
    self.columnMetadata =
      columnMetadata
      ?? columns.map { WorkbenchColumn(name: $0, engine: 0, engineType: "unknown", nullable: true) }
    self.cells =
      cells
      ?? rows.map { row in
        row.map {
          WorkbenchCell(
            display: $0, kind: 7, truncation: 0, originalByteCount: nil, bytes: Data($0.utf8))
        }
      }
  }
  public func appending(_ page: WorkbenchTable) -> WorkbenchTable? {
    guard columns == page.columns, columnMetadata == page.columnMetadata else { return nil }
    return WorkbenchTable(
      columns: columns, rows: rows + page.rows, columnMetadata: columnMetadata,
      cells: cells + page.cells)
  }
}
