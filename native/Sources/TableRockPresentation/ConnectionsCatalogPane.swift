import SwiftUI

/// Catalog half of the connections sidebar when a session is live.
struct ConnectionsCatalogPane: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(spacing: 0) {
      HStack {
        Text("CATALOG")
          .font(.caption.weight(.bold).monospaced())
        Spacer()
        Button {
          Task { await model.browse() }
        } label: {
          Image(systemName: "arrow.clockwise")
        }
        .buttonStyle(.borderless)
        .controlSize(.small)
        .disabled(model.isRunning || model.isCatalogRefreshing)
        .accessibilityLabel("Refresh catalog")
        .accessibilityIdentifier("catalog.refresh")
      }
      .padding(.horizontal, 10)
      .padding(.vertical, 6)
      if model.isCatalogRefreshing {
        ProgressView("Refreshing catalog…")
          .controlSize(.small)
          .padding(.horizontal, 10)
      }
      if let snapshot = model.catalogSnapshot {
        CatalogOutline(
          table: snapshot,
          selection: $model.catalogSelection,
          refreshState: model.catalogRefreshState,
          onExpand: { nodeKey in
            Task { await model.browse(expandedNodeKey: nodeKey) }
          },
          onOpen: { nodeKey in
            Task { await model.openCatalogObject(nodeKey: nodeKey) }
          }
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      } else {
        switch model.catalogRefreshState {
        case .loading:
          ProgressView("Loading catalog…")
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .failed(let message):
          ContentUnavailableView(
            "Catalog failed",
            systemImage: "exclamationmark.triangle",
            description: Text(message)
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
        default:
          ContentUnavailableView(
            "Catalog not loaded",
            systemImage: "sidebar.left",
            description: Text("Refresh to list database objects.")
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
      }
    }
  }
}
