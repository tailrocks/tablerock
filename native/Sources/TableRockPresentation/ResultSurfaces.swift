import AppKit
import Foundation
import SwiftUI
import TableRockFeature

struct ResultGridWithInspector: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let table: WorkbenchTable
  let minimumHeight: CGFloat
  var exposesResultPaging = false

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

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      // One glass chrome cluster above opaque grid content.
      GlassEffectContainer {
        VStack(alignment: .leading, spacing: 4) {
          HStack(spacing: 8) {
            ResultCopyMenu()
            ResultExportMenu()
            Button {
              Task { await model.openRelationContinuumFromSelection() }
            } label: {
              Label("Continuum", systemImage: "arrow.triangle.branch")
            }
            .buttonStyle(.glassProminent)
            .controlSize(.small)
            .disabled(!model.canOpenRelationContinuum)
            .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
            .help("Row Continuum: related rows for this cell (⌘⌥→)")
            .accessibilityIdentifier("relation.continuum.open")
            if model.relationContinuumLoading {
              ProgressView()
                .controlSize(.small)
                .accessibilityLabel("Loading related rows")
            }
            if model.relationContinuum != nil {
              Button("Close Continuum") { model.closeRelationContinuum() }
                .buttonStyle(.glass)
                .controlSize(.small)
                .keyboardShortcut(.leftArrow, modifiers: [.command, .option])
                .accessibilityIdentifier("relation.continuum.close")
            }
            Spacer(minLength: 0)
          }
          HStack(spacing: 8) {
            TextField(
              "Filter loaded rows",
              text: Binding(
                get: { model.loadedRowQuickFilter },
                set: { model.loadedRowQuickFilter = $0 })
            )
            .textFieldStyle(.roundedBorder)
            .frame(minWidth: 120, maxWidth: 220)
            .accessibilityIdentifier("results.quick-filter")
            let loadedRowsStatus =
              "Loaded rows only · \(visibleRowIndices.count)/\(table.rows.count)"
            Text(loadedRowsStatus)
              .font(.caption.monospacedDigit())
              .foregroundStyle(.secondary)
              .accessibilityIdentifier("results.quick-filter.status")
              .accessibilityValue(loadedRowsStatus)
            if exposesResultPaging && model.nextStartRow != nil {
              Button("Load more") { Task { await model.loadMore() } }
                .buttonStyle(.glass)
                .controlSize(.small)
                .accessibilityIdentifier("results.next-page")
            }
            if let outcome = model.copyOutcome {
              Text(outcome)
                .font(.caption).foregroundStyle(.secondary)
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
            Spacer(minLength: 0)
          }
        }
        .controlSize(.small)
      }
      if let selection = model.selectedCellSnapshot {
        let presentation = GridCellPresentation.project(selection.1)
        Text(
          "\(selection.0.name) · \(presentation.statusFact) · R\(selection.2 + 1) C\(selection.3 + 1)"
        )
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("results.selection.status")
        .accessibilityValue(
          "\(selection.0.name), \(presentation.accessibilityValue), row \(selection.2 + 1)")
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
  }

  private var performanceAutoScroll: Bool {
    #if TABLEROCK_DEVELOPMENT_SUPPORT
      model.fixtures.performanceAutoScroll
    #else
      false
    #endif
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

private struct ResultExportMenu: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    HStack(spacing: 6) {
      exportButton("Export CSV", format: "csv")
      Menu {
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
        Label("More Export Formats", systemImage: "ellipsis.circle")
      }
      .accessibilityIdentifier("results.export.more")
    }
    .fixedSize(horizontal: true, vertical: true)
    .disabled(model.resultIdData == nil)
    .accessibilityHint("Atomically export all rows currently resident in this result")
  }

  private func exportButton(_ label: String, format: String) -> some View {
    Button(label) { Task { await model.exportLoadedResult(format: format) } }
      .buttonStyle(.glass)
      .accessibilityIdentifier("results.export.\(format)")
  }

  private func fullExportButton(_ label: String, format: String) -> some View {
    Button(label) { Task { await model.exportFullResult(format: format) } }
      .accessibilityIdentifier("results.export.full.\(format)")
  }
}

