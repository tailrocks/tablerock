import Foundation
import SwiftUI
import TableRockFeature

/// One compact object-data rail. Detailed filter, raw-WHERE, preset, and
/// loaded-row controls live in native popovers so the grid keeps the work area.
func objectBrowseRail(tab: NativeObjectTab, table: WorkbenchTable) -> some View {
  ObjectBrowseRail(tab: tab, table: table)
}

private struct ObjectBrowseRail: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab
  let table: WorkbenchTable

  @State private var filterEditorPresented = false
  @State private var advancedEditorPresented = false
  @State private var quickFilterPresented = false

  private var availableColumns: [String] {
    table.columns.filter { column in
      !tab.sort.contains(where: { $0.column == column })
    }
  }

  private func operatorLabel(_ name: String) -> String {
    BrowseRailFilterOperator.all.first(where: { $0.id == name })?.label ?? name
  }

  var body: some View {
    HStack(spacing: 8) {
      Image(systemName: "line.3.horizontal.decrease")
        .foregroundStyle(.secondary)
        .accessibilityHidden(true)

      ScrollView(.horizontal, showsIndicators: false) {
        HStack(spacing: 6) {
          ForEach(tab.filters) { filter in
            ObjectFilterToken(
              text: [filter.column, operatorLabel(filter.operatorName), filter.value]
                .compactMap { $0 }.joined(separator: " "),
              disabled: tab.isRunning
            ) {
              Task { await model.removeObjectFilter(id: filter.id) }
            }
          }

          if tab.rawWhere != nil {
            ObjectFilterToken(text: "Raw WHERE", disabled: tab.isRunning) {
              Task { await model.clearObjectRawWhere() }
            }
            .accessibilityIdentifier("object.raw-where.active")
          }

          Button {
            filterEditorPresented = true
          } label: {
            Label("Add filter", systemImage: "plus")
          }
          .buttonStyle(.plain)
          .foregroundStyle(.secondary)
          .disabled(tab.isRunning || table.columns.isEmpty || tab.filters.count >= 32)
          .accessibilityIdentifier("object.filter.editor.open")
        }
        .padding(.vertical, 4)
      }

      Spacer(minLength: 0)

      Button {
        quickFilterPresented = true
      } label: {
        Image(
          systemName: model.loadedRowQuickFilter.isEmpty
            ? "magnifyingglass" : "line.3.horizontal.decrease.circle.fill")
      }
      .buttonStyle(.plain)
      .help("Filter loaded rows")
      .accessibilityLabel("Filter loaded rows")
      .accessibilityIdentifier("object.quick-filter.open")

      Button {
        advancedEditorPresented = true
      } label: {
        Image(systemName: "slider.horizontal.3")
      }
      .buttonStyle(.plain)
      .help("Raw WHERE and filter presets")
      .accessibilityLabel("Raw WHERE and filter presets")
      .accessibilityIdentifier("object.filter.more")

      NativeActionMenu(
        title: sortSummary,
        systemImage: "arrow.up.arrow.down",
        accessibilityLabel: sortSummary,
        identifier: "object.sort.add",
        isEnabled: !tab.isRunning && (!availableColumns.isEmpty || !tab.sort.isEmpty),
        entries: sortEntries
      )

      if !tab.filters.isEmpty {
        Button {
          Task { await model.clearObjectFilters() }
        } label: {
          Image(systemName: "xmark.circle")
        }
        .buttonStyle(.plain)
        .disabled(tab.isRunning)
        .help("Clear filters")
        .accessibilityLabel("Clear filters")
      }
    }
    .font(.caption)
    .controlSize(.small)
    .padding(.horizontal, 10)
    .frame(height: 38)
    .background(Color(nsColor: .controlBackgroundColor))
    .overlay(alignment: .bottom) { Divider() }
    .popover(isPresented: $filterEditorPresented, arrowEdge: .bottom) {
      ObjectFilterEditor(tab: tab, table: table, isPresented: $filterEditorPresented)
    }
    .popover(isPresented: $advancedEditorPresented, arrowEdge: .bottom) {
      ObjectAdvancedFilterEditor(tab: tab)
    }
    .popover(isPresented: $quickFilterPresented, arrowEdge: .bottom) {
      ObjectLoadedRowFilter(table: table)
    }
    .task {
      if tab.filterColumn.isEmpty { tab.filterColumn = table.columns.first ?? "" }
    }
  }

  private var sortSummary: String {
    guard let first = tab.sort.first else { return "Sort" }
    let direction = first.descending ? "descending" : "ascending"
    return tab.sort.count == 1
      ? "Sorted by \(first.column) \(direction)"
      : "Sorted by \(first.column) +\(tab.sort.count - 1)"
  }

  private var sortEntries: [NativeActionMenuEntry] {
    var entries: [NativeActionMenuEntry] = []
    if !availableColumns.isEmpty {
      entries.append(.section("Add sort"))
      entries += availableColumns.map { column in
        .command(title: column) { Task { await model.addObjectSort(column: column) } }
      }
    }
    if !tab.sort.isEmpty {
      entries.append(.section("Active sort"))
      for key in tab.sort {
        entries.append(
          .command(
            title: "\(key.column), \(key.descending ? "descending" : "ascending")",
            accessibilityLabel:
              "\(key.column), \(key.descending ? "descending" : "ascending"); change direction",
            identifier: "object.sort.active.\(key.column)"
          ) {
            Task { await model.toggleObjectSort(column: key.column) }
          })
        entries.append(
          .command(title: "Remove \(key.column)") {
            Task { await model.removeObjectSort(column: key.column) }
          })
      }
    }
    return entries
  }
}

