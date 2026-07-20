import Foundation

/// Presentation-only time source. Database and operation truth remains Rust-owned.
@MainActor
public protocol AppClock {
    func nowMilliseconds() -> UInt64
}

public struct SystemAppClock: AppClock {
    public init() {}

    public func nowMilliseconds() -> UInt64 {
        UInt64(Date().timeIntervalSince1970 * 1_000)
    }
}

/// Identity source for presentation-owned windows and tabs.
@MainActor
public protocol AppIdentifierGenerator {
    func next() -> UUID
}

public struct SystemAppIdentifierGenerator: AppIdentifierGenerator {
    public init() {}

    public func next() -> UUID { UUID() }
}

@MainActor
public struct AppDependencies {
    public let clock: any AppClock
    public let identifiers: any AppIdentifierGenerator

    public init(
        clock: any AppClock = SystemAppClock(),
        identifiers: any AppIdentifierGenerator = SystemAppIdentifierGenerator()
    ) {
        self.clock = clock
        self.identifiers = identifiers
    }
}
