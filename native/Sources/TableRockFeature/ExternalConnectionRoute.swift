import Foundation

public enum ExternalConnectionRouteError: Error, Equatable, Sendable {
  case invalidRoute
  case tooLarge(actual: Int, maximum: Int)
}

/// Extracts one database URL from TableRock's custom-scheme envelope.
/// Database URL semantics remain owned by the Rust parser.
public func externalConnectionUrlPayload(
  _ externalUrl: URL,
  maximumBytes: Int = 8_192
) throws -> String {
  let byteCount = externalUrl.absoluteString.utf8.count
  guard byteCount <= maximumBytes else {
    throw ExternalConnectionRouteError.tooLarge(actual: byteCount, maximum: maximumBytes)
  }
  guard externalUrl.scheme?.lowercased() == "tablerock",
    externalUrl.host?.lowercased() == "open",
    externalUrl.path.isEmpty,
    externalUrl.user == nil,
    externalUrl.password == nil,
    externalUrl.port == nil,
    externalUrl.fragment == nil,
    let components = URLComponents(url: externalUrl, resolvingAgainstBaseURL: false),
    let items = components.queryItems,
    items.count == 1,
    items[0].name == "url",
    let input = items[0].value,
    !input.isEmpty
  else {
    throw ExternalConnectionRouteError.invalidRoute
  }
  return input
}
