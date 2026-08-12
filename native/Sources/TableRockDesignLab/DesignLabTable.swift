import AppKit
import SwiftUI

private final class LabSelectionTableView: NSTableView {
    override func mouseDown(with event: NSEvent) {
        let clickedRow = row(at: convert(event.locationInWindow, from: nil))
        super.mouseDown(with: event)
        selectClickedRowIfNeeded(clickedRow)
    }

    override func rightMouseDown(with event: NSEvent) {
        let clickedRow = row(at: convert(event.locationInWindow, from: nil))
        selectClickedRowIfNeeded(clickedRow)
        super.rightMouseDown(with: event)
    }

    private func selectClickedRowIfNeeded(_ row: Int) {
        guard row >= 0, selectedRow != row else { return }
        selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
    }
}

private final class LabTableCellView: NSTableCellView {
    override func hitTest(_ point: NSPoint) -> NSView? {
        // Keep labels accessible while letting NSTableView own pointer
        // selection and context-menu behavior for the whole row.
        nil
    }
}

struct LabNativeDataTable: NSViewRepresentable {
    let rows: [LabRow]
    let columns: [LabColumn]
    @Binding var selectedRowID: Int?
    let openInspector: () -> Void
    let openQuery: () -> Void
    let reviewChanges: () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let table = LabSelectionTableView()
        table.identifier = NSUserInterfaceItemIdentifier("design-lab-native-grid")
        table.setAccessibilityLabel("Customer result grid")
        table.setAccessibilityHelp(
            "Select rows, resize or reorder columns, and open row actions."
        )
        table.usesAlternatingRowBackgroundColors = true
        table.allowsMultipleSelection = false
        table.allowsColumnReordering = true
        table.allowsColumnResizing = true
        table.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        table.rowHeight = 28
        table.intercellSpacing = NSSize(width: 1, height: 1)
        table.selectionHighlightStyle = .regular
        table.delegate = context.coordinator
        table.dataSource = context.coordinator
        table.menu = context.coordinator.makeContextMenu()

        for column in columns {
            let tableColumn = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(column.id))
            tableColumn.title = "\(column.title) · \(column.type)"
            tableColumn.width = column.width
            tableColumn.minWidth = 72
            tableColumn.maxWidth = max(column.width * 2, 220)
            table.addTableColumn(tableColumn)
        }

        let scrollView = NSScrollView()
        scrollView.identifier = NSUserInterfaceItemIdentifier("design-lab-grid-scroll")
        scrollView.setAccessibilityLabel("Customer result grid")
        scrollView.documentView = table
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true
        scrollView.drawsBackground = true
        scrollView.backgroundColor = .textBackgroundColor

        context.coordinator.tableView = table
        context.coordinator.restoreSelection()
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let table = scrollView.documentView as? NSTableView else { return }
        if context.coordinator.renderedRows != rows {
            context.coordinator.renderedRows = rows
            table.reloadData()
        }
        context.coordinator.restoreSelection()
    }

    @MainActor
    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate {
        var parent: LabNativeDataTable
        var renderedRows: [LabRow]
        weak var tableView: NSTableView?

        init(parent: LabNativeDataTable) {
            self.parent = parent
            renderedRows = parent.rows
        }

        func numberOfRows(in tableView: NSTableView) -> Int {
            parent.rows.count
        }

        func tableView(
            _ tableView: NSTableView,
            viewFor tableColumn: NSTableColumn?,
            row: Int
        ) -> NSView? {
            guard row < parent.rows.count,
                  let tableColumn,
                  let columnIndex = parent.columns.firstIndex(where: {
                      $0.id == tableColumn.identifier.rawValue
                  })
            else { return nil }

            let identifier = NSUserInterfaceItemIdentifier("lab-grid-cell")
            let cell = (tableView.makeView(withIdentifier: identifier, owner: self) as? NSTableCellView)
                ?? makeCell(identifier: identifier)
            let value = parent.rows[row].values[columnIndex]
            cell.textField?.stringValue = value
            cell.textField?.alignment = alignment(for: parent.columns[columnIndex].id)
            cell.toolTip = value
            cell.setAccessibilityLabel("\(parent.columns[columnIndex].title), \(value)")
            return cell
        }

        func tableViewSelectionDidChange(_ notification: Notification) {
            guard let tableView,
                  tableView.selectedRow >= 0,
                  tableView.selectedRow < parent.rows.count
            else { return }
            let rowID = parent.rows[tableView.selectedRow].id
            if parent.selectedRowID != rowID {
                parent.selectedRowID = rowID
            }
            parent.openInspector()
        }

        func makeContextMenu() -> NSMenu {
            let menu = NSMenu(title: "Data Grid")
            menu.addItem(withTitle: "Inspect Selected Value", action: #selector(inspect), keyEquivalent: "")
            menu.addItem(withTitle: "Open Selection in Query", action: #selector(openQuery), keyEquivalent: "")
            menu.addItem(.separator())
            menu.addItem(withTitle: "Review Pending Changes…", action: #selector(review), keyEquivalent: "")
            for item in menu.items { item.target = self }
            return menu
        }

        func restoreSelection() {
            guard let tableView,
                  let selectedRowID = parent.selectedRowID,
                  let index = parent.rows.firstIndex(where: { $0.id == selectedRowID })
            else { return }
            guard tableView.selectedRow != index else { return }
            tableView.selectRowIndexes(IndexSet(integer: index), byExtendingSelection: false)
            tableView.scrollRowToVisible(index)
        }

        @objc private func inspect() {
            parent.openInspector()
        }

        @objc private func openQuery() {
            parent.openQuery()
        }

        @objc private func review() {
            parent.reviewChanges()
        }

        private func makeCell(identifier: NSUserInterfaceItemIdentifier) -> NSTableCellView {
            let cell = LabTableCellView()
            cell.identifier = identifier
            let label = NSTextField(labelWithString: "")
            label.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
            label.lineBreakMode = .byTruncatingTail
            label.translatesAutoresizingMaskIntoConstraints = false
            cell.textField = label
            cell.addSubview(label)
            NSLayoutConstraint.activate([
                label.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 8),
                label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -8),
                label.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
            ])
            return cell
        }

        private func alignment(for columnID: String) -> NSTextAlignment {
            ["seats", "mrr"].contains(columnID) ? .right : .left
        }
    }
}
