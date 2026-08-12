import Foundation
import SwiftUI
import TableRockFeature

func objectSortBar(tab: NativeObjectTab, table: WorkbenchTable) -> some View {
  ObjectSortBar(tab: tab, table: table)
}

private struct ObjectSortBar: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab
  let table: WorkbenchTable

  private var availableColumns: [String] {
    table.columns.filter { column in
      !tab.sort.contains(where: { $0.column == column })
    }
  }

  var body: some View {
    ScrollView(.horizontal) {
      HStack(spacing: 6) {
        Menu("Add sort", systemImage: "arrow.up.arrow.down") {
          ForEach(availableColumns, id: \.self) { column in
            Button(column) { Task { await model.addObjectSort(column: column) } }
          }
        }
        .disabled(tab.isRunning || availableColumns.isEmpty || tab.sort.count >= 16)
        .accessibilityIdentifier("object.sort.add")
        ForEach(tab.sort) { key in
          ControlGroup {
            Button {
              Task { await model.toggleObjectSort(column: key.column) }
            } label: {
              Label(
                key.descending ? "Descending" : "Ascending",
                systemImage: key.descending ? "arrow.down" : "arrow.up")
            }
            .accessibilityLabel(
              "\(key.column), \(key.descending ? "descending" : "ascending"); change direction")
            Button(role: .destructive) {
              Task { await model.removeObjectSort(column: key.column) }
            } label: {
              Label("Remove \(key.column) sort", systemImage: "xmark")
            }
          } label: {
            Text(key.column)
          }
          .disabled(tab.isRunning)
          .accessibilityIdentifier("object.sort.active.\(key.column)")
        }
      }
    }
    .accessibilityLabel("Object sort order")
  }
}

private struct BrowseFilterOperatorOption: Identifiable {
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

func objectFilterBar(tab: NativeObjectTab, table: WorkbenchTable) -> some View {
  ObjectFilterBar(tab: tab, table: table)
}

private struct ObjectFilterBar: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab
  let table: WorkbenchTable

  private var selectedOperator: BrowseFilterOperatorOption {
    BrowseFilterOperatorOption.all.first(where: { $0.id == tab.filterOperator })
      ?? BrowseFilterOperatorOption.all[0]
  }

  private func operatorLabel(_ name: String) -> String {
    BrowseFilterOperatorOption.all.first(where: { $0.id == name })?.label ?? name
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(spacing: 6) {
        Picker(
          "Filter column",
          selection: Binding(
            get: { tab.filterColumn },
            set: { tab.filterColumn = $0 })
        ) {
          ForEach(table.columns, id: \.self) { Text($0).tag($0) }
        }
        .frame(maxWidth: 180)
        Picker(
          "Filter operator",
          selection: Binding(
            get: { tab.filterOperator },
            set: { tab.filterOperator = $0 })
        ) {
          ForEach(BrowseFilterOperatorOption.all) { option in
            Text(option.label).tag(option.id)
          }
        }
        .frame(maxWidth: 150)
        if selectedOperator.needsValue {
          TextField(
            "Typed value",
            text: Binding(
              get: { tab.filterValue },
              set: { tab.filterValue = $0 })
          )
          .frame(minWidth: 120, maxWidth: 240)
          .accessibilityIdentifier("object.filter.value")
        }
        Button("Add filter") { Task { await model.addObjectFilter() } }
          .disabled(tab.isRunning || tab.filterColumn.isEmpty || tab.filters.count >= 32)
          .accessibilityIdentifier("object.filter.add")
        if !tab.filters.isEmpty {
          Button("Clear filters", role: .destructive) {
            Task { await model.clearObjectFilters() }
          }
          .disabled(tab.isRunning)
        }
      }
      if !tab.filters.isEmpty {
        ScrollView(.horizontal) {
          HStack(spacing: 6) {
            ForEach(tab.filters) { filter in
              ControlGroup {
                Text(
                  [filter.column, operatorLabel(filter.operatorName), filter.value]
                    .compactMap { $0 }.joined(separator: " "))
                Button(role: .destructive) {
                  Task { await model.removeObjectFilter(id: filter.id) }
                } label: {
                  Label("Remove filter", systemImage: "xmark")
                }
              }
              .disabled(tab.isRunning)
              .accessibilityIdentifier("object.filter.active")
              .accessibilityLabel(
                [filter.column, operatorLabel(filter.operatorName), filter.value]
                  .compactMap { $0 }.joined(separator: " "))
            }
          }
        }
        .accessibilityLabel("Active object filters")
      }
      HStack(alignment: .firstTextBaseline, spacing: 6) {
        TextField(
          "Raw WHERE fragment",
          text: Binding(
            get: { tab.rawWhereDraft },
            set: { tab.rawWhereDraft = $0 }), axis: .vertical
        )
        .lineLimit(1...4)
        .accessibilityIdentifier("object.raw-where.editor")
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
        }
      }
      if tab.rawWhere != nil {
        Label("Raw WHERE active", systemImage: "exclamationmark.triangle")
          .font(.caption)
          .foregroundStyle(.secondary)
          .accessibilityIdentifier("object.raw-where.active")
      }
      HStack(spacing: 6) {
        TextField(
          "Preset name",
          text: Binding(
            get: { tab.filterPresetName },
            set: { tab.filterPresetName = $0 })
        )
        .frame(maxWidth: 180)
        .accessibilityIdentifier("object.filter-preset.name")
        Button("Save preset") { Task { await model.saveObjectFilterPreset() } }
          .disabled(
            tab.isRunning
              || tab.filterPresetName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              || tab.filterPresetName.utf8.count > 64
          )
          .accessibilityIdentifier("object.filter-preset.save")
        Menu("Load preset") {
          ForEach(tab.filterPresets) { preset in
            Button(preset.name) { Task { await model.applyObjectFilterPreset(preset) } }
              .accessibilityIdentifier("object.filter-preset.load.\(preset.name)")
          }
        }
        .disabled(tab.isRunning || tab.filterPresets.isEmpty)
        .accessibilityIdentifier("object.filter-preset.load")
        if let outcome = tab.filterPresetOutcome {
          Text(outcome).font(.caption).foregroundStyle(.secondary)
            .accessibilityIdentifier("object.filter-preset.outcome")
        }
      }
      if let error = tab.filterPresetError {
        Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .task {
      if tab.filterColumn.isEmpty { tab.filterColumn = table.columns.first ?? "" }
    }
  }
}