private struct ObjectFilterToken: View {
  let text: String
  let disabled: Bool
  let remove: () -> Void

  var body: some View {
    HStack(spacing: 5) {
      Text(text).lineLimit(1)
      Button(action: remove) {
        Image(systemName: "xmark")
          .font(.system(size: 7, weight: .bold))
      }
      .buttonStyle(.plain)
      .disabled(disabled)
      .accessibilityLabel("Remove filter")
    }
    .padding(.horizontal, 7)
    .frame(height: 24)
    .background(Color.accentColor.opacity(0.1), in: .rect(cornerRadius: 6))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("object.filter.active")
    .accessibilityLabel(text)
  }
}

private struct ObjectFilterEditor: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab
  let table: WorkbenchTable
  @Binding var isPresented: Bool

  private var selectedOperator: BrowseRailFilterOperator {
    BrowseRailFilterOperator.all.first(where: { $0.id == tab.filterOperator })
      ?? BrowseRailFilterOperator.all[0]
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text("Add Filter").font(.headline)
      HStack(spacing: 8) {
        Picker(
          "Column",
          selection: Binding(get: { tab.filterColumn }, set: { tab.filterColumn = $0 })
        ) {
          ForEach(table.columns, id: \.self) { Text($0).tag($0) }
        }
        Picker(
          "Operator",
          selection: Binding(get: { tab.filterOperator }, set: { tab.filterOperator = $0 })
        ) {
          ForEach(BrowseRailFilterOperator.all) { option in
            Text(option.label).tag(option.id)
          }
        }
      }
      if selectedOperator.needsValue {
        TextField(
          "Typed value",
          text: Binding(get: { tab.filterValue }, set: { tab.filterValue = $0 })
        )
        .accessibilityIdentifier("object.filter.value")
      }
      HStack {
        Spacer()
        Button("Cancel") { isPresented = false }
        Button("Add filter") {
          Task {
            await model.addObjectFilter()
            isPresented = false
          }
        }
        .keyboardShortcut(.defaultAction)
        .disabled(tab.isRunning || tab.filterColumn.isEmpty || tab.filters.count >= 32)
        .accessibilityIdentifier("object.filter.add")
      }
    }
    .padding(14)
    .frame(width: 390)
  }
}

