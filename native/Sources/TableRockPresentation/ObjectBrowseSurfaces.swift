import Foundation
import SwiftUI
import TableRockFeature

func redisKeyObjectView(view: WorkbenchRedisKeyView) -> some View {
  RedisKeyObjectView(view: view)
}

private struct RedisKeyObjectView: View {
  let view: WorkbenchRedisKeyView

  private var entryCount: Int {
    view.lines.count(where: { !isMetadata($0) })
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      HStack {
        Label("Redis \(view.kind)", systemImage: "key.horizontal")
          .font(.headline)
        Spacer()
        Text("\(entryCount) \(entryCount == 1 ? "entry" : "entries")")
          .font(.caption.monospacedDigit())
          .foregroundStyle(.secondary)
      }
      .padding(.horizontal, 12)
      .frame(height: 38)
      .background(Color(nsColor: .controlBackgroundColor))
      Divider()

      if entryCount == 0 {
        ContentUnavailableView(
          "Empty Redis value", systemImage: "key.horizontal",
          description: Text("This key has no visible value entries."))
      } else {
        List(Array(view.lines.enumerated()), id: \.offset) { _, line in
          Text(line)
            .font(isMetadata(line) ? .caption.monospaced() : .body.monospaced())
            .foregroundStyle(isMetadata(line) ? .secondary : .primary)
            .textSelection(.enabled)
            .accessibilityLabel(isMetadata(line) ? "Redis metadata, \(line)" : line)
        }
        .listStyle(.inset)
      }
    }
    .background(Color(nsColor: .textBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityIdentifier("redis.key.view")
  }

  private func isMetadata(_ line: String) -> Bool {
    line.hasPrefix("type: ") || line.hasPrefix("ttl: ") || line.contains("SCAN page ")
      || line.hasSuffix("SCAN: empty") || line.hasPrefix("… more")
  }
}

func objectStructureView(tab: NativeObjectTab) -> some View {
  ObjectStructureView(tab: tab)
}

private struct ObjectStructureView: View {
  let tab: NativeObjectTab

  private struct ColumnRow: Identifiable {
    let id: Int
    let column: WorkbenchRelationColumn
  }

  private var columnRows: [ColumnRow] {
    tab.structure.map { structure in
      structure.columns.enumerated().map { ColumnRow(id: $0.offset, column: $0.element) }
    } ?? []
  }

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
      HSplitView {
        Table(columnRows) {
          TableColumn("Column") { row in
            HStack(spacing: 7) {
              Image(systemName: columnSymbol(row.column))
                .foregroundStyle(row.column.primaryKey ? Color.orange : .secondary)
                .accessibilityHidden(true)
              Text(row.column.name)
                .font(.body.monospaced())
            }
          }
          TableColumn("Type") { row in
            Text(row.column.dataType)
              .font(.body.monospaced())
          }
          TableColumn("Nullable") { row in
            Text(row.column.nullable ? "YES" : "NO")
          }
          TableColumn("Default") { row in
            Text(row.column.defaultExpression ?? "—")
              .font(.body.monospaced())
          }
        }
        .tableStyle(.bordered(alternatesRowBackgrounds: true))
        .frame(minWidth: 500)
        .accessibilityLabel("\(structure.relation) columns")
        .accessibilityIdentifier("structure.columns")

        StructureDetailsList(structure: structure)
          .frame(minWidth: 240, idealWidth: 300, maxWidth: 360)
      }
      .background(Color(nsColor: .textBackgroundColor))
      .accessibilityElement(children: .contain)
      .accessibilityIdentifier("object.structure")
    } else {
      ContentUnavailableView(
        "Structure not loaded", systemImage: "list.bullet.rectangle",
        description: Text("Select Structure to load bounded database metadata.")
      )
    }
  }

  private func columnSymbol(_ column: WorkbenchRelationColumn) -> String {
    if column.primaryKey { return "key.fill" }
    if column.sortingKey { return "arrow.up.arrow.down" }
    return "textformat"
  }

}

private struct StructureDetailsList: View {
  let structure: WorkbenchRelationStructure

