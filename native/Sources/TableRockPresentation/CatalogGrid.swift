import AppKit
import SwiftUI
import TableRockFeature

struct CatalogGrid: NSViewRepresentable {
  let table: WorkbenchTable
  let sorts: [WorkbenchBrowseSort]
  let performanceAutoScroll: Bool
  let onSelect: @MainActor (Int, Int) -> Void

  init(
    table: WorkbenchTable,
    sorts: [WorkbenchBrowseSort] = [],
    performanceAutoScroll: Bool,
    onSelect: @escaping @MainActor (Int, Int) -> Void = { _, _ in }
  ) {
    self.table = table
    self.sorts = sorts
    self.performanceAutoScroll = performanceAutoScroll
    self.onSelect = onSelect
  }

  func makeCoordinator() -> Coordinator {
    Coordinator(
      table,
      sorts: sorts,
      performanceAutoScroll: performanceAutoScroll,
      onSelect: onSelect
    )
  }

  final class ResultTableView: NSTableView {
    var onCellActivate: ((Int, Int) -> Void)?

    override func mouseDown(with event: NSEvent) {
      let point = convert(event.locationInWindow, from: nil)
      let activatedRow = row(at: point)
      let activatedColumn = column(at: point)
      super.mouseDown(with: event)
      if activatedRow >= 0, activatedColumn >= 0 {
        onCellActivate?(activatedRow, activatedColumn)
      }
    }
  }

  func makeNSView(context: Context) -> NSScrollView {
    let grid = ResultTableView()
    grid.usesAlternatingRowBackgroundColors = true
    grid.allowsColumnReordering = true
    grid.allowsColumnResizing = true
    grid.allowsMultipleSelection = true
    grid.rowSizeStyle = .small
    // Opaque content surface — never glass (Liquid Glass / design-system).
    grid.backgroundColor = .textBackgroundColor
    grid.gridStyleMask = []  // no interior gridlines; hierarchy via alternating rows
    grid.intercellSpacing = NSSize(width: 6, height: 1)
    grid.setAccessibilityLabel("Query results")
    grid.setAccessibilityIdentifier("results.grid")
    let scroll = NSScrollView()
    scroll.documentView = grid
    scroll.drawsBackground = true
    scroll.backgroundColor = .textBackgroundColor
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    // Content surface: separator only, not heavy bezel chrome.
    scroll.borderType = .lineBorder
    scroll.focusRingType = .exterior
    context.coordinator.installColumns(on: grid)
    grid.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
    grid.delegate = context.coordinator
    grid.dataSource = context.coordinator
    grid.target = context.coordinator
    grid.action = #selector(Coordinator.tableClicked(_:))
    grid.onCellActivate = { [weak coordinator = context.coordinator, weak grid] row, column in
      guard let grid else { return }
      coordinator?.activate(row: row, column: column, in: grid)
    }
    context.coordinator.startPerformanceScrollIfRequested(on: grid)
    return scroll
  }

  func updateNSView(_ scroll: NSScrollView, context: Context) {
    guard let grid = scroll.documentView as? NSTableView else { return }
    let selectedRows = grid.selectedRowIndexes
    context.coordinator.snapshot = table
    context.coordinator.sorts = sorts
    context.coordinator.performanceAutoScroll = performanceAutoScroll
    context.coordinator.onSelect = onSelect
    if let resultGrid = grid as? ResultTableView {
      resultGrid.onCellActivate = {
        [weak coordinator = context.coordinator, weak resultGrid] row, column in
        guard let resultGrid else { return }
        coordinator?.activate(row: row, column: column, in: resultGrid)
      }
    }
    context.coordinator.installColumns(on: grid)
    grid.reloadData()
    context.coordinator.startPerformanceScrollIfRequested(on: grid)
    let validSelection = selectedRows.filter { $0 < table.rows.count }
    grid.selectRowIndexes(IndexSet(validSelection), byExtendingSelection: false)
  }

  @MainActor
  final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    final class ResultCellView: NSTableCellView {}

    final class ResultCellButton: NSButton {
      var onActivate: (() -> Void)?

      @objc func activateCell() {
        onActivate?()
      }

      override func mouseDown(with event: NSEvent) {
        onActivate?()
        super.mouseDown(with: event)
      }

      override func accessibilityPerformPress() -> Bool {
        onActivate?()
        return true
      }
    }

    var snapshot: WorkbenchTable
    var sorts: [WorkbenchBrowseSort]
    var performanceAutoScroll: Bool
    var onSelect: @MainActor (Int, Int) -> Void
    private var performanceScrollTask: Task<Void, Never>?
    private var lastActivatedColumn = 0

    init(
      _ snapshot: WorkbenchTable,
      sorts: [WorkbenchBrowseSort],
      performanceAutoScroll: Bool,
      onSelect: @escaping @MainActor (Int, Int) -> Void
    ) {
      self.snapshot = snapshot
      self.sorts = sorts
      self.performanceAutoScroll = performanceAutoScroll
      self.onSelect = onSelect
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
      guard let tableView = notification.object as? NSTableView,
        tableView.selectedRow >= 0
      else { return }
      let column =
        tableView.clickedColumn >= 0
        ? tableView.clickedColumn : lastActivatedColumn
      guard snapshot.columns.indices.contains(column) else { return }
      onSelect(tableView.selectedRow, column)
    }

    @objc func tableClicked(_ tableView: NSTableView) {
      let row = tableView.clickedRow
      let column = tableView.clickedColumn
      guard row >= 0, column >= 0 else { return }
      activate(row: row, column: column, in: tableView)
    }

    func activate(row: Int, column: Int, in tableView: NSTableView) {
      guard snapshot.rows.indices.contains(row), snapshot.columns.indices.contains(column) else {
        return
      }
      lastActivatedColumn = column
      tableView.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
      onSelect(row, column)
    }

