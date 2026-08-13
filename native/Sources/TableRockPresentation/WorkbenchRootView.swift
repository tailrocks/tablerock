import SwiftUI

public struct ContentView: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @State private var columnVisibility: NavigationSplitViewVisibility = .all
  @State private var availableWidth: CGFloat = .infinity
  @State private var automaticallyCollapsedSidebar = false

  public init() {}

  public var body: some View {
    @Bindable var model = model
    // The disconnected toolbar is installed before profile loading completes.
    // Observe readiness here so SwiftUI re-attaches customizable toolbar
    // content even when the selected connection route remains unchanged.
    let _ = model.profiles.count
    let _ = model.profilesLoading
    let trailingInspectorVisible = hasTrailingInspector
    workbenchSplitView(trailingInspectorVisible: trailingInspectorVisible)
    .sheet(
      isPresented: Binding(
        get: { model.profileEditorSheetPresented },
        set: {
          model.profileEditorSheetPresented = $0
          if !$0 { model.editorDraft = nil }
        }
      )
    ) {
      if let draft = model.editorDraft {
        ProfileEditorSheet(
          initialDraft: draft,
          onTest: { tested in await model.testProfileDraft(tested) },
          onSave: { saved in await model.saveProfile(saved) }
        )
      }
    }
    .sheet(isPresented: $model.directConnectionPresented) {
      TemporaryConnectionSheet()
    }
    .sheet(item: $model.groupDialog) { dialog in
      ProfileGroupEditorSheet(initialDialog: dialog) { saved in
        await model.saveGroup(saved)
      }
    }
    .sheet(item: $model.passwordPrompt) { prompt in
      ProfilePasswordSheet(profile: prompt.profile) { password in
        await model.submitPasswordPrompt(prompt, password: password)
      }
    }
    .sheet(item: $model.connectionUrlImport) { importState in
      ConnectionUrlImportSheet(initial: importState) { input in
        await model.parseConnectionUrl(input)
      }
    }
    .sheet(item: $model.externalUrlReview) { review in
      ExternalUrlConfirmationSheet(review: review)
    }
    .sheet(isPresented: $model.quickSwitcherPresented) {
      QuickSwitcherSheet()
    }
    .sheet(isPresented: $model.historyPresented) {
      HistorySheet()
    }
    .sheet(isPresented: $model.savedQueriesPresented) {
      SavedQueriesSheet()
    }
    .sheet(isPresented: $model.findReplacePresented) {
      FindReplaceSheet()
    }
    .sheet(
      isPresented: $model.queryParametersPresented,
      onDismiss: { model.cancelQueryParameters() }
    ) {
      QueryParametersSheet()
    }
    .sheet(isPresented: $model.redisOverviewPresented) {
      RedisOverviewSheet()
    }
    .sheet(
      isPresented: $model.redisSubscriptionPresented,
      onDismiss: { Task { await model.closeRedisSubscription() } }
    ) {
      RedisSubscriptionSheet()
    }
    .sheet(
      isPresented: $model.ddlChangePresented,
      onDismiss: { Task { await model.closeDdlChange() } }
    ) {
      DdlChangeSheet()
    }
    .sheet(
      isPresented: $model.mutationReviewPresented,
      onDismiss: { Task { await model.discardRowUpdate() } }
    ) {
      MutationWorkflowSheet()
    }
    .sheet(
      isPresented: $model.tableOperationPresented,
      onDismiss: { Task { await model.closeTableOperation() } }
    ) {
      TableOperationSheet()
    }
    .sheet(isPresented: $model.postgresActivityPresented) {
      PostgresActivitySheet()
    }
    .sheet(isPresented: $model.postgresRelationshipsPresented) {
      PostgresRelationshipsSheet()
    }
    .sheet(isPresented: $model.postgresRolesPresented) {
      PostgresRolesSheet()
    }
    .sheet(isPresented: $model.postgresToolsPresented) {
      PostgresToolsSheet()
    }
    .sheet(
      isPresented: $model.csvImportPresented,
      onDismiss: { Task { await model.closeCsvImport() } }
    ) {
      CsvImportSheet()
    }
    .sheet(isPresented: $model.streamExportPresented) {
      StreamExportSheet()
    }
    .alert("Save Query", isPresented: $model.saveQueryDialog) {
      TextField("Name", text: $model.savedQueryName)
      Button("Save") { Task { await model.saveCurrentQuery() } }
      Button("Cancel", role: .cancel) { model.saveQueryDialog = false }
    } message: {
      Text("Save current editor text for the active database engine.")
    }
    .confirmationDialog(
      "Remove connection?",
      isPresented: Binding(
        get: { model.pendingRemoval != nil },
        set: { if !$0 { model.pendingRemoval = nil } }
      ),
      presenting: model.pendingRemoval
    ) { _ in
      Button("Remove", role: .destructive) { Task { await model.removePendingProfile() } }
      Button("Cancel", role: .cancel) { model.pendingRemoval = nil }
    } message: { item in
      Text("\(item.name) will be removed. Active sessions remain open.")
    }
    .confirmationDialog(
      "Remove group?",
      isPresented: Binding(
        get: { model.pendingGroupRemoval != nil },
        set: { if !$0 { model.pendingGroupRemoval = nil } }
      ),
      presenting: model.pendingGroupRemoval
    ) { _ in
      Button("Remove Group", role: .destructive) {
        Task { await model.removePendingGroup() }
      }
      Button("Cancel", role: .cancel) { model.pendingGroupRemoval = nil }
    } message: { name in
      Text("Connections in \(name) move to Ungrouped. No connection is deleted.")
    }
    .confirmationDialog(
      "Discard unsaved editor changes?",
      isPresented: $model.confirmDiscardForOpen
    ) {
      Button("Discard and Open", role: .destructive) { Task { await model.openSqlFile() } }
      Button("Cancel", role: .cancel) { model.confirmDiscardForOpen = false }
    } message: {
      Text("Opening another SQL file replaces current editor text.")
    }
    .confirmationDialog(
      "Close query tab with unsaved changes?",
      isPresented: Binding(
        get: { model.pendingQueryTabClose != nil },
        set: { if !$0 { model.pendingQueryTabClose = nil } }
      ),
      presenting: model.pendingQueryTabClose
    ) { _ in
      Button("Discard and Close", role: .destructive) { model.closePendingQueryTab() }
        .accessibilityIdentifier("query.tab.discard-close")
      Button("Cancel", role: .cancel) { model.pendingQueryTabClose = nil }
    } message: { tab in
      Text("Unsaved editor text in \(tab.title) will be discarded.")
    }
    .confirmationDialog(
      "SQL file changed outside TableRock",
      isPresented: $model.confirmExternalOverwrite
    ) {
      Button("Reload External Changes") { Task { await model.reloadSqlFile() } }
      Button("Overwrite External Changes", role: .destructive) {
        Task { await model.saveSqlFile(overwriteExternalChange: true) }
      }
      Button("Cancel", role: .cancel) { model.confirmExternalOverwrite = false }
    } message: {
      Text("Reload discards editor changes. Overwrite replaces external changes atomically.")
    }
    .alert(
      "Connection action failed",
      isPresented: Binding(
        get: { model.profileActionError != nil },
        set: { if !$0 { model.profileActionError = nil } }
      )
    ) {
      Button("OK") { model.profileActionError = nil }
    } message: {
      Text(model.profileActionError ?? "Unknown failure")
    }
    .alert(
      "Rename Query Tab",
      isPresented: Binding(
        get: { model.queryTabRename != nil },
        set: { if !$0 { model.queryTabRename = nil } }
      )
    ) {
      TextField("Title", text: $model.queryTabRenameText)
      Button("Rename") { model.renameQueryTab() }
      Button("Cancel", role: .cancel) { model.queryTabRename = nil }
    }
    .task { await model.initialize() }
    .focusedSceneValue(
      \.workbenchActions,
      focusedWorkbenchActions
    )
    .toolbar(id: "workbench") {
      WorkbenchToolbar(model: model)
    }
  }

  private func workbenchSplitView(trailingInspectorVisible: Bool) -> some View {
    NavigationSplitView(columnVisibility: $columnVisibility) {
      // Native Workbench uses one stable leading plane. Connections own it
      // before connect; the database catalog owns it for the live session.
      Group {
        if model.sessionHex != nil {
          ConnectionsCatalogPane()
        } else {
          ConnectionsNavigatorPane()
        }
      }
      .navigationTitle(model.sessionHex == nil ? "TableRock" : "Catalog")
      .navigationSplitViewColumnWidth(min: 210, ideal: 232, max: 300)
    } detail: {
      // Workbench shell when connected; welcome/direct-connect when not.
      // Spec: context strip · tabs · content · status (workbench.md).
      Group {
        if model.sessionHex != nil {
          WorkbenchShellView()
        } else {
          ConnectionWorkspaceView()
        }
      }
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
    .navigationSplitViewStyle(.balanced)
    .onGeometryChange(for: CGFloat.self) { proxy in
      proxy.size.width
    } action: { width in
      availableWidth = width
      updateResponsiveSidebar(
        width: width,
        trailingInspectorVisible: trailingInspectorVisible
      )
    }
    .onChange(of: trailingInspectorVisible) { _, visible in
      updateResponsiveSidebar(width: availableWidth, trailingInspectorVisible: visible)
    }
  }

  private var focusedWorkbenchActions: WorkbenchActions {
    // Focused scene values carry a reference. Explicit reads make Observation
    // invalidate this value when command capabilities change.
    _ = model.sessionHex
    _ = model.connectedEngine
    _ = model.queryWorkbenchSelected
    _ = model.isRunning
    _ = model.isCatalogRefreshing
    _ = model.selectedObjectTabId
    return WorkbenchActions(model: model)
  }

  private var hasTrailingInspector: Bool {
    guard model.sessionHex != nil else { return false }
    if let tab = model.selectedObjectTab, tab.selectedSection == "structure" {
      return true
    }
    return model.selectedCellSnapshot != nil
  }

  private func updateResponsiveSidebar(width: CGFloat, trailingInspectorVisible: Bool) {
    let shouldCollapse = WorkbenchResponsiveLayout.shouldCollapseSidebar(
      availableWidth: width,
      sessionActive: model.sessionHex != nil,
      trailingInspectorVisible: trailingInspectorVisible
    )
    if shouldCollapse {
      guard columnVisibility != .detailOnly else { return }
      automaticallyCollapsedSidebar = true
      columnVisibility = .detailOnly
    } else if automaticallyCollapsedSidebar {
      automaticallyCollapsedSidebar = false
      columnVisibility = .all
    }
  }
}

enum WorkbenchResponsiveLayout {
  static let navigationCollapseWidth: CGFloat = 1_200
  static let inspectorMinimumWidth: CGFloat = 240
  static let inspectorIdealWidth: CGFloat = 270

  static func shouldCollapseSidebar(
    availableWidth: CGFloat,
    sessionActive: Bool,
    trailingInspectorVisible: Bool
  ) -> Bool {
    sessionActive
      && trailingInspectorVisible
      && availableWidth < navigationCollapseWidth
  }
}