private struct ObjectAdvancedFilterEditor: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      Text("Advanced Filters").font(.headline)
      GroupBox("Raw WHERE") {
        VStack(alignment: .leading, spacing: 8) {
          TextField(
            "Raw WHERE fragment",
            text: Binding(get: { tab.rawWhereDraft }, set: { tab.rawWhereDraft = $0 }),
            axis: .vertical
          )
          .lineLimit(1...4)
          .accessibilityIdentifier("object.raw-where.editor")
          HStack {
            Button("Apply raw WHERE") { Task { await model.applyObjectRawWhere() } }
              .disabled(
                tab.isRunning
                  || tab.rawWhereDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                  || tab.rawWhereDraft.utf8.count > 65_536
              )
              .accessibilityIdentifier("object.raw-where.apply")
            if tab.rawWhere != nil {
              Button("Clear raw WHERE", role: .destructive) {
                Task { await model.clearObjectRawWhere() }
              }
              .disabled(tab.isRunning)
              .accessibilityIdentifier("object.raw-where.clear")
              Label("Raw WHERE active", systemImage: "exclamationmark.triangle")
                .font(.caption)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("object.raw-where.active")
            }
          }
        }
      }

      GroupBox("Presets") {
        VStack(alignment: .leading, spacing: 8) {
          HStack(spacing: 8) {
            TextField(
              "Preset name",
              text: Binding(get: { tab.filterPresetName }, set: { tab.filterPresetName = $0 })
            )
            .accessibilityIdentifier("object.filter-preset.name")
            Button("Save preset") { Task { await model.saveObjectFilterPreset() } }
              .disabled(
                tab.isRunning
                  || tab.filterPresetName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                  || tab.filterPresetName.utf8.count > 64
              )
              .accessibilityIdentifier("object.filter-preset.save")
          }
          Menu("Load preset") {
            ForEach(tab.filterPresets) { preset in
              Button(preset.name) { Task { await model.applyObjectFilterPreset(preset) } }
                .accessibilityIdentifier("object.filter-preset.load.\(preset.name)")
            }
          }
          .disabled(tab.isRunning || tab.filterPresets.isEmpty)
          .accessibilityIdentifier("object.filter-preset.load")
          if let outcome = tab.filterPresetOutcome {
            Text(outcome)
              .font(.caption)
              .foregroundStyle(.secondary)
              .accessibilityIdentifier("object.filter-preset.outcome")
          }
          if let error = tab.filterPresetError {
            Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)
          }
        }
      }
    }
    .padding(14)
    .frame(width: 460)
  }
}

private struct ObjectLoadedRowFilter: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let table: WorkbenchTable

  private var visibleRowCount: Int {
    let term = model.loadedRowQuickFilter.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !term.isEmpty else { return table.rows.count }
    return table.rows.filter { row in
      row.contains { value in
        value.range(of: term, options: [.caseInsensitive, .diacriticInsensitive]) != nil
      }
    }.count
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text("Loaded Rows").font(.headline)
      TextField(
        "Filter loaded rows",
        text: Binding(
          get: { model.loadedRowQuickFilter },
          set: { model.loadedRowQuickFilter = $0 })
      )
      .accessibilityIdentifier("results.quick-filter")
      let status = "Loaded rows only · \(visibleRowCount)/\(table.rows.count)"
      Text(status)
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .accessibilityIdentifier("results.quick-filter.status")
        .accessibilityValue(status)
    }
    .padding(14)
    .frame(width: 300)
  }
}

private struct BrowseRailFilterOperator: Identifiable {
  let id: String
  let label: String
  let needsValue: Bool

  static let all = [
    Self(id: "eq", label: "Equals", needsValue: true),
    Self(id: "ne", label: "Not equal", needsValue: true),
    Self(id: "lt", label: "Less than", needsValue: true),
    Self(id: "le", label: "At most", needsValue: true),
    Self(id: "gt", label: "Greater than", needsValue: true),
    Self(id: "ge", label: "At least", needsValue: true),
    Self(id: "like", label: "LIKE", needsValue: true),
    Self(id: "ilike", label: "ILIKE", needsValue: true),
    Self(id: "not_like", label: "NOT LIKE", needsValue: true),
    Self(id: "not_ilike", label: "NOT ILIKE", needsValue: true),
    Self(id: "is_null", label: "Is NULL", needsValue: false),
    Self(id: "is_not_null", label: "Is not NULL", needsValue: false),
  ]
}
