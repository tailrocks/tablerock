import Foundation
import Observation
import TableRockFeature

enum CatalogRefreshState: Equatable {
  case idle
  case loading(nodeKey: String?)
  case loaded
  case stale(nodeKey: String?, message: String)
  case failed(message: String)
}

struct ProfileSection: Identifiable {
  let id: String
  let title: String
  let profiles: [WorkbenchProfileItem]
  let alphabetical: Bool
}

struct ProfileGroupDialog: Identifiable {
  let id: UUID
  let oldName: String?
  var name: String
  var title: String { oldName == nil ? "New Group" : "Rename Group" }
}

enum ProfilePasswordAction: String {
  case connect, test, reconnect
}

struct ProfilePasswordPrompt: Identifiable {
  let profile: WorkbenchProfileItem
  let action: ProfilePasswordAction
  var id: String { profile.idBytes.base64EncodedString() + ":" + action.rawValue }
}

struct ConnectionUrlImport: Identifiable {
  let id = UUID()
  var input = ""
  var error: String?
  var parsing = false
}

struct ExternalUrlReview: Identifiable {
  let id = UUID()
  let draft: ProfileEditorDraft
  let summary: String
  let matchedProfile: WorkbenchProfileItem?
}

enum QuickSwitcherTarget {
  case profile(Data)
  case queryTab(UUID)
  case objectTab(UUID)
  case catalog(String)
  case savedQuery(Int64)
}

struct QuickSwitcherItem: Identifiable {
  let id: String
  let title: String
  let subtitle: String
  let favorite: Bool
  let target: QuickSwitcherTarget
}

func catalogNodeKey(_ id: Data) -> String {
  "node:" + id.map { String(format: "%02x", $0) }.joined()
}

func catalogDescendantIds(
  of parentId: Data,
  in nodes: [WorkbenchCatalogNode]
) -> Set<Data> {
  var descendants: Set<Data> = []
  var frontier: Set<Data> = [parentId]
  while !frontier.isEmpty {
    let children = Set<Data>(
      nodes.compactMap { node in
        guard let parent = node.parentIdBytes, frontier.contains(parent) else { return nil }
        return node.idBytes
      })
    let fresh = children.subtracting(descendants)
    descendants.formUnion(fresh)
    frontier = fresh
  }
  return descendants
}

@MainActor
@Observable
final class NativeCellSelection {
  let row: Int
  let column: Int

  init(row: Int, column: Int) {
    self.row = row
    self.column = column
  }
}

@MainActor
@Observable
final class NativeQueryTab: Identifiable {
  let id: UUID
  var title: String
  var statementText: String
  var resultTable: WorkbenchTable?
  var resultIdData: Data?
  var resultRevision: UInt64 = 0
  var nextStartRow: UInt64?
  var writeOutcome: String?
  var isRunning = false
  var cancelOutcome: String?
  var reviewOutcome: String?
  var reviewError: String?
  var querySummary: String?
  var queryError: String?
  var activeOperationId: Data?
  var sqlFile: WorkbenchSQLFile?
  var sqlFileBaseline: String
  var sqlFileError: String?
  var selectedCell: NativeCellSelection?
  var copyOutcome: String?
  var copyError: String?
  var quickFilter = ""
  var explainPlan: String?
  var editorSelection = NSRange(location: 0, length: 0)
  var findScopeRange: NSRange?
  var lastFindMatch: NSRange?

  init(id: UUID, title: String, statementText: String) {
    self.id = id
    self.title = title
    self.statementText = statementText
    sqlFileBaseline = statementText
  }
}

@MainActor
@Observable
final class NativeObjectTab: Identifiable {
  let id: UUID
  let catalogNodeId: Data
  let kind: String
  var title: String
  var pinned: Bool
  var resultTable: WorkbenchTable?
  var resultIdData: Data?
  var resultRevision: UInt64 = 0
  var nextStartRow: UInt64?
  var isRunning = false
  var activeOperationId: Data?
  var summary: String?
  var error: String?
  var selectedCell: NativeCellSelection?
  var copyOutcome: String?
  var copyError: String?
  var quickFilter = ""
  var selectedSection = "data"
  var structure: WorkbenchRelationStructure?
  var structureLoading = false
  var structureError: String?
  var redisView: WorkbenchRedisKeyView?
  var sort: [WorkbenchBrowseSort] = []
  var filters: [WorkbenchBrowseFilter] = []
  var filterColumn = ""
  var filterOperator = "eq"
  var filterValue = ""
  var rawWhere: String?
  var rawWhereDraft = ""
  var filterPresets: [WorkbenchSavedFilterPreset] = []
  var filterPresetName = ""
  var filterPresetOutcome: String?
  var filterPresetError: String?

  init(id: UUID, node: WorkbenchCatalogNode, pinned: Bool = false) {
    self.id = id
    catalogNodeId = node.idBytes
    kind = node.kind
    title = node.name
    self.pinned = pinned
  }
}
