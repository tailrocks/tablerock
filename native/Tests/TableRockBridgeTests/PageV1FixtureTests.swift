import Foundation
import XCTest
@testable import TableRockBridge

final class PageV1FixtureTests: XCTestCase {
    private struct Fixture: Sendable {
        let file: String
        let engine: UInt8
        let type: String
        let value: String
    }

    func testCrossEngineFixtures() throws {
        let fixtures = [
            Fixture(file: "postgres-signed-v1", engine: 0, type: "int8", value: "-42"),
            Fixture(file: "clickhouse-signed-v1", engine: 1, type: "Int64", value: "7"),
            Fixture(file: "redis-signed-v1", engine: 2, type: "integer", value: "99"),
        ]
        for fixture in fixtures {
            let data = try fixtureData(named: fixture.file)
            let envelope = try PageV1.decodeEnvelope(data)
            let table = try PageV1.decodeTable(data)

            XCTAssertEqual(envelope.encodingVersion, 1, fixture.file)
            XCTAssertEqual(envelope.engine, fixture.engine, fixture.file)
            XCTAssertEqual(table.columns, ["n"], fixture.file)
            XCTAssertEqual(table.columnMetadata[0].engineType, fixture.type, fixture.file)
            XCTAssertEqual(table.rows, [[fixture.value]], fixture.file)
        }
    }

    private func fixtureData(named name: String) throws -> Data {
        let root = try XCTUnwrap(Bundle.module.resourceURL)
        let url = root
            .appendingPathComponent("Fixtures/PageV1")
            .appendingPathComponent(name)
            .appendingPathExtension("hex")
        let hex = try String(contentsOf: url, encoding: .utf8)
            .filter { !$0.isWhitespace }
        guard hex.count.isMultiple(of: 2) else {
            throw FixtureError.invalidHex
        }
        var data = Data(capacity: hex.count / 2)
        var cursor = hex.startIndex
        while cursor < hex.endIndex {
            let end = hex.index(cursor, offsetBy: 2)
            guard let byte = UInt8(hex[cursor..<end], radix: 16) else {
                throw FixtureError.invalidHex
            }
            data.append(byte)
            cursor = end
        }
        return data
    }
}

private enum FixtureError: Error {
    case invalidHex
}
