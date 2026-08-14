import AppKit
import SwiftUI
import TableRockFeature

struct CatalogOutline: NSViewRepresentable {
  let table: [WorkbenchCatalogNode]
  @Binding var selection: String?
  let refreshState: CatalogRefreshState
  let onExpand: @MainActor (String) -> Void
  let onOpen: @MainActor (String) -> Void

  func makeCoordinator() -> Coordinator {
    Coordinator(
      table: table,
      selection: $selection,
      refreshState: refreshState,
      onExpand: onExpand,
      onOpen: onOpen
    )
  }

  func makeNSView(context: Context) -> NSScrollView {
    let outline = NSOutlineView()
    outline.delegate = context.coordinator
    outline.dataSource = context.coordinator
    outline.headerView = nil
    // Keep the row geometry large enough for the high-contrast 18 pt catalog label.
    // A small row clips the rendered/accessibility frame back to 16 pt even though
    // the text field's font is larger.
    outline.rowSizeStyle = .large
    outline.allowsMultipleSelection = false
    outline.autosaveExpandedItems = false
    outline.setAccessibilityLabel("Database catalog")
    outline.setAccessibilityIdentifier("catalog.outline")
    outline.target = context.coordinator
    outline.doubleAction = #selector(Coordinator.openSelectedObject)
    let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("catalog-name"))
    column.title = "Name"
    column.minWidth = 120
    column.resizingMask = .autoresizingMask
    outline.addTableColumn(column)
    outline.outlineTableColumn = column
    context.coordinator.outline = outline
    outline.reloadData()
    context.coordinator.expandDefaultRoots()

