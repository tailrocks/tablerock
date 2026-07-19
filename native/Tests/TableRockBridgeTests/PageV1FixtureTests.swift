import Foundation
import Testing
@testable import TableRockBridge

@Suite("Rust-generated PageV1 fixtures")
struct PageV1FixtureTests {
    struct Fixture: Sendable {
        let file: String
        let engine: UInt8
        let type: String
        let value: String
    }

    @Test(
        "all engines decode through the same versioned contract",
        arguments: [
            Fixture(file: "postgres-signed-v1", engine: 0, type: "int8", value: "-42"),
            Fixture(file: "clickhouse-signed-v1", engine: 1, type: "Int64", value: "7"),
            Fixture(file: "redis-signed-v1", engine: 2, type: "integer", value: "99"),
        ]
    )
    func crossEngineFixture(fixture: Fixture) throws {
        let data = try fixtureData(named: fixture.file)
        let envelope = try PageV1.decodeEnvelope(data)
        let table = try PageV1.decodeTable(data)

        #expect(envelope.encodingVersion == 1)
        #expect(envelope.engine == fixture.engine)
        #expect(table.columns == ["n"])
        #expect(table.columnMetadata[0].engineType == fixture.type)
        #expect(table.rows == [[fixture.value]])
    }

    private func fixtureData(named name: String) throws -> Data {
        let root = try #require(Bundle.module.resourceURL)
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
