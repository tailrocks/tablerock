import AppKit
import Foundation
import SwiftUI
import TableRockFeature

struct ResultGridWithInspector: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let table: WorkbenchTable
  let minimumHeight: CGFloat
  var exposesResultPaging = false
  var showsUtilityRail = true

  private var visibleRowIndices: [Int] {
    let term = model.loadedRowQuickFilter.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !term.isEmpty else { return Array(table.rows.indices) }
    return table.rows.indices.filter { row in
      table.rows[row].contains { value in
        value.range(of: term, options: [.caseInsensitive, .diacriticInsensitive]) != nil
      }
    }
  }

  private var visibleTable: WorkbenchTable {
    WorkbenchTable(
      columns: table.columns,
      rows: visibleRowIndices.map { table.rows[$0] },
      columnMetadata: table.columnMetadata,
      cells: visibleRowIndices.map { table.cells[$0] })
  }

  private var selectedVisibleRow: Int? {
    guard let selected = model.selectedCell?.row else { return nil }
    return visibleRowIndices.firstIndex(of: selected)
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      if showsUtilityRail {
        ResultUtilityRail(
          visibleRowCount: visibleRowIndices.count,
          totalRowCount: table.rows.count,
          exposesResultPaging: exposesResultPaging
        )
      }
      if table.rows.isEmpty {
        ContentUnavailableView(
          "No rows in this result",
          systemImage: "tablecells",
          description: Text("Run a query or open a table that returns rows.")
        )
        .frame(minHeight: minimumHeight)
        .accessibilityIdentifier("results.grid.empty")
      } else {
        HSplitView {
          CatalogGrid(
            table: visibleTable,
            sorts: model.resultSort,
            selectedRow: selectedVisibleRow,
            performanceAutoScroll: performanceAutoScroll
          ) { row, column in
            guard visibleRowIndices.indices.contains(row) else { return }
            model.selectCell(row: visibleRowIndices[row], column: column)
          }
          .frame(minWidth: 280, minHeight: 100, idealHeight: minimumHeight)
          if let continuum = model.relationContinuum {
            RelationContinuumPlane(state: continuum) {
              model.closeRelationContinuum()
            }
            .frame(minWidth: 220, idealWidth: 320, maxWidth: 480)
          }
        }
      }
    }
    .background(Color(nsColor: .textBackgroundColor))
  }

  private var performanceAutoScroll: Bool {
    #if TABLEROCK_DEVELOPMENT_SUPPORT
      model.fixtures.performanceAutoScroll
    #else
      false
    #endif
  }
}

