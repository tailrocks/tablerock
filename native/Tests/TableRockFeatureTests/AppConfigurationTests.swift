import Foundation
import Testing
@testable import TableRockFeature

@Suite("Application configuration isolation")
struct AppConfigurationTests {
    private let support = URL(fileURLWithPath: "/Users/test/Library/Application Support")
    private let temporary = URL(fileURLWithPath: "/private/tmp")

    @Test("production uses TableRock Application Support")
    func productionRoot() throws {
        let configuration = try resolve([:])
        #expect(!configuration.isTestMode)
        #expect(configuration.backend == .live)
        #expect(configuration.paths.profilesDatabase.path ==
            "/Users/test/Library/Application Support/TableRock/profiles.db")
    }

    @Test("explicit test root and scripted scenario are typed")
    func explicitTestRoot() throws {
        let configuration = try resolve([
            "TABLEROCK_TEST_MODE": "1",
            "TABLEROCK_TEST_ROOT": "/private/tmp/TableRockUITest-123",
            "TABLEROCK_TEST_BACKEND": "scripted",
            "TABLEROCK_TEST_SCENARIO": "slow-query",
        ])
        #expect(configuration.isTestMode)
        #expect(configuration.backend == .scripted(scenario: "slow-query"))
        #expect(configuration.paths.dataRoot.path == "/private/tmp/TableRockUITest-123")
        #expect(configuration.keychainNamespace.contains("TableRockUITest") == false)
        #expect(configuration.keychainNamespace != try resolve([:]).keychainNamespace)
    }

    @Test("legacy fixture launches cannot touch Application Support")
    func fixtureIsolation() throws {
        let configuration = try resolve(["TABLEROCK_FIXTURE_QUERY_TABS": "1"])
        #expect(configuration.isTestMode)
        #expect(configuration.paths.dataRoot.path == "/private/tmp/TableRockFixture-77")
        #expect(!configuration.paths.dataRoot.path.contains("Application Support"))
    }

    @Test("invalid test configurations fail closed")
    func invalidConfiguration() {
        #expect(throws: AppConfigurationError.absoluteTestRootRequired) {
            _ = try resolve([
                "TABLEROCK_TEST_MODE": "1", "TABLEROCK_TEST_ROOT": "relative/path",
            ])
        }
        #expect(throws: AppConfigurationError.scriptedScenarioRequired) {
            _ = try resolve([
                "TABLEROCK_TEST_MODE": "1", "TABLEROCK_TEST_ROOT": "/private/tmp/t",
                "TABLEROCK_TEST_BACKEND": "scripted",
            ])
        }
    }

    @Test("prepare creates only the configured root")
    func preparedRoot() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(
            "TableRockFeatureTests-\(UUID().uuidString)", isDirectory: true
        )
        defer { try? FileManager.default.removeItem(at: root) }
        let paths = AppPaths(dataRoot: root)
        try paths.prepare()
        #expect(FileManager.default.fileExists(atPath: root.path))
        #expect(!FileManager.default.fileExists(atPath: paths.profilesDatabase.path))
    }

    private func resolve(_ environment: [String: String]) throws -> AppConfiguration {
        try AppConfiguration.resolve(
            environment: environment,
            applicationSupportRoot: support,
            temporaryRoot: temporary,
            processIdentifier: 77
        )
    }
}
