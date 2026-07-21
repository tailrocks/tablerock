import Foundation
import XCTest
@testable import TableRockBridge

/// Named live-server coverage for behavior that was historically asserted by
/// the monolithic `BehaviorProof` executable. The harness selects one test at
/// a time and supplies an isolated server through environment variables.
@MainActor
final class LiveBridgeBehaviorTests: XCTestCase {
    private struct Configuration {
        let engine: String
        let host: String
        let port: UInt16
        let database: String
        let user: String
        let password: String

        init(environment: [String: String] = ProcessInfo.processInfo.environment) throws {
            guard environment["TABLEROCK_LIVE_TEST"] == "1" else {
                throw XCTSkip("live bridge server not configured")
            }
            engine = environment["TABLEROCK_ENGINE"] ?? "postgresql"
            host = environment["TABLEROCK_HOST"] ?? "127.0.0.1"
            guard let rawPort = environment["TABLEROCK_PORT"], let parsedPort = UInt16(rawPort) else {
                XCTFail("TABLEROCK_PORT must contain a UInt16")
                throw ConfigurationError.invalidPort
            }
            port = parsedPort
            database = environment["TABLEROCK_DB"] ?? "db"
            user = environment["TABLEROCK_USER"] ?? "u"
            password = environment["TABLEROCK_PASSWORD"] ?? "secret"
        }

        var openParams: OpenParams {
            OpenParams(
                engine: engine, host: host, port: port, database: database,
                user: user, password: password, tlsMode: "off"
            )
        }
    }

    private enum ConfigurationError: Error {
        case invalidPort
    }

    private func withBridge<T>(
        _ body: (TableRockBridge, Configuration, Data) async throws -> T
    ) async throws -> T {
        let configuration = try Configuration()
        let bridge = TableRockBridge.create()
        try bridge.ensureRuntime()
        defer {
            _ = try? bridge.shutdown(cancelActive: true, deadlineMs: 2_000)
            try? bridge.destroyRuntime()
        }
        let session = try bridge.open(params: configuration.openParams)
        return try await body(bridge, configuration, session)
    }

    private func page(
        from bridge: TableRockBridge,
        operationID: Data
    ) throws -> PageV1Table {
        try bridge.pump(operationId: operationID)
        let events = try bridge.nextEvents(cursor: 0, maximum: 64).events
        let bytes = try XCTUnwrap(events.last(where: {
            $0.operationId == operationID && $0.kind == "page"
        })?.pageBytes, "operation emitted no page")
        return try PageV1.decodeTable(bytes)
    }

    func testQueryReturnsTypedPageWithExpectedValue() async throws {
        try await withBridge { bridge, _, session in
            let environment = ProcessInfo.processInfo.environment
            let statement = environment["TABLEROCK_QUERY"] ?? "SELECT 1 AS n"
            let operation = try bridge.submit(spec: SubmitSpec(
                intent: "execute", sessionId: session, statement: statement,
                resultId: nil, startRow: nil, rowCount: 500, expectedRevision: 0
            ))
            let table = try page(from: bridge, operationID: operation)

            XCTAssertEqual(table.columnMetadata.count, table.columns.count)
            XCTAssertEqual(table.cells.count, table.rows.count)
            XCTAssertTrue(zip(table.columnMetadata, table.columns).allSatisfy {
                $0.0.name == $0.1
            })
            XCTAssertTrue(zip(table.cells, table.rows).allSatisfy { cells, row in
                cells.count == row.count
                    && zip(cells, row).allSatisfy { $0.0.display == $0.1 }
            })
            if let expectedColumns = environment["TABLEROCK_EXPECT_COLS"] {
                XCTAssertEqual(table.columns.joined(separator: ","), expectedColumns)
            }
            if let expectedRow = environment["TABLEROCK_EXPECT_ROW"] {
                XCTAssertEqual(table.rows.first?.joined(separator: ","), expectedRow)
            }
        }
    }

    func testCatalogReturnsTypedNodesAndBrowsableObjectPage() async throws {
        try await withBridge { bridge, configuration, session in
            var level = try bridge.refreshCatalog(sessionId: session, parentNodeId: nil)
            var total = level.count
            for depth in 0..<2 {
                let preferredName = switch (configuration.engine, depth) {
                case ("postgresql", 0), ("clickhouse", 0): configuration.database
                case ("postgresql", 1): "public"
                default: ""
                }
                guard let parent = level.first(where: {
                    $0.expandable && (preferredName.isEmpty || $0.name == preferredName)
                }) ?? level.first(where: { $0.expandable }) else { break }
                level = try bridge.refreshCatalog(
                    sessionId: session, parentNodeId: parent.idBytes
                )
                total += level.count
            }
            XCTAssertGreaterThan(total, 0)

            let browsableKinds: Set<String> = [
                "postgresql_table", "postgresql_view", "postgresql_materialized_view",
                "postgresql_foreign_table", "postgresql_partitioned_table",
                "postgresql_sequence", "clickhouse_table", "clickhouse_view",
                "clickhouse_materialized_view", "clickhouse_dictionary",
            ]
            guard configuration.engine != "redis" else { return }
            let object = try XCTUnwrap(level.first(where: { browsableKinds.contains($0.kind) }))
            let operation = try bridge.submitCatalogBrowse(
                sessionId: session, catalogNodeId: object.idBytes, rowCount: 500
            )
            _ = try page(from: bridge, operationID: operation)
        }
    }

    func testPostgreSQLCancellationReportsRuntimeAndTerminalWithinBudget() async throws {
        try await withBridge { bridge, configuration, session in
            guard configuration.engine == "postgresql" else {
                throw XCTSkip("cancellation probe is PostgreSQL-specific")
            }
            let operation = try bridge.submit(spec: SubmitSpec(
                intent: "execute", sessionId: session, statement: "SELECT pg_sleep(10)",
                resultId: nil, startRow: nil, rowCount: 500, expectedRevision: 0
            ))
            let started = ContinuousClock.now
            let pump = Task.detached { try bridge.pump(operationId: operation) }
            try await Task.sleep(for: .milliseconds(150))
            let cancellation = try bridge.cancel(operationId: operation)
            try await pump.value
            let elapsed = ContinuousClock.now - started
            let events = try bridge.nextEvents(cursor: 0, maximum: 64).events
                .filter { $0.operationId == operation }
            let terminal = events.last(where: { $0.kind == "terminal" })?.outcome

            XCTAssertNotNil(cancellation.runtime)
            XCTAssertTrue(events.contains { $0.kind == "cancel_dispatched" })
            let acceptedTerminals: Set<String> = [
                "server_confirmed_cancelled", "client_stopped", "completed_before_cancel",
            ]
            XCTAssertNotNil(terminal)
            if let terminal {
                XCTAssertTrue(acceptedTerminals.contains(terminal))
            }
            XCTAssertLessThan(elapsed, .seconds(3))
        }
    }

    func testPostgreSQLReviewTokenAppliesProbe() async throws {
        try await withBridge { bridge, configuration, session in
            guard configuration.engine == "postgresql" else {
                throw XCTSkip("review probe is PostgreSQL-specific")
            }
            let now = UInt64(Date().timeIntervalSince1970 * 1_000)
            let token = try bridge.stageProbeReview(sessionId: session, nowMs: now)
            let outcome = try bridge.applyReviewToken(
                tokenId: token, nowMs: now, sessionId: session, expectedRevision: 0
            )
            XCTAssertGreaterThan(outcome.appliedCount, 0)
            XCTAssertEqual(outcome.conflictCount, 0)
            XCTAssertEqual(outcome.failedCount, 0)
        }
    }
}