private struct ResultUtilityRail: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let visibleRowCount: Int
  let totalRowCount: Int
  let exposesResultPaging: Bool

  var body: some View {
    HStack(spacing: 8) {
      TextField(
        "Filter loaded rows",
        text: Binding(
          get: { model.loadedRowQuickFilter },
          set: { model.loadedRowQuickFilter = $0 })
      )
      .textFieldStyle(.roundedBorder)
      .frame(minWidth: 120, maxWidth: 200)
      .accessibilityIdentifier("results.quick-filter")
      let loadedRowsStatus = "Loaded rows only · \(visibleRowCount)/\(totalRowCount)"
      Text(loadedRowsStatus)
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("results.quick-filter.status")
        .accessibilityValue(loadedRowsStatus)
      if let selection = model.selectedCellSnapshot {
        let presentation = GridCellPresentation.project(selection.1)
        Text(
          "\(selection.0.name) · \(presentation.statusFact) · R\(selection.2 + 1) C\(selection.3 + 1)"
        )
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .lineLimit(1)
        .accessibilityIdentifier("results.selection.status")
        .accessibilityValue(
          "\(selection.0.name), \(presentation.accessibilityValue), row \(selection.2 + 1)")
      }
      Spacer(minLength: 0)
      if exposesResultPaging && model.nextStartRow != nil {
        Button("Load more") { Task { await model.loadMore() } }
          .accessibilityIdentifier("results.next-page")
      }
      if model.canEditSelectedRow {
        Button("Edit Selected", systemImage: "pencil") {
          model.showSelectedRowEditor()
        }
        .accessibilityIdentifier("mutation.edit-selected")
      }
      ResultCopyMenu()
      ResultExportMenu()
      Button {
        Task { await model.openRelationContinuumFromSelection() }
      } label: {
        Label("Continuum", systemImage: "arrow.triangle.branch")
      }
      .disabled(!model.canOpenRelationContinuum)
      .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
      .help("Row Continuum: related rows for this cell (⌘⌥→)")
      .accessibilityIdentifier("relation.continuum.open")
      if model.relationContinuumLoading {
        ProgressView().accessibilityLabel("Loading related rows")
      }
      if model.relationContinuum != nil {
        Button("Close Continuum") { model.closeRelationContinuum() }
          .keyboardShortcut(.leftArrow, modifiers: [.command, .option])
          .accessibilityIdentifier("relation.continuum.close")
      }
      if let outcome = model.copyOutcome {
        Text(outcome)
          .font(.caption)
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("results.copy.outcome")
          .accessibilityValue(outcome)
      }
      if let error = model.copyError {
        Text(error).font(.caption).foregroundStyle(.red)
      }
      if let continuumError = model.relationContinuumError {
        Text(continuumError)
          .font(.caption)
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("relation.continuum.error")
      }
      if let outcome = model.activeObjectTab?.mutationOutcome {
        Text(outcome)
          .font(.caption)
          .foregroundStyle(.green)
          .accessibilityIdentifier("mutation.outcome")
      }
    }
    .font(.caption)
    .controlSize(.small)
    .padding(.horizontal, 10)
    .frame(height: 38)
    .background(Color(nsColor: .controlBackgroundColor))
    .overlay(alignment: .bottom) { Divider() }
  }
}

/// Spatial peer plane for Row Continuum (opaque content; chrome is labels only).
private struct RelationContinuumPlane: View {
  let state: RelationContinuumState
  let onClose: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .firstTextBaseline) {
        VStack(alignment: .leading, spacing: 2) {
          Text("CONTINUUM")
            .font(.caption.weight(.bold))
            .tracking(0.6)
          Text(state.edgeTitle)
            .font(.caption.monospaced())
            .textSelection(.enabled)
        }
        Spacer(minLength: 0)
        Text(state.statusWord)
          .font(.caption2.weight(.semibold))
          .accessibilityLabel("Continuum status \(state.statusWord)")
        Button(action: onClose) {
          Image(systemName: "xmark")
        }
        .buttonStyle(.borderless)
        .accessibilityLabel("Close Continuum")
      }
      Text("\(state.directionWord) · \(state.fromColumn) = \(state.fromValue)")
        .font(.caption)
        .foregroundStyle(.secondary)
      if state.rows.isEmpty {
        ContentUnavailableView(
          "No related rows",
          systemImage: "arrow.triangle.branch",
          description: Text("The related table returned no rows for this value.")
        )
        .frame(maxHeight: .infinity)
      } else {
        // Dense opaque preview grid (not glass). AppKit NSTableView remains
        // primary for full results; this plane is a focused neighbor surface.
        VStack(alignment: .leading, spacing: 0) {
          HStack(spacing: 8) {
            ForEach(state.columns, id: \.self) { name in
              Text(name)
                .font(.caption2.weight(.semibold))
                .frame(maxWidth: .infinity, alignment: .leading)
            }
          }
          .padding(.vertical, 4)
          Divider()
          ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
              ForEach(Array(state.rows.enumerated()), id: \.offset) { _, cells in
                HStack(spacing: 8) {
                  ForEach(Array(cells.enumerated()), id: \.offset) { _, cell in
                    Text(cell)
                      .font(.system(.caption, design: .monospaced))
                      .frame(maxWidth: .infinity, alignment: .leading)
                      .textSelection(.enabled)
                  }
                }
                .padding(.vertical, 2)
              }
            }
          }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
    }
    .padding(10)
    .background(Color(nsColor: .textBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("relation.continuum.plane")
    .accessibilityLabel(
      "Relation Continuum \(state.edgeTitle), \(state.rows.count) rows, \(state.statusWord)")
  }
}

