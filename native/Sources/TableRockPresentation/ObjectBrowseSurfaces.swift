import Foundation
import SwiftUI
import TableRockFeature

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
