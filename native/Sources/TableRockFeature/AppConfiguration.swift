import Foundation

public struct AppConfiguration: Sendable, Equatable {
    public enum Backend: Sendable, Equatable {
        case live
        case scripted(scenario: String)
    }

    public let backend: Backend
    public let paths: AppPaths
    public let isTestMode: Bool

    public var keychainNamespace: String {
        let root = Data(paths.dataRoot.path.utf8).base64EncodedString()
        return "app.tablerock.credentials.\(root)"
    }

    public static func resolve(
        environment: [String: String],
        applicationSupportRoot: URL,
        temporaryRoot: URL,
        processIdentifier: Int32
    ) throws -> Self {
        let explicitTest = environment["TABLEROCK_TEST_MODE"] == "1"
        let fixtureTest = environment.keys.contains { $0.hasPrefix("TABLEROCK_FIXTURE_") }
        let isTestMode = explicitTest || fixtureTest
        let dataRoot: URL
        if explicitTest {
            guard let raw = environment["TABLEROCK_TEST_ROOT"], raw.hasPrefix("/") else {
                throw AppConfigurationError.absoluteTestRootRequired
            }
            dataRoot = URL(fileURLWithPath: raw, isDirectory: true).standardizedFileURL
        } else if fixtureTest {
            dataRoot = temporaryRoot.appendingPathComponent(
                "TableRockFixture-\(processIdentifier)", isDirectory: true
            )
        } else {
            dataRoot = applicationSupportRoot.appendingPathComponent(
                "TableRock", isDirectory: true
            )
        }

        let backend: Backend
        switch environment["TABLEROCK_TEST_BACKEND"] {
        case nil, "", "live":
            backend = .live
        case "scripted":
            guard let scenario = environment["TABLEROCK_TEST_SCENARIO"], !scenario.isEmpty else {
                throw AppConfigurationError.scriptedScenarioRequired
            }
            backend = .scripted(scenario: scenario)
        case let value?:
            throw AppConfigurationError.unsupportedBackend(value)
        }
        return Self(backend: backend, paths: AppPaths(dataRoot: dataRoot), isTestMode: isTestMode)
    }
}

public struct AppPaths: Sendable, Equatable {
    public let dataRoot: URL

    public init(dataRoot: URL) {
        self.dataRoot = dataRoot
    }

    public var profilesDatabase: URL {
        dataRoot.appendingPathComponent("profiles.db", isDirectory: false)
    }

    public func prepare(fileManager: FileManager = .default) throws {
        try fileManager.createDirectory(at: dataRoot, withIntermediateDirectories: true)
    }
}

public enum AppConfigurationError: Error, Equatable {
    case absoluteTestRootRequired
    case scriptedScenarioRequired
    case unsupportedBackend(String)
}