struct ResultExportMenu: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    Menu {
      exportButton("CSV", format: "csv")
      exportButton("TSV", format: "tsv")
      exportButton("JSON", format: "json")
      exportButton("Markdown", format: "markdown")
      if model.sqlInsertCopyAvailable {
        exportButton("SQL INSERT", format: "sql_insert")
      }
      Divider()
      fullExportButton("Full Result CSV", format: "csv")
      fullExportButton("Full Result TSV", format: "tsv")
      fullExportButton("Full Result JSON", format: "json")
    } label: {
      Label("Export", systemImage: "square.and.arrow.up")
    }
    .fixedSize(horizontal: true, vertical: true)
    .disabled(model.resultIdData == nil)
    .accessibilityIdentifier("results.export.more")
    .accessibilityHint("Export loaded rows or stream the full result")
  }

  private func exportButton(_ label: String, format: String) -> some View {
    Button(label) { Task { await model.exportLoadedResult(format: format) } }
      .accessibilityIdentifier("results.export.\(format)")
  }

  private func fullExportButton(_ label: String, format: String) -> some View {
    Button(label) { Task { await model.exportFullResult(format: format) } }
      .accessibilityIdentifier("results.export.full.\(format)")
  }
}

struct ResultCopyMenu: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    Menu {
      Section("Selected cell") {
        copyButtons(scope: "cell")
      }
      Section("Selected row") {
        copyButtons(scope: "row")
      }
      Section("Loaded result") {
        copyButtons(scope: "loaded")
      }
    } label: {
      Label("Copy", systemImage: "doc.on.doc")
    }
    .disabled(model.resultIdData == nil)
    .accessibilityHint("Choose scope and Rust-formatted clipboard representation")
  }

  @ViewBuilder
  private func copyButtons(scope: String) -> some View {
    Button("TSV") { Task { await model.copyResult(scope: scope, preferredFormat: "tsv") } }
    Button("CSV") { Task { await model.copyResult(scope: scope, preferredFormat: "csv") } }
    Button("JSON") { Task { await model.copyResult(scope: scope, preferredFormat: "json") } }
    Button("Markdown") {
      Task { await model.copyResult(scope: scope, preferredFormat: "markdown") }
    }
    if model.sqlInsertCopyAvailable {
      Button("SQL INSERT") {
        Task { await model.copyResult(scope: scope, preferredFormat: "sql_insert") }
      }
    }
  }
}

