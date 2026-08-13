import SwiftUI
import TableRockFeature

/// Catalog half of the connections sidebar when a session is live.
struct ConnectionsCatalogPane: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @State private var searchText = ""

  private var visibleCatalog: [WorkbenchCatalogNode]? {
    guard let snapshot = model.catalogSnapshot else { return nil }
    let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !query.isEmpty else { return snapshot }

    let matches = snapshot.filter {
      $0.name.localizedCaseInsensitiveContains(query)
        || $0.kind.localizedCaseInsensitiveContains(query)
    }
    var visibleIds = Set(matches.map(\.idBytes))
    for match in matches {
      visibleIds.formUnion(catalogDescendantIds(of: match.idBytes, in: snapshot))
      var parent = match.parentIdBytes
      while let parentId = parent,
        let parentNode = snapshot.first(where: { $0.idBytes == parentId })
      {
        visibleIds.insert(parentId)
        parent = parentNode.parentIdBytes
      }
    }
    return snapshot.filter { visibleIds.contains($0.idBytes) }
  }

  var body: some View {
    @Bindable var model = model
    VStack(spacing: 0) {
      HStack(spacing: 7) {
        Image(systemName: "magnifyingglass")
          .foregroundStyle(.secondary)
        TextField("Search objects", text: $searchText)
          .textFieldStyle(.plain)
          .accessibilityIdentifier("catalog.search")
        Spacer()
        if !searchText.isEmpty {
          Button {
            searchText = ""
          } label: {
            Image(systemName: "xmark.circle.fill")
          }
          .buttonStyle(.plain)
          .foregroundStyle(.secondary)
          .accessibilityLabel("Clear catalog search")
        }
      }
      .font(.caption)
      .padding(.horizontal, 9)
      .frame(height: 30)
      .background(Color(nsColor: .controlBackgroundColor).opacity(0.72), in: .rect(cornerRadius: 7))
      .padding(10)

      HStack {
        Text(model.activeProfile?.context ?? model.activeProfile?.name ?? "Database")
          .font(.caption.weight(.semibold))
          .lineLimit(1)
        Spacer()
        Text(model.connectedEngine.uppercased())
          .font(.caption2.monospaced())
          .foregroundStyle(.secondary)
      }
      .padding(.horizontal, 12)
      .padding(.bottom, 6)

      if model.isCatalogRefreshing, model.catalogSnapshot != nil {
        ProgressView("Refreshing catalog…")
          .controlSize(.small)
          .padding(.horizontal, 10)
      }
      if let snapshot = visibleCatalog, !snapshot.isEmpty {
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
      } else if model.catalogSnapshot != nil, !searchText.isEmpty {
        ContentUnavailableView.search(text: searchText)
          .frame(maxWidth: .infinity, maxHeight: .infinity)
      } else {
        switch model.catalogRefreshState {
        case .loading:
          ProgressView("Loading catalog…")
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .accessibilityIdentifier("catalog.loading")
        case .failed(let message):
          ContentUnavailableView(
            "Catalog failed",
            systemImage: "exclamationmark.triangle",
            description: Text(message)
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .accessibilityIdentifier("catalog.error")
        default:
          ContentUnavailableView(
            "Catalog not loaded",
            systemImage: "sidebar.left",
            description: Text("Refresh to list database objects.")
          )
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .accessibilityIdentifier("catalog.empty")
        }
      }

      Divider()
      HStack(spacing: 7) {
        Image(systemName: connectionSymbol)
          .foregroundStyle(connectionColor)
        Text(connectionTitle)
          .accessibilityIdentifier("connection.status")
          .accessibilityValue(connectionTitle)
        Spacer()
        if let elapsed = model.sessionHealth?.elapsedMillis {
          Text("\(elapsed) ms")
            .monospacedDigit()
            .foregroundStyle(.secondary)
        }
        Button {
          Task { await model.browse() }
        } label: {
          Image(systemName: "arrow.clockwise")
        }
        .buttonStyle(.plain)
        .disabled(model.isRunning || model.isCatalogRefreshing)
        .accessibilityLabel("Refresh catalog")
        .accessibilityIdentifier("catalog.refresh")
      }
      .font(.caption2)
      .padding(.horizontal, 12)
      .frame(height: 32)
    }
    .accessibilityElement(children: .contain)
    .accessibilityLabel("Database catalog pane")
  }

  private var connectionTitle: String {
    guard let health = model.sessionHealth else { return "Connected" }
    return health.serverReachable ? "Connected" : "Unavailable"
  }

  private var connectionSymbol: String {
    model.sessionHealth?.serverReachable == false ? "exclamationmark.circle.fill" : "circle.fill"
  }

  private var connectionColor: Color {
    model.sessionHealth?.serverReachable == false ? .red : .green
  }
}
