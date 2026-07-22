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

public struct AppFilePanelRequest: Equatable, Sendable {
    public let title: String
    public let prompt: String
    public let suggestedFilename: String?
    public let allowedExtensions: [String]

    public init(
        title: String,
        prompt: String,
        suggestedFilename: String? = nil,
        allowedExtensions: [String]
    ) {
        self.title = title
        self.prompt = prompt
        self.suggestedFilename = suggestedFilename
        self.allowedExtensions = allowedExtensions
    }
}

@MainActor
public protocol AppFilePanelPort {
    func chooseOpenFile(_ request: AppFilePanelRequest) -> URL?
    func chooseSaveFile(_ request: AppFilePanelRequest) -> URL?
}

public struct AppPasteboardRepresentation: Equatable, Sendable {
    public let type: String
    public let value: String

    public init(type: String, value: String) {
        self.type = type
        self.value = value
    }
}

@MainActor
public protocol AppPasteboardPort {
    func write(_ representations: [AppPasteboardRepresentation]) throws
}

@MainActor
public protocol AppKeychainPort {
    func store(secret: Data, account: String) throws -> Data
    func read(reference: Data) throws -> Data
    func remove(reference: Data) throws
}

@MainActor
public protocol AppPreferencesPort {
    func vimModeEnabled() -> Bool
    func setVimModeEnabled(_ enabled: Bool)
}

public enum AppCapabilityError: Error, Equatable {
    case unavailable(String)
    case rejected(String)
}

public struct UnavailableFilePanelPort: AppFilePanelPort {
    public init() {}
    public func chooseOpenFile(_ request: AppFilePanelRequest) -> URL? { nil }
    public func chooseSaveFile(_ request: AppFilePanelRequest) -> URL? { nil }
}

public struct UnavailablePasteboardPort: AppPasteboardPort {
    public init() {}
    public func write(_ representations: [AppPasteboardRepresentation]) throws {
        throw AppCapabilityError.unavailable("pasteboard")
    }
}

public struct UnavailableKeychainPort: AppKeychainPort {
    public init() {}
    public func store(secret: Data, account: String) throws -> Data {
        throw AppCapabilityError.unavailable("keychain")
    }
    public func read(reference: Data) throws -> Data {
        throw AppCapabilityError.unavailable("keychain")
    }
    public func remove(reference: Data) throws {
        throw AppCapabilityError.unavailable("keychain")
    }
}

public final class MemoryAppPreferencesPort: AppPreferencesPort {
    private var vimEnabled: Bool

    public init(vimModeEnabled: Bool = false) { self.vimEnabled = vimModeEnabled }
    public func vimModeEnabled() -> Bool { vimEnabled }
    public func setVimModeEnabled(_ enabled: Bool) { vimEnabled = enabled }
}

@MainActor
public struct AppDependencies {
    public let clock: any AppClock
    public let identifiers: any AppIdentifierGenerator
    public let filePanels: any AppFilePanelPort
    public let pasteboard: any AppPasteboardPort
    public let keychain: any AppKeychainPort
    public let preferences: any AppPreferencesPort

    public init(
        clock: any AppClock = SystemAppClock(),
        identifiers: any AppIdentifierGenerator = SystemAppIdentifierGenerator(),
        filePanels: any AppFilePanelPort = UnavailableFilePanelPort(),
        pasteboard: any AppPasteboardPort = UnavailablePasteboardPort(),
        keychain: any AppKeychainPort = UnavailableKeychainPort(),
        preferences: any AppPreferencesPort = MemoryAppPreferencesPort()
    ) {
        self.clock = clock
        self.identifiers = identifiers
        self.filePanels = filePanels
        self.pasteboard = pasteboard
        self.keychain = keychain
        self.preferences = preferences
    }
}