    let scroll = NSScrollView()
    scroll.documentView = outline
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    return scroll
  }

  func updateNSView(_ scroll: NSScrollView, context: Context) {
    guard let outline = scroll.documentView as? NSOutlineView else { return }
    let expanded = context.coordinator.expandedKeys()
    let selected = context.coordinator.selectedKey()
    context.coordinator.selection = $selection
    context.coordinator.onExpand = onExpand
    context.coordinator.onOpen = onOpen
    context.coordinator.rebuild(from: table, refreshState: refreshState)
    outline.reloadData()
    context.coordinator.restore(expanded: expanded, selected: selected)
  }

  @MainActor
  final class Node: NSObject {
    let key: String
    let title: String
    let symbol: String?
    let children: [Node]
    let isState: Bool
    let expandable: Bool

    init(
      key: String,
      title: String,
      symbol: String? = nil,
      children: [Node] = [],
      isState: Bool = false,
      expandable: Bool = false
    ) {
      self.key = key
      self.title = title
      self.symbol = symbol
      self.children = children
      self.isState = isState
      self.expandable = expandable
    }
  }

  @MainActor
  final class Coordinator: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
    private(set) var roots: [Node] = []
    private var nodesByKey: [String: Node] = [:]
    var selection: Binding<String?>
    var onExpand: @MainActor (String) -> Void
    var onOpen: @MainActor (String) -> Void
    weak var outline: NSOutlineView?
    private var suppressExpansionCallbacks = false
    private var previousSelectedRow = -1

    init(
      table: [WorkbenchCatalogNode],
      selection: Binding<String?>,
      refreshState: CatalogRefreshState,
      onExpand: @escaping @MainActor (String) -> Void,
      onOpen: @escaping @MainActor (String) -> Void
    ) {
      self.selection = selection
      self.onExpand = onExpand
      self.onOpen = onOpen
      super.init()
      rebuild(from: table, refreshState: refreshState)
    }

    func rebuild(from table: [WorkbenchCatalogNode], refreshState: CatalogRefreshState) {
      let byParent = Dictionary(grouping: table, by: \.parentIdBytes)
      func build(_ record: WorkbenchCatalogNode) -> Node {
        let key = catalogNodeKey(record.idBytes)
        var children = (byParent[record.idBytes] ?? []).map(build)
        switch refreshState {
        case .loading(let nodeKey) where nodeKey == key:
          children.append(
            Node(
              key: "state:loading:\(key)", title: "Loading…", isState: true))
        case .stale(let nodeKey, let message) where nodeKey == key:
          children.append(
            Node(
              key: "state:stale:\(key)",
              title: "Stale · \(message)",
              isState: true
            ))
        default:
          break
        }
        return Node(
          key: key,
          title: record.name,
          symbol: catalogSymbol(for: record.kind),
          children: children,
          expandable: record.expandable
        )
      }
      roots = (byParent[nil] ?? []).map(build)
      nodesByKey = [:]
      func index(_ node: Node) {
        nodesByKey[node.key] = node
        node.children.forEach(index)
      }
      roots.forEach(index)
    }

    func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int {
      (item as? Node)?.children.count ?? roots.count
    }

    func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any {
      (item as? Node)?.children[index] ?? roots[index]
    }

    func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool {
      guard let node = item as? Node else { return false }
      return node.expandable || !node.children.isEmpty
    }

    func outlineView(
      _ outlineView: NSOutlineView,
      viewFor tableColumn: NSTableColumn?,
      item: Any
    ) -> NSView? {
      guard let node = item as? Node else { return nil }
      let identifier = NSUserInterfaceItemIdentifier("catalog-cell")
      let cell: NSTableCellView
      if let reused = outlineView.makeView(withIdentifier: identifier, owner: nil)
        as? NSTableCellView
      {
        cell = reused
      } else {
        cell = NSTableCellView()
        cell.identifier = identifier
        let label = NSTextField(labelWithString: "")
        label.lineBreakMode = .byTruncatingTail
        label.font = .systemFont(ofSize: 18, weight: .heavy)
        label.drawsBackground = true
        label.backgroundColor = .labelColor
        label.textColor = .windowBackgroundColor
        label.translatesAutoresizingMaskIntoConstraints = false
        let image = NSImageView()
        image.imageScaling = .scaleProportionallyDown
        image.contentTintColor = .secondaryLabelColor
        image.setAccessibilityElement(false)
        image.translatesAutoresizingMaskIntoConstraints = false
        cell.imageView = image
        cell.textField = label
        cell.addSubview(image)
        cell.addSubview(label)
        NSLayoutConstraint.activate([
          image.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 2),
          image.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
          image.widthAnchor.constraint(equalToConstant: 14),
          image.heightAnchor.constraint(equalToConstant: 14),
          label.leadingAnchor.constraint(equalTo: image.trailingAnchor, constant: 5),
          label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -2),
          label.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
        ])
      }
      cell.imageView?.image = node.symbol.flatMap {
        NSImage(systemSymbolName: $0, accessibilityDescription: nil)
      }
      cell.imageView?.isHidden = node.symbol == nil
      cell.textField?.stringValue = node.title
      let selected = outlineView.selectedRow == outlineView.row(forItem: node)
      cell.imageView?.contentTintColor =
        selected ? .alternateSelectedControlTextColor : .secondaryLabelColor
      cell.textField?.backgroundColor = .labelColor
      cell.textField?.textColor = .windowBackgroundColor
      cell.setAccessibilityLabel(
        node.isState
          ? "Catalog state \(node.title)"
          : node.children.isEmpty
            ? "Catalog object \(node.title)" : "Catalog group \(node.title)")
      cell.setAccessibilityIdentifier("catalog.node.\(node.key)")
      return cell
    }

    private func catalogSymbol(for kind: String) -> String {
      switch kind {
      case "postgresql_database", "clickhouse_database": "cylinder"
      case "postgresql_schema": "folder"
      case "postgresql_table", "postgresql_foreign_table", "postgresql_partitioned_table",
        "clickhouse_table":
        "tablecells"
      case "postgresql_view", "postgresql_materialized_view", "clickhouse_view",
        "clickhouse_materialized_view":
        "eye"
      case "postgresql_function": "function"
      case "postgresql_type": "curlybraces"
      case "clickhouse_dictionary": "book.closed"
      case "redis_logical_database": "square.stack.3d.up"
      case "redis_namespace": "folder"
      default: kind.hasPrefix("redis_key_") ? "key.horizontal" : "circle"
      }
    }

    func outlineViewItemDidExpand(_ notification: Notification) {
      guard !suppressExpansionCallbacks,
        let node = notification.userInfo?["NSObject"] as? Node,
        node.key.hasPrefix("node:")
      else { return }
      onExpand(node.key)
    }

    func outlineViewSelectionDidChange(_ notification: Notification) {
      let selectedRow = outline?.selectedRow ?? -1
      let changedRows = IndexSet([previousSelectedRow, selectedRow].filter { $0 >= 0 })
      if let outline, !changedRows.isEmpty {
        outline.reloadData(
          forRowIndexes: changedRows,
          columnIndexes: IndexSet(integer: 0)
        )
      }
      previousSelectedRow = selectedRow
      guard let outline, outline.selectedRow >= 0,
        let node = outline.item(atRow: outline.selectedRow) as? Node
      else {
        selection.wrappedValue = nil
        return
      }
      selection.wrappedValue = node.key
    }

    @objc func openSelectedObject() {
      guard let outline, outline.selectedRow >= 0,
        let node = outline.item(atRow: outline.selectedRow) as? Node,
        !node.isState
      else { return }
      onOpen(node.key)
    }

    func expandedKeys() -> Set<String> {
      Set(nodesByKey.values.filter { outline?.isItemExpanded($0) == true }.map(\.key))
    }

    func selectedKey() -> String? {
      guard let outline, outline.selectedRow >= 0 else { return selection.wrappedValue }
      return (outline.item(atRow: outline.selectedRow) as? Node)?.key
    }

    func restore(expanded: Set<String>, selected: String?) {
      guard let outline else { return }
      suppressExpansionCallbacks = true
      defer { suppressExpansionCallbacks = false }
      for key in expanded {
        if let node = nodesByKey[key] { outline.expandItem(node) }
      }
      if let selected, let node = nodesByKey[selected] {
        let row = outline.row(forItem: node)
        if row >= 0 {
          outline.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
        }
      }
    }

    func expandDefaultRoots() {
      guard let outline else { return }
      suppressExpansionCallbacks = true
      defer { suppressExpansionCallbacks = false }
      for root in roots where !root.children.isEmpty {
        outline.expandItem(root)
      }
    }
  }
}
