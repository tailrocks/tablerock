import SwiftUI

@MainActor
struct WorkbenchActions {
  let model: WorkbenchPresentationStore

  var canRun: Bool {
    model.queryWorkbenchSelected && model.sessionHex != nil
      && !model.isRunning && !model.isCatalogRefreshing
  }
  var canCancel: Bool { model.isRunning }
  var canRefresh: Bool {
    model.sessionHex != nil && !model.isRunning && !model.isCatalogRefreshing
  }
  var canShowActivity: Bool {
    model.sessionHex != nil && model.connectedEngine == "postgresql"
  }
  var canShowPostgresTools: Bool { canShowActivity }
  var canShowRelationships: Bool {
    canShowActivity && model.selectedObjectTab != nil
  }
  var canShowRoles: Bool { canShowActivity }
  var canShowRedisSubscription: Bool {
    model.sessionHex != nil && model.connectedEngine == "redis"
  }
  var canShowFindReplace: Bool { model.queryWorkbenchSelected }

  func run() { Task { await model.runQuery() } }
  func cancel() { Task { await model.cancel() } }
  func refresh() { Task { await model.browse() } }
  func quickSwitch() { Task { await model.showQuickSwitcher() } }
  func explain() { Task { await model.runExplain() } }
  func showActivity() { Task { await model.showPostgresActivity() } }
  func showPostgresTools() { Task { await model.showPostgresTools() } }
  func showRelationships() { Task { await model.showPostgresRelationships() } }
  func showRoles() { Task { await model.showPostgresRoles() } }
  func showRedisSubscription() { model.showRedisSubscription() }
  func showFindReplace() { model.showFindReplace() }
}

private struct WorkbenchActionsKey: FocusedValueKey {
  typealias Value = WorkbenchActions
}

extension FocusedValues {
  var workbenchActions: WorkbenchActions? {
    get { self[WorkbenchActionsKey.self] }
    set { self[WorkbenchActionsKey.self] = newValue }
  }
}

struct WorkbenchCommands: Commands {
  @FocusedValue(\.workbenchActions) private var actions

  var body: some Commands {
    CommandMenu("Query") {
      Button("Run Query") { actions?.run() }
        .keyboardShortcut(.return, modifiers: .command)
        .disabled(actions?.canRun != true)
      Button("Cancel Query") { actions?.cancel() }
        .keyboardShortcut(".", modifiers: .command)
        .disabled(actions?.canCancel != true)
      Divider()
      Button("Refresh Catalog") { actions?.refresh() }
        .keyboardShortcut("r", modifiers: [.command, .shift])
        .disabled(actions?.canRefresh != true)
      Divider()
      Button("Quick Switcher…") { actions?.quickSwitch() }
        .keyboardShortcut("o", modifiers: [.command, .shift])
      Button("Explain Query") { actions?.explain() }
        .keyboardShortcut("e", modifiers: [.command, .shift])
        .disabled(actions?.canRun != true)
      Button("Find and Replace…") { actions?.showFindReplace() }
        .keyboardShortcut("f", modifiers: [.command, .option])
        .disabled(actions?.canShowFindReplace != true)
      Button("PostgreSQL Activity…") { actions?.showActivity() }
        .disabled(actions?.canShowActivity != true)
      Button("PostgreSQL Backup and Restore…") { actions?.showPostgresTools() }
        .disabled(actions?.canShowPostgresTools != true)
      Button("Relation Lens…") { actions?.showRelationships() }
        .disabled(actions?.canShowRelationships != true)
      Button("PostgreSQL Roles and Privileges…") { actions?.showRoles() }
        .disabled(actions?.canShowRoles != true)
      Button("Redis Pub/Sub…") { actions?.showRedisSubscription() }
        .disabled(actions?.canShowRedisSubscription != true)
    }
  }
}
