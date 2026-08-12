import SwiftUI

struct WorkbenchToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    WorkbenchConnectionToolbar(model: model)
    WorkbenchFileToolbar(model: model)
    WorkbenchQueryToolbar(model: model)
  }
}

struct WorkbenchFileToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    ToolbarItem(id: "open-sql-file", placement: .automatic) {
      Button {
        model.requestOpenSqlFile()
      } label: {
        Label("Open SQL File", systemImage: "folder")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarItem(id: "save-sql-file", placement: .automatic) {
      Button {
        Task { await model.saveSqlFile() }
      } label: {
        Label("Save SQL File", systemImage: "square.and.arrow.down")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarItem(id: "save-sql-file-as", placement: .automatic) {
      Button {
        Task { await model.saveSqlFile(saveAs: true) }
      } label: {
        Label("Save SQL File As", systemImage: "square.and.arrow.down.on.square")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarItem(id: "reload-sql-file", placement: .automatic) {
      Button {
        Task { await model.reloadSqlFile() }
      } label: {
        Label("Reload SQL File", systemImage: "arrow.clockwise")
      }
      .disabled(!model.queryWorkbenchSelected || model.sqlFile == nil)
    }
  }
}

struct WorkbenchConnectionToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    ToolbarItem(id: "connection", placement: .automatic) {
      Label(
        model.sessionHex == nil ? "Disconnected" : model.connectedEngine,
        systemImage: model.sessionHex == nil ? "bolt.slash" : "bolt.horizontal"
      )
      .accessibilityLabel(
        model.sessionHex == nil
          ? "No active connection" : "Connected to \(model.connectedEngine)")
    }
    ToolbarItem(id: "environment-safety", placement: .automatic) {
      EnvironmentSafetyBadge(model: model)
    }
    ToolbarItem(id: "disconnect", placement: .automatic) {
      Button {
        Task { await model.disconnectActive() }
      } label: {
        Label("Disconnect", systemImage: "bolt.slash")
      }
      .disabled(model.sessionHex == nil || model.isRunning)
    }
    ToolbarItem(id: "health", placement: .automatic) {
      Button {
        Task { await model.checkActiveHealth() }
      } label: {
        Label("Check Health", systemImage: "heart.text.square")
      }
      .disabled(model.sessionHex == nil || model.isRunning || model.healthChecking)
    }
    ToolbarItem(id: "reconnect", placement: .automatic) {
      Button {
        Task { await model.reconnectActive() }
      } label: {
        Label("Reconnect", systemImage: "arrow.triangle.2.circlepath")
      }
      .disabled(
        model.sessionHex == nil || model.isRunning
          || model.reconnectState?.hasPrefix("Reconnecting") == true
      )
    }
    ToolbarItem(id: "history", placement: .automatic) {
      Button {
        Task { await model.presentHistory() }
      } label: {
        Label("Query History", systemImage: "clock.arrow.circlepath")
      }
    }
    ToolbarItem(id: "saved-queries", placement: .automatic) {
      Button {
        Task { await model.presentSavedQueries() }
      } label: {
        Label("Saved Queries", systemImage: "bookmark")
      }
    }
  }
}

struct WorkbenchQueryToolbar: CustomizableToolbarContent {
  let model: WorkbenchPresentationStore

  var body: some CustomizableToolbarContent {
    ToolbarItem(id: "save-query", placement: .automatic) {
      Button {
        model.beginSaveCurrentQuery()
      } label: {
        Label("Save Query", systemImage: "bookmark.badge.plus")
      }
      .disabled(!model.queryWorkbenchSelected)
    }
    ToolbarSpacer(.fixed)
    ToolbarItem(id: "refresh", placement: .automatic) {
      Button {
        Task { await model.browse() }
      } label: {
        Label("Refresh Catalog", systemImage: "arrow.clockwise")
      }
      .disabled(model.sessionHex == nil || model.isRunning || model.isCatalogRefreshing)
    }
    ToolbarSpacer(.fixed)
    ToolbarItem(id: "run", placement: .primaryAction) {
      Button {
        Task { await model.runQuery() }
      } label: {
        Label("Run Query", systemImage: "play.fill")
      }
      .buttonStyle(.glassProminent)
      .disabled(
        !model.queryWorkbenchSelected || model.sessionHex == nil
          || model.isRunning || model.isCatalogRefreshing)
    }
    ToolbarItem(id: "cancel", placement: .primaryAction) {
      Button {
        Task { await model.cancel() }
      } label: {
        Label("Cancel Query", systemImage: "stop.fill")
      }
      .disabled(!model.isRunning)
    }
  }
}