func redisKeyObjectView(view: WorkbenchRedisKeyView) -> some View {
  RedisKeyObjectView(view: view)
}

private struct RedisKeyObjectView: View {
  let view: WorkbenchRedisKeyView

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 8) {
        Label("Redis \(view.kind)", systemImage: "key.horizontal")
          .font(.title3.bold())
        ForEach(view.lines.indices, id: \.self) { index in
          Text(view.lines[index])
            .font(.system(.body, design: .monospaced))
            .textSelection(.enabled)
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(8)
    }
  }
}

func objectStructureView(tab: NativeObjectTab) -> some View {
  ObjectStructureView(tab: tab)
}

private struct ObjectStructureView: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let tab: NativeObjectTab

  var body: some View {
    if tab.structureLoading {
      ProgressView("Loading structure…")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    } else if let error = tab.structureError {
      ContentUnavailableView(
        "Structure unavailable", systemImage: "exclamationmark.triangle",
        description: Text(error)
      )
    } else if let structure = tab.structure {
      ScrollView {
        VStack(alignment: .leading, spacing: 14) {
          HStack {
            Text("\(structure.namespace).\(structure.relation)")
              .font(.title3.bold())
              .textSelection(.enabled)
            Spacer()
            Button("Copy DDL", systemImage: "doc.on.doc") {
              model.copyStructureDdl(structure.ddl)
            }
            .disabled(structure.ddl.isEmpty)
            .accessibilityHint("Copies database-generated structure SQL")
            Button("Change Structure…", systemImage: "slider.horizontal.3") {
              model.showDdlChange()
            }
            .disabled(!model.canEditSelectedStructure)
            .accessibilityIdentifier("structure.change.open")
            Button("Table Operations…", systemImage: "wrench.and.screwdriver") {
              model.showTableOperation()
            }
            .disabled(!model.canOperateSelectedTable)
            .accessibilityIdentifier("table-operation.open")
          }
          GroupBox("Columns") {
            Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 6) {
              GridRow {
                Text("Name").bold()
                Text("Type").bold()
                Text("Nullability").bold()
                Text("Default").bold()
                Text("Keys").bold()
                Text("Comment").bold()
              }
              Divider()
              ForEach(structure.columns.indices, id: \.self) { index in
                let column = structure.columns[index]
                GridRow {
                  Text(column.name)
                  Text(column.dataType)
                  Text(column.nullable ? "NULL" : "NOT NULL")
                  Text(column.defaultExpression ?? "—")
                  Text(
                    [
                      column.primaryKey ? "PRIMARY" : nil,
                      column.sortingKey ? "SORTING" : nil,
                    ].compactMap { $0 }.joined(separator: ", "))
                  Text(column.comment ?? "—")
                }
                .textSelection(.enabled)
              }
            }
            .padding(6)
          }
          if !structure.facts.isEmpty {
            GroupBox("Engine facts") {
              Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 7) {
                ForEach(structure.facts.indices, id: \.self) { index in
                  GridRow {
                    Text(structure.facts[index].name).bold()
                    Text(
                      structure.facts[index].value.isEmpty
                        ? "—" : structure.facts[index].value
                    )
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                  }
                }
              }
              .padding(6)
            }
          }
          structureSection(
            "Indexes",
            rows: structure.indexes.map {
              ("\($0.kind) · \($0.name)", $0.definition)
            }
          )
          structureSection(
            "Constraints",
            rows: structure.constraints.map {
              ("\($0.kind) · \($0.name)", $0.definition)
            }
          )
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(4)
      }
    } else {
      ContentUnavailableView(
        "Structure not loaded", systemImage: "list.bullet.rectangle",
        description: Text("Select Structure to load bounded database metadata.")
      )
    }
  }

  private func structureSection(_ title: String, rows: [(String, String)]) -> some View {
    GroupBox(title) {
      if rows.isEmpty {
        Text("None").foregroundStyle(.secondary).padding(6)
      } else {
        VStack(alignment: .leading, spacing: 8) {
          ForEach(rows.indices, id: \.self) { index in
            Text(rows[index].0).bold()
            Text(rows[index].1)
              .font(.system(.caption, design: .monospaced))
              .textSelection(.enabled)
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(6)
      }
    }
  }
}