private struct ResultCopyMenu: View {
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
      Label("Copy Result", systemImage: "doc.on.doc")
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

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      header
      Divider()
      ScrollView {
        VStack(alignment: .leading, spacing: 8) {
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

          primarySurface

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
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
    .background(Color(nsColor: .textBackgroundColor))
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
    HStack(alignment: .firstTextBaseline, spacing: 8) {
      Text(presentation.kindGlyph)
        .font(.caption.weight(.bold).monospaced())
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 2) {
        Text(presentation.kindLabel.uppercased())
          .font(.caption.weight(.bold))
          .tracking(0.5)
          .accessibilityIdentifier("value.inspector.kind")
        Text(column.name)
          .font(.caption.monospaced())
          .textSelection(.enabled)
          .lineLimit(2)
          .accessibilityIdentifier("value.inspector.column")
      }
      Spacer(minLength: 4)
      VStack(alignment: .trailing, spacing: 4) {
        HStack(spacing: 6) {
          Text(ValueInspectorProjection.locationFact(row: row, columnIndex: columnIndex))
            .font(.caption2.monospacedDigit())
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("value.inspector.location")
          Button {
            model.selectedCell = nil
          } label: {
            Image(systemName: "xmark")
          }
          .buttonStyle(.plain)
          .accessibilityLabel("Close inspector")
          .accessibilityIdentifier("value.inspector.close")
        }
        HStack(spacing: 6) {
          Button("Copy Text") { copyToPasteboard(cell.display) }
            .buttonStyle(.borderless)
            .controlSize(.mini)
            .disabled(presentation.isNull && cell.display.isEmpty)
            .accessibilityIdentifier("value.inspector.copy.text")
          Button("Copy Hex") { copyToPasteboard(hexLinear) }
            .buttonStyle(.borderless)
            .controlSize(.mini)
            .disabled(cell.bytes.isEmpty)
            .accessibilityIdentifier("value.inspector.copy.hex")
        }
      }
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 8)
  }

  @ViewBuilder
  private var primarySurface: some View {
    if presentation.isNull {
      VStack(alignment: .leading, spacing: 4) {
        Text("∅")
          .font(.title2.monospaced())
        Text("NULL")
          .font(.caption.weight(.semibold))
          .foregroundStyle(.secondary)
        // Keep raw display text in the hierarchy for audit / selection.
        Text(cell.display.isEmpty ? "NULL" : cell.display)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .opacity(cell.display.isEmpty ? 0 : 1)
          .accessibilityIdentifier("value.inspector.text")
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .accessibilityElement(children: .combine)
      .accessibilityLabel("NULL value")
    } else if presentation.kindLabel == "Text" && presentation.title == "·" {
      VStack(alignment: .leading, spacing: 4) {
        Text("·")
          .font(.title2.monospaced())
        Text("Empty text")
          .font(.caption.weight(.semibold))
          .foregroundStyle(.secondary)
        Text(cell.display)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityIdentifier("value.inspector.text")
      }
    } else if prefersHexPrimary {
      VStack(alignment: .leading, spacing: 6) {
        sectionLabel("BYTES")
        Text(hexDump.isEmpty ? "Empty" : hexDump)
          .font(.system(.caption, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityIdentifier("value.inspector.hex")
        if !cell.display.isEmpty {
          sectionLabel("LABEL")
          Text(cell.display)
            .font(.system(.caption, design: .monospaced))
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityIdentifier("value.inspector.text")
        }
      }
    } else {
      VStack(alignment: .leading, spacing: 6) {
        sectionLabel("TEXT")
        Text(cell.display.isEmpty ? "Empty" : cell.display)
          .font(.system(.body, design: .monospaced))
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
          .accessibilityIdentifier("value.inspector.text")
      }
    }
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
    // Binary already surfaces dump as primary. Other kinds keep linear hex always
    // painted (audit + copy), with multi-line dump progressive when large.
    Group {
      if prefersHexPrimary {
        EmptyView()
      } else {
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
          // Always-visible compact hex — required for fixture audits and quick scan.
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