  var body: some View {
    List {
      Section("Indexes") {
        if structure.indexes.isEmpty {
          Text("None").foregroundStyle(.secondary)
        } else {
          ForEach(structure.indexes.indices, id: \.self) { index in
            structureRow(
              structure.indexes[index].name,
              detail: structure.indexes[index].definition,
              symbol: structure.indexes[index].kind.localizedCaseInsensitiveContains("primary")
                ? "key.fill" : "list.bullet.rectangle"
            )
          }
        }
      }
      Section("Constraints") {
        if structure.constraints.isEmpty {
          Text("None").foregroundStyle(.secondary)
        } else {
          ForEach(structure.constraints.indices, id: \.self) { index in
            structureRow(
              "\(structure.constraints[index].kind) · \(structure.constraints[index].name)",
              detail: structure.constraints[index].definition,
              symbol: "checkmark.seal"
            )
          }
        }
      }
      if !structure.facts.isEmpty {
        Section("Engine") {
          ForEach(structure.facts.indices, id: \.self) { index in
            structureRow(
              structure.facts[index].name,
              detail: structure.facts[index].value.isEmpty ? "—" : structure.facts[index].value,
              symbol: "cylinder"
            )
          }
        }
      }
    }
    .listStyle(.sidebar)
    .accessibilityLabel("Structure details")
    .accessibilityIdentifier("structure.details")
  }

  private func structureRow(_ title: String, detail: String, symbol: String) -> some View {
    HStack(alignment: .top, spacing: 8) {
      Image(systemName: symbol)
        .foregroundStyle(.secondary)
        .accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 2) {
        Text(title)
          .lineLimit(1)
        Text(detail)
          .font(.caption.monospaced())
          .foregroundStyle(.secondary)
          .lineLimit(2)
          .textSelection(.enabled)
      }
    }
    .accessibilityElement(children: .combine)
  }
}

struct NativeStructureInspector: View {
  let tab: NativeObjectTab

  private var structure: WorkbenchRelationStructure? { tab.structure }

  private var primaryColumns: [WorkbenchRelationColumn] {
    structure?.columns.filter(\.primaryKey) ?? []
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      HStack {
        Text("STRUCTURE")
          .font(.system(size: 10, weight: .bold))
          .foregroundStyle(.secondary)
        Spacer()
        Button {
          tab.selectedSection = "data"
        } label: {
          Image(systemName: "xmark")
        }
        .buttonStyle(.plain)
        .help("Close structure")
        .accessibilityLabel("Close structure")
        .accessibilityIdentifier("structure.inspector.close")
      }
      .padding(.horizontal, 12)
      .frame(height: 40)
      Divider()

      ScrollView {
        VStack(alignment: .leading, spacing: 14) {
          Label(tab.title, systemImage: "tablecells")
            .font(.headline)
          LabeledContent("Kind", value: kindLabel(tab.kind))
          if let structure {
            LabeledContent("Engine", value: engineLabel(structure.engine))
            LabeledContent("Namespace", value: structure.namespace)
            LabeledContent("Columns", value: "\(structure.columns.count)")
          }
          if let rows = tab.summary?.split(separator: "·").first {
            LabeledContent(
              "Rows", value: String(rows).trimmingCharacters(in: .whitespaces))
          }
          Divider()
          inspectorSection("PRIMARY KEY") {
            if primaryColumns.isEmpty {
              Text("None").foregroundStyle(.secondary)
            } else {
              ForEach(primaryColumns.indices, id: \.self) { index in
                Text("\(primaryColumns[index].name) · \(primaryColumns[index].dataType)")
                  .font(.caption.monospaced())
                  .textSelection(.enabled)
              }
            }
          }
          if let facts = structure?.facts, !facts.isEmpty {
            inspectorSection("ENGINE FACTS") {
              ForEach(facts.indices, id: \.self) { index in
                VStack(alignment: .leading, spacing: 2) {
                  Text(facts[index].name)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                  Text(facts[index].value.isEmpty ? "—" : facts[index].value)
                    .font(.caption.monospaced())
                    .textSelection(.enabled)
                }
              }
            }
          }
          Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
    .background(Color(nsColor: .controlBackgroundColor))
    .accessibilityElement(children: .contain)
    .accessibilityLabel("Structure inspector for \(tab.title)")
    .accessibilityIdentifier("structure.inspector")
  }

  private func inspectorSection<Content: View>(
    _ title: String,
    @ViewBuilder content: () -> Content
  ) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(title)
        .font(.caption2.weight(.bold))
        .foregroundStyle(.secondary)
      content()
    }
  }

  private func engineLabel(_ engine: String) -> String {
    switch engine.lowercased() {
    case "postgresql": "PostgreSQL"
    case "clickhouse": "ClickHouse"
    case "redis": "Redis"
    default: engine
    }
  }

  private func kindLabel(_ kind: String) -> String {
    kind.replacingOccurrences(of: "postgresql_", with: "")
      .replacingOccurrences(of: "clickhouse_", with: "")
      .replacingOccurrences(of: "_", with: " ")
      .capitalized
  }
}
