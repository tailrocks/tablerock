import Foundation
import CoreFoundation

public struct StructuredValueTreeRow: Sendable, Equatable, Identifiable {
  public let id: String
  public let depth: Int
  public let label: String
  public let value: String?

  public init(id: String, depth: Int, label: String, value: String?) {
    self.id = id
    self.depth = depth
    self.label = label
    self.value = value
  }
}

public enum StructuredValueTreeError: Error, Equatable {
  case inputLimitExceeded(actual: Int, limit: Int)
  case invalidJSON
  case depthLimitExceeded(Int)
  case nodeLimitExceeded(Int)
}

public enum StructuredValueTree {
  public static func decode(
    _ data: Data,
    maxInputBytes: Int = 64 * 1024,
    maxNodes: Int = 1_024,
    maxDepth: Int = 64
  ) throws -> [StructuredValueTreeRow] {
    guard data.count <= maxInputBytes else {
      throw StructuredValueTreeError.inputLimitExceeded(actual: data.count, limit: maxInputBytes)
    }
    guard maxNodes > 0, maxDepth >= 0,
      let root = try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
    else { throw StructuredValueTreeError.invalidJSON }

    var rows: [StructuredValueTreeRow] = []
    try append(
      root, label: "root", path: "$", depth: 0,
      maxNodes: maxNodes, maxDepth: maxDepth, rows: &rows)
    return rows
  }

  private static func append(
    _ node: Any, label: String, path: String, depth: Int,
    maxNodes: Int, maxDepth: Int, rows: inout [StructuredValueTreeRow]
  ) throws {
    guard depth <= maxDepth else {
      throw StructuredValueTreeError.depthLimitExceeded(maxDepth)
    }
    guard rows.count < maxNodes else {
      throw StructuredValueTreeError.nodeLimitExceeded(maxNodes)
    }

    switch node {
    case let object as [String: Any]:
      rows.append(.init(id: path, depth: depth, label: label, value: "Object (\(object.count))"))
      for key in object.keys.sorted() {
        guard let value = object[key] else { continue }
        try append(
          value, label: key, path: "\(path).\(escaped(key))", depth: depth + 1,
          maxNodes: maxNodes, maxDepth: maxDepth, rows: &rows)
      }
    case let array as [Any]:
      rows.append(.init(id: path, depth: depth, label: label, value: "Array (\(array.count))"))
      for (index, value) in array.enumerated() {
        try append(
          value, label: "[\(index)]", path: "\(path)[\(index)]", depth: depth + 1,
          maxNodes: maxNodes, maxDepth: maxDepth, rows: &rows)
      }
    case let value as String:
      rows.append(.init(id: path, depth: depth, label: label, value: value))
    case let value as NSNumber:
      let rendered =
        CFGetTypeID(value) == CFBooleanGetTypeID()
        ? (value.boolValue ? "true" : "false")
        : value.stringValue
      rows.append(.init(id: path, depth: depth, label: label, value: rendered))
    case is NSNull:
      rows.append(.init(id: path, depth: depth, label: label, value: "NULL"))
    default:
      throw StructuredValueTreeError.invalidJSON
    }
  }

  private static func escaped(_ key: String) -> String {
    key.replacingOccurrences(of: "~", with: "~0").replacingOccurrences(of: ".", with: "~1")
  }
}
