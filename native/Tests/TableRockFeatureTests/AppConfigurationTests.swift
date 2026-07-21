import Foundation
import XCTest
@testable import TableRockFeature

final class AppConfigurationTests: XCTestCase {
    private let support = URL(fileURLWithPath: "/Users/test/Library/Application Support")
    private let temporary = URL(fileURLWithPath: "/private/tmp")

    func testProductionRoot() throws {
        let configuration = try resolve([:])
        XCTAssertFalse(configuration.isTestMode)
        XCTAssertEqual(configuration.backend, .live)
        XCTAssertEqual(
            configuration.paths.profilesDatabase.path,
            "/Users/test/Library/Application Support/TableRock/profiles.db"
        )
    }

    func testExplicitTestRoot() throws {
        let configuration = try resolve([
            "TABLEROCK_TEST_MODE": "1",
            "TABLEROCK_TEST_ROOT": "/private/tmp/TableRockUITest-123",
            "TABLEROCK_TEST_BACKEND": "scripted",
            "TABLEROCK_TEST_SCENARIO": "slow-query",
        ])
        XCTAssertTrue(configuration.isTestMode)
        XCTAssertEqual(configuration.backend, .scripted(scenario: "slow-query"))
        XCTAssertEqual(configuration.paths.dataRoot.path, "/private/tmp/TableRockUITest-123")
        XCTAssertFalse(configuration.keychainNamespace.contains("TableRockUITest"))
        XCTAssertNotEqual(configuration.keychainNamespace, try resolve([:]).keychainNamespace)
    }

    func testFixtureIsolation() throws {
        let configuration = try resolve(["TABLEROCK_FIXTURE_QUERY_TABS": "1"])
        XCTAssertTrue(configuration.isTestMode)
        XCTAssertEqual(configuration.paths.dataRoot.path, "/private/tmp/TableRockFixture-77")
        XCTAssertFalse(configuration.paths.dataRoot.path.contains("Application Support"))
    }

    func testInvalidConfiguration() {
        XCTAssertThrowsError(try resolve([
            "TABLEROCK_TEST_MODE": "1", "TABLEROCK_TEST_ROOT": "relative/path",
        ])) { error in
            XCTAssertEqual(error as? AppConfigurationError, .absoluteTestRootRequired)
        }
        XCTAssertThrowsError(try resolve([
            "TABLEROCK_TEST_MODE": "1", "TABLEROCK_TEST_ROOT": "/private/tmp/t",
            "TABLEROCK_TEST_BACKEND": "scripted",
        ])) { error in
            XCTAssertEqual(error as? AppConfigurationError, .scriptedScenarioRequired)
        }
    }

    func testPreparedRoot() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(
            "TableRockFeatureTests-\(UUID().uuidString)", isDirectory: true
        )
        defer { try? FileManager.default.removeItem(at: root) }
        let paths = AppPaths(dataRoot: root)
        try paths.prepare()
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: paths.profilesDatabase.path))
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