    func startPerformanceScrollIfRequested(on tableView: NSTableView) {
      guard performanceScrollTask == nil,
        performanceAutoScroll,
        !snapshot.rows.isEmpty
      else { return }
      let finalRow = snapshot.rows.count - 1
      #if TABLEROCK_DEVELOPMENT_SUPPORT
        writePerformanceMetric("PERF_SCROLL_ARMED rows=\(finalRow + 1)")
      #endif
      performanceScrollTask = Task { @MainActor [weak tableView] in
        try? await Task.sleep(for: .milliseconds(500))
        guard let tableView, !Task.isCancelled else { return }
        let started = Date()
        for row in stride(from: 0, through: finalRow, by: 250) {
          tableView.scrollRowToVisible(row)
          try? await Task.sleep(for: .milliseconds(16))
        }
        for row in stride(from: finalRow, through: 0, by: -250) {
          tableView.scrollRowToVisible(row)
          try? await Task.sleep(for: .milliseconds(16))
        }
        let elapsed = Date().timeIntervalSince(started)
        #if TABLEROCK_DEVELOPMENT_SUPPORT
          writePerformanceMetric(
            "PERF_SCROLL_DONE rows=\(finalRow + 1) elapsed_seconds=\(String(format: "%.6f", elapsed))"
          )
        #else
          _ = elapsed
        #endif
      }
    }

    func numberOfRows(in tableView: NSTableView) -> Int { snapshot.rows.count }

    func installColumns(on tableView: NSTableView) {
      let expected = snapshot.columns.indices.map {
        NSUserInterfaceItemIdentifier("result-column-\($0)")
      }
      if tableView.tableColumns.map(\.identifier) == expected {
        for (column, title) in zip(tableView.tableColumns, snapshot.columns) {
          column.title = workbenchColumnHeaderTitle(column: title, sorts: sorts)
        }
        return
      }
      for column in tableView.tableColumns { tableView.removeTableColumn(column) }
      for (index, title) in snapshot.columns.enumerated() {
        let column = NSTableColumn(
          identifier: NSUserInterfaceItemIdentifier("result-column-\(index)"))
        column.title = workbenchColumnHeaderTitle(column: title, sorts: sorts)
        column.minWidth = 72
        column.width = 148
        column.resizingMask = [.autoresizingMask, .userResizingMask]
        if snapshot.columnMetadata.indices.contains(index) {
          let meta = snapshot.columnMetadata[index]
          let nullability = meta.nullable ? "nullable" : "not null"
          column.headerToolTip = "\(meta.engineType) · \(nullability)"
        }
        tableView.addTableColumn(column)
      }
    }

    func tableView(
      _ tableView: NSTableView,
      viewFor tableColumn: NSTableColumn?,
      row: Int
    ) -> NSView? {
      guard let tableColumn,
        let column = tableView.tableColumns.firstIndex(of: tableColumn),
        snapshot.rows.indices.contains(row),
        snapshot.rows[row].indices.contains(column)
      else { return nil }
      let identifier = NSUserInterfaceItemIdentifier("result-cell")
      let cell: ResultCellView
      if let reused = tableView.makeView(withIdentifier: identifier, owner: nil)
        as? ResultCellView
      {
        cell = reused
      } else {
        cell = ResultCellView()
        cell.identifier = identifier
        let button = ResultCellButton(title: "", target: nil, action: nil)
        button.target = button
        button.action = #selector(ResultCellButton.activateCell)
        button.identifier = NSUserInterfaceItemIdentifier("result-cell-button")
        button.isBordered = false
        button.alignment = .left
        button.lineBreakMode = .byTruncatingTail
        // Monospaced digits for professional dense tables (numbers align visually).
        button.font = NSFont.monospacedDigitSystemFont(
          ofSize: NSFont.smallSystemFontSize, weight: .regular)
        button.translatesAutoresizingMaskIntoConstraints = false
        cell.addSubview(button)
        NSLayoutConstraint.activate([
          button.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
          button.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
          button.topAnchor.constraint(equalTo: cell.topAnchor),
          button.bottomAnchor.constraint(equalTo: cell.bottomAnchor),
        ])
      }
      let typed: WorkbenchCell = {
        if snapshot.cells.indices.contains(row),
          snapshot.cells[row].indices.contains(column)
        {
          return snapshot.cells[row][column]
        }
        let raw = snapshot.rows[row][column]
        return WorkbenchCell(
          display: raw, kind: 7, truncation: 0, originalByteCount: nil, bytes: Data(raw.utf8))
      }()
      let presentation = GridCellPresentation.project(typed)
      guard let button = cell.subviews.first as? ResultCellButton else { return nil }
      button.title = presentation.title
      button.alignment = presentation.isNumeric ? .right : .left
      // Null/empty use secondary label color *plus* glyph (never color alone).
      button.contentTintColor =
        presentation.isNull || presentation.title == "·"
        ? NSColor.secondaryLabelColor : NSColor.labelColor
      let engineType =
        snapshot.columnMetadata.indices.contains(column)
        ? snapshot.columnMetadata[column].engineType : "unknown"
      button.setAccessibilityElement(true)
      button.setAccessibilityRole(.button)
      button.setAccessibilityLabel(
        "\(snapshot.columns[column]), \(engineType), row \(row + 1), \(presentation.kindLabel)")
      button.setAccessibilityValue(presentation.accessibilityValue)
      button.setAccessibilityIdentifier("results.cell.\(row).\(column)")
      button.onActivate = { [weak self, weak tableView] in
        guard let self, let tableView else { return }
        self.activate(row: row, column: column, in: tableView)
      }
      cell.setAccessibilityElement(false)
      return cell
    }
  }
}
