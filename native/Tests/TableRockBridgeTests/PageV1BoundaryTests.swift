import Foundation
import XCTest
@testable import TableRockBridge

final class PageV1BoundaryTests: XCTestCase {
    func testValidTextBody() throws {
        let table = try PageV1.decodeTable(pageBytes())
        XCTAssertEqual(table.columns, ["value"])
        XCTAssertEqual(table.columnMetadata[0].engineType, "text")
        XCTAssertEqual(table.rows, [["abc"]])
        XCTAssertEqual(table.cells[0][0].bytes, Data("abc".utf8))
    }

    func testBadMagic() {
        do {
            _ = try PageV1.decodeEnvelope(Data([0x00, 0x01, 0x02, 0x03]))
            XCTFail("invalid magic was accepted")
        } catch PageV1DecodeError.invalidMagic {
        } catch {
            XCTFail("unexpected error: \(error)")
        }
    }

    func testOversizedArena() {
        var bytes = Data()
        bytes.append(contentsOf: [0x54, 0x52, 0x50, 0x31])
        bytes.append(contentsOf: UInt16(1).littleEndianBytes)
        bytes.append(contentsOf: Data(repeating: 1, count: 16))
        bytes.append(contentsOf: UInt64(0).littleEndianBytes)
        bytes.append(0)
        bytes.append(contentsOf: UInt64(0).littleEndianBytes)
        bytes.append(contentsOf: UInt32(1).littleEndianBytes)
        bytes.append(contentsOf: UInt32(1).littleEndianBytes)
        bytes.append(0)
        bytes.append(contentsOf: UInt64(0).littleEndianBytes)
        bytes.append(contentsOf: UInt64(9_000_000).littleEndianBytes)
        bytes.append(contentsOf: UInt64(0).littleEndianBytes)
        bytes.append(0)
        bytes.append(contentsOf: UInt16(0).littleEndianBytes)

        do {
            _ = try PageV1.decodeEnvelope(
                bytes, limits: PageV1Limits(maxArenaBytes: 1_024)
            )
            XCTFail("oversized arena was accepted")
        } catch PageV1DecodeError.arenaLimitExceeded {
        } catch {
            XCTFail("unexpected error: \(error)")
        }
    }

    func testUnsupportedVersion() {
        var bytes = pageBytes()
        bytes.replaceSubrange(4..<6, with: UInt16(2).littleEndianBytes)
        do {
            _ = try PageV1.decodeEnvelope(bytes)
            XCTFail("future version was accepted")
        } catch PageV1DecodeError.unsupportedVersion(2) {
        } catch {
            XCTFail("unexpected error: \(error)")
        }
    }

    func testFixedHeaderLimits() {
        let bytes = pageBytes()
        do {
            _ = try PageV1.decodeEnvelope(bytes, limits: PageV1Limits(maxRows: 0))
            XCTFail("row limit was ignored")
        } catch PageV1DecodeError.rowLimitExceeded {
        } catch { XCTFail("unexpected row error: \(error)") }
        do {
            _ = try PageV1.decodeEnvelope(bytes, limits: PageV1Limits(maxColumns: 0))
            XCTFail("column limit was ignored")
        } catch PageV1DecodeError.columnLimitExceeded {
        } catch { XCTFail("unexpected column error: \(error)") }
        do {
            _ = try PageV1.decodeEnvelope(bytes, limits: PageV1Limits(maxColumnTextBytes: 4))
            XCTFail("column-text limit was ignored")
        } catch PageV1DecodeError.columnTextLimitExceeded {
        } catch { XCTFail("unexpected text error: \(error)") }
    }

    func testInvalidOffsets() {
        for offsets in [[1, 3], [2, 1], [0, 4]] as [[UInt64]] {
            do {
                _ = try PageV1.decodeTable(pageBytes(offsets: offsets))
                XCTFail("invalid offsets were accepted: \(offsets)")
            } catch PageV1DecodeError.invalidOffsets {
            } catch {
                XCTFail("unexpected error for \(offsets): \(error)")
            }
        }
    }

    private func pageBytes(offsets: [UInt64] = [0, 3]) -> Data {
        var bytes = Data()
        bytes.append(contentsOf: [0x54, 0x52, 0x50, 0x31])
        bytes.append(contentsOf: UInt16(1).littleEndianBytes)
        bytes.append(contentsOf: Data(repeating: 1, count: 16))
        bytes.append(contentsOf: UInt64(7).littleEndianBytes)
        bytes.append(0)
        bytes.append(contentsOf: UInt64(0).littleEndianBytes)
        bytes.append(contentsOf: UInt32(1).littleEndianBytes)
        bytes.append(contentsOf: UInt32(1).littleEndianBytes)
        bytes.append(0)
        bytes.append(contentsOf: UInt64(0).littleEndianBytes)
        bytes.append(contentsOf: UInt64(3).littleEndianBytes)
        bytes.append(contentsOf: UInt64(9).littleEndianBytes)
        bytes.append(0)
        bytes.append(contentsOf: UInt16(0).littleEndianBytes)
        appendBounded("value", to: &bytes)
        bytes.append(0)
        appendBounded("text", to: &bytes)
        bytes.append(0)
        for offset in offsets { bytes.append(contentsOf: offset.littleEndianBytes) }
        bytes.append(0)
        bytes.append(7)
        bytes.append(0)
        bytes.append(contentsOf: Data("abc".utf8))
        return bytes
    }

    private func appendBounded(_ text: String, to bytes: inout Data) {
        let data = Data(text.utf8)
        bytes.append(contentsOf: UInt32(data.count).littleEndianBytes)
        bytes.append(data)
    }
}

private extension FixedWidthInteger {
    var littleEndianBytes: [UInt8] {
        withUnsafeBytes(of: littleEndian, Array.init)
    }
}
