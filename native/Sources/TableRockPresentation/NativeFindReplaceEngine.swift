import Foundation

enum NativeFindReplaceError: Error, CustomStringConvertible {
  case emptyPattern
  case invalidPattern(String)
  case invalidScope
  case replacementLimit

  var description: String {
    switch self {
    case .emptyPattern: "Enter text to find"
    case .invalidPattern(let message): "Invalid regular expression: \(message)"
    case .invalidScope: "Select editor text before using selection scope"
    case .replacementLimit: "More than 10,000 matches; narrow scope before replacing"
    }
  }
}

struct NativeReplaceOutcome {
  let text: String
  let selection: NSRange
  let replacedRange: NSRange
  let delta: Int
  var count = 1
}

enum NativeFindReplaceEngine {
  private static let limit = 10_000

  static func find(
    in text: String, pattern: String, mode: String, scope: NSRange,
    selection: NSRange, previousMatch: NSRange?, backwards: Bool
  ) throws -> NSRange? {
    let matches = try boundedMatches(in: text, pattern: pattern, mode: mode, scope: scope)
    guard !matches.isEmpty else { return nil }
    if backwards {
      let boundary = previousMatch == selection ? selection.location : NSMaxRange(selection)
      return matches.last(where: { NSMaxRange($0.range) <= boundary && $0.range != previousMatch })?
        .range ?? matches.last?.range
    }
    let boundary =
      previousMatch == selection
      ? advancedBoundary(in: text, after: selection) : NSMaxRange(selection)
    return matches.first(where: { $0.range.location >= boundary && $0.range != previousMatch })?
      .range ?? matches.first?.range
  }

  static func replaceCurrent(
    in text: String, pattern: String, replacement: String, mode: String,
    scope: NSRange, selection: NSRange
  ) throws -> NativeReplaceOutcome? {
    let regex = try expression(pattern: pattern, mode: mode)
    try validateScope(scope, in: text)
    guard selection.location >= scope.location, NSMaxRange(selection) <= NSMaxRange(scope),
      let match = regex.firstMatch(in: text, range: selection), match.range == selection
    else { return nil }
    let inserted = replacementText(
      replacement, mode: mode, match: match, source: text, regex: regex)
    let mutable = NSMutableString(string: text)
    mutable.replaceCharacters(in: match.range, with: inserted)
    let insertedLength = (inserted as NSString).length
    return NativeReplaceOutcome(
      text: mutable as String,
      selection: NSRange(location: match.range.location, length: insertedLength),
      replacedRange: match.range, delta: insertedLength - match.range.length)
  }

  static func replaceAll(
    in text: String, pattern: String, replacement: String, mode: String, scope: NSRange
  ) throws -> NativeReplaceOutcome {
    let regex = try expression(pattern: pattern, mode: mode)
    let matches = try boundedMatches(regex: regex, in: text, scope: scope)
    let mutable = NSMutableString(string: text)
    var delta = 0
    for match in matches.reversed() {
      let inserted = replacementText(
        replacement, mode: mode, match: match, source: text, regex: regex)
      mutable.replaceCharacters(in: match.range, with: inserted)
      delta += (inserted as NSString).length - match.range.length
    }
    let resultingScope = NSRange(location: scope.location, length: max(0, scope.length + delta))
    return NativeReplaceOutcome(
      text: mutable as String, selection: resultingScope, replacedRange: scope,
      delta: delta, count: matches.count)
  }

  private static func boundedMatches(
    in text: String, pattern: String, mode: String, scope: NSRange
  ) throws -> [NSTextCheckingResult] {
    try boundedMatches(regex: expression(pattern: pattern, mode: mode), in: text, scope: scope)
  }

  private static func boundedMatches(
    regex: NSRegularExpression, in text: String, scope: NSRange
  ) throws -> [NSTextCheckingResult] {
    try validateScope(scope, in: text)
    var matches: [NSTextCheckingResult] = []
    regex.enumerateMatches(in: text, range: scope) { match, _, stop in
      guard let match else { return }
      matches.append(match)
      if matches.count > limit { stop.pointee = true }
    }
    guard matches.count <= limit else { throw NativeFindReplaceError.replacementLimit }
    return matches
  }

  private static func expression(pattern: String, mode: String) throws -> NSRegularExpression {
    guard !pattern.isEmpty else { throw NativeFindReplaceError.emptyPattern }
    let source: String
    let options: NSRegularExpression.Options
    switch mode {
    case "regular_expression":
      source = pattern
      options = []
    case "whole_word":
      let escaped = NSRegularExpression.escapedPattern(for: pattern)
      source = "(?<![\\p{L}\\p{N}_])\(escaped)(?![\\p{L}\\p{N}_])"
      options = [.caseInsensitive]
    case "case_sensitive":
      source = NSRegularExpression.escapedPattern(for: pattern)
      options = []
    default:
      source = NSRegularExpression.escapedPattern(for: pattern)
      options = [.caseInsensitive]
    }
    do { return try NSRegularExpression(pattern: source, options: options) } catch {
      throw NativeFindReplaceError.invalidPattern(error.localizedDescription)
    }
  }

  private static func replacementText(
    _ replacement: String, mode: String, match: NSTextCheckingResult,
    source: String, regex: NSRegularExpression
  ) -> String {
    mode == "regular_expression"
      ? regex.replacementString(for: match, in: source, offset: 0, template: replacement)
      : replacement
  }

  private static func validateScope(_ scope: NSRange, in text: String) throws {
    let length = (text as NSString).length
    guard scope.location <= length, NSMaxRange(scope) <= length else {
      throw NativeFindReplaceError.invalidScope
    }
  }

  private static func advancedBoundary(in text: String, after range: NSRange) -> Int {
    let length = (text as NSString).length
    let end = NSMaxRange(range)
    guard range.length == 0, end < length else { return end }
    return NSMaxRange((text as NSString).rangeOfComposedCharacterSequence(at: end))
  }
}