/// Kind-first trailing inspector: opaque content plane matching Continuum density.
/// Rust/UniFFI own typed bytes; Swift only projects text, hex dump, and bounded JSON tree.
struct NativeValueInspector: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let column: WorkbenchColumn
  let cell: WorkbenchCell
  let row: Int
  let columnIndex: Int

  @State private var hexExpanded: Bool = true

  private struct InspectorRow: Identifiable {
    let id: Int
    let column: WorkbenchColumn
    let cell: WorkbenchCell
  }

  private var presentation: GridCellPresentation {
    GridCellPresentation.project(cell)
  }

  private var hexLinear: String {
    ValueInspectorProjection.hexLinear(cell.bytes)
  }

  private var hexDump: String {
    ValueInspectorProjection.hexDump(cell.bytes)
  }

  private var metadataFact: String {
    ValueInspectorProjection.metadataFact(
      engineType: column.engineType,
      nullable: column.nullable,
      byteCount: cell.bytes.count,
      originalByteCount: cell.originalByteCount,
      isTruncated: cell.isTruncated)
  }

  private var structuredRows: [StructuredValueTreeRow]? {
    guard cell.kind == 8 else { return nil }
    return try? StructuredValueTree.decode(cell.bytes)
  }

  private var structuredTreeFailed: Bool {
    cell.kind == 8 && structuredRows == nil
  }

  private var prefersHexPrimary: Bool {
    cell.kind == 9  // Binary
  }

  /// Multi-line dump is progressive when large; compact linear hex always stays painted.
  private var hexNeedsDisclosure: Bool {
    cell.bytes.count > 64
  }

  private var typeFact: String {
    "\(column.engineType.uppercased()) · \(column.nullable ? "NULLABLE" : "NOT NULL")"
  }

  private var selectedDisplay: String {
    if presentation.isNull { return "NULL" }
    return cell.display.isEmpty ? "Empty" : cell.display
  }

  private var rowDetails: [InspectorRow] {
    guard let table = model.resultTable,
      table.cells.indices.contains(row)
    else { return [] }
    return table.columnMetadata.indices.compactMap { index in
      guard index != columnIndex,
        table.cells[row].indices.contains(index)
      else { return nil }
      return InspectorRow(
        id: index,
        column: table.columnMetadata[index],
        cell: table.cells[row][index]
      )
    }
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      header
      Divider()
      ScrollView {
        VStack(alignment: .leading, spacing: 14) {
          selectedColumnSection
          rowDetailsSection
          technicalDetails
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
    .background(Color(nsColor: .controlBackgroundColor))
    .onAppear {
      // Large blobs start collapsed; binary and small values keep hex open.
      hexExpanded = prefersHexPrimary || !hexNeedsDisclosure
    }
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("value.inspector")
    .accessibilityLabel(
      "Value inspector for \(column.name), \(presentation.accessibilityValue), \(ValueInspectorProjection.locationFact(row: row, columnIndex: columnIndex))"
    )
  }

  private var header: some View {
    HStack {
      Text("INSPECTOR")
        .font(.system(size: 10, weight: .bold))
        .foregroundStyle(.secondary)
      Spacer()
      Button {
        model.selectedCell = nil
      } label: {
        Image(systemName: "xmark")
      }
      .buttonStyle(.plain)
      .accessibilityLabel("Close inspector")
      .accessibilityIdentifier("value.inspector.close")
    }
    .padding(.horizontal, 12)
    .frame(height: 40)
  }

  private var selectedColumnSection: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .top) {
        VStack(alignment: .leading, spacing: 2) {
          Text(column.name)
            .font(.caption.weight(.semibold))
            .textSelection(.enabled)
            .lineLimit(2)
            .accessibilityIdentifier("value.inspector.column")
          Text(typeFact)
            .font(.caption)
            .foregroundStyle(.secondary)
            .accessibilityLabel(typeFact)
            .accessibilityIdentifier("value.inspector.kind")
        }
        Spacer(minLength: 4)
        Menu {
          Button("Copy Value") { copyToPasteboard(cell.display) }
            .disabled(presentation.isNull && cell.display.isEmpty)
          Button("Copy Hex") { copyToPasteboard(hexLinear) }
            .disabled(cell.bytes.isEmpty)
            .accessibilityIdentifier("value.inspector.copy.hex")
        } label: {
          Image(systemName: "doc.on.doc")
        }
        .menuStyle(.borderlessButton)
        .help("Copy value")
        .accessibilityLabel("Copy value options")
        .accessibilityIdentifier("value.inspector.copy.text")
      }

      Text(selectedDisplay)
        .font(.body.monospaced())
        .textSelection(.enabled)
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay {
          RoundedRectangle(cornerRadius: 7)
            .stroke(Color(nsColor: .separatorColor))
        }
        .accessibilityIdentifier("value.inspector.text")
        .accessibilityLabel(presentation.accessibilityValue)
    }
  }

  private var rowDetailsSection: some View {
    VStack(alignment: .leading, spacing: 10) {
      sectionLabel("ROW DETAILS")
      if rowDetails.isEmpty {
        Text("No additional columns")
          .font(.caption)
          .foregroundStyle(.secondary)
      } else {
        ForEach(rowDetails) { detail in
          VStack(alignment: .leading, spacing: 2) {
            Text(detail.column.name)
              .font(.caption)
              .foregroundStyle(.secondary)
            Text(GridCellPresentation.project(detail.cell).accessibilityValue)
              .font(.caption.monospaced())
              .textSelection(.enabled)
          }
          .accessibilityElement(children: .combine)
          .accessibilityIdentifier("value.inspector.row.\(detail.id)")
        }
      }
    }
    .accessibilityIdentifier("value.inspector.row-details")
  }

  private var technicalDetails: some View {
    VStack(alignment: .leading, spacing: 8) {
      sectionLabel("VALUE DETAILS")
      Text(metadataFact)
        .font(.caption2.monospaced())
        .foregroundStyle(.secondary)
        .textSelection(.enabled)
        .accessibilityIdentifier("value.inspector.metadata")

      if cell.isTruncated {
        Label(
          cell.originalByteCount.map { "Truncated — original \($0) B" }
            ?? "Truncated value",
          systemImage: "scissors"
        )
        .font(.caption.weight(.semibold))
        .accessibilityIdentifier("value.inspector.truncated")
      }

      if structuredTreeFailed {
        Text(
          "JSON tree unavailable — payload is not valid JSON within tree bounds. Text and hex remain."
        )
        .font(.caption)
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("value.inspector.tree.unavailable")
      }

      if let structuredRows {
        treeSection(structuredRows)
      }

      hexSection
    }
    .accessibilityIdentifier("value.inspector.details")
  }

  private func treeSection(_ rows: [StructuredValueTreeRow]) -> some View {
    VStack(alignment: .leading, spacing: 4) {
      sectionLabel("JSON TREE")
      VStack(alignment: .leading, spacing: 3) {
        ForEach(rows) { treeRow in
          HStack(alignment: .firstTextBaseline, spacing: 6) {
            Text(treeRow.label)
              .fontWeight(.medium)
            if let value = treeRow.value {
              Text(value)
                .foregroundStyle(.secondary)
                .lineLimit(4)
            }
          }
          .font(.system(.caption, design: .monospaced))
          .padding(.leading, CGFloat(treeRow.depth) * 12)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityElement(children: .combine)
          .accessibilityIdentifier("value.inspector.tree.\(treeRow.id)")
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .accessibilityIdentifier("value.inspector.tree")
  }

  private var hexSection: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack(spacing: 6) {
        Text("HEX")
          .font(.caption2.weight(.semibold))
          .tracking(0.4)
          .foregroundStyle(.secondary)
        Text("\(cell.bytes.count) B")
          .font(.caption2.monospacedDigit())
          .foregroundStyle(.secondary)
      }
      // Always-visible compact hex preserves binary inspection and fixture audits.
      Text(hexLinear.isEmpty ? "Empty" : hexLinear)
        .font(.system(.caption, design: .monospaced))
        .textSelection(.enabled)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityIdentifier("value.inspector.hex")
      if hexNeedsDisclosure {
        DisclosureGroup(isExpanded: $hexExpanded) {
          Text(hexDump)
            .font(.system(.caption, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.top, 2)
            .accessibilityIdentifier("value.inspector.hex.dump")
        } label: {
          Text("Dump")
            .font(.caption2)
        }
        .accessibilityIdentifier("value.inspector.hex.group")
      } else if !hexDump.isEmpty, hexDump.contains("\n") {
        Text(hexDump)
          .font(.system(.caption2, design: .monospaced))
          .foregroundStyle(.secondary)
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityIdentifier("value.inspector.hex.dump")
      }
    }
  }

  private func sectionLabel(_ title: String) -> some View {
    Text(title)
      .font(.caption2.weight(.semibold))
      .tracking(0.4)
      .foregroundStyle(.secondary)
  }

  private func copyToPasteboard(_ string: String) {
    let pasteboard = NSPasteboard.general
    pasteboard.clearContents()
    pasteboard.setString(string, forType: .string)
  }
}

/// Environment Halo: production / staging / development must be unmistakable
/// without relying on color alone (Increase Contrast / Reduce Transparency).
