import Foundation
import Testing
@testable import TableRockBridge

@Suite("PageV1 hostile boundaries")
struct PageV1BoundaryTests {
    @Test("valid text body decodes column metadata and cell bytes")
    func validTextBody() throws {
        let table = try PageV1.decodeTable(pageBytes())
        #expect(table.columns == ["value"])
        #expect(table.columnMetadata[0].engineType == "text")
        #expect(table.rows == [["abc"]])
        #expect(table.cells[0][0].bytes == Data("abc".utf8))
    }

    @Test("bad magic fails before body decode")
    func badMagic() {
        do {
            _ = try PageV1.decodeEnvelope(Data([0x00, 0x01, 0x02, 0x03]))
            Issue.record("invalid magic was accepted")
        } catch PageV1DecodeError.invalidMagic {
        } catch {
            Issue.record("unexpected error: \(error)")
        }
    }

    @Test("oversized arena fails from the fixed header")
    func oversizedArena() {
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
            Issue.record("oversized arena was accepted")
        } catch PageV1DecodeError.arenaLimitExceeded {
        } catch {
            Issue.record("unexpected error: \(error)")
        }
    }

    @Test("unsupported version is rejected")
    func unsupportedVersion() {
        var bytes = pageBytes()
        bytes.replaceSubrange(4..<6, with: UInt16(2).littleEndianBytes)
        do {
            _ = try PageV1.decodeEnvelope(bytes)
            Issue.record("future version was accepted")
        } catch PageV1DecodeError.unsupportedVersion(2) {
        } catch {
            Issue.record("unexpected error: \(error)")
        }
    }

    @Test("row, column, and column-text limits reject from header")
    func fixedHeaderLimits() {
        let bytes = pageBytes()
        do {
            _ = try PageV1.decodeEnvelope(bytes, limits: PageV1Limits(maxRows: 0))
            Issue.record("row limit was ignored")
        } catch PageV1DecodeError.rowLimitExceeded {
        } catch { Issue.record("unexpected row error: \(error)") }
        do {
            _ = try PageV1.decodeEnvelope(bytes, limits: PageV1Limits(maxColumns: 0))
            Issue.record("column limit was ignored")
        } catch PageV1DecodeError.columnLimitExceeded {
        } catch { Issue.record("unexpected column error: \(error)") }
        do {
            _ = try PageV1.decodeEnvelope(bytes, limits: PageV1Limits(maxColumnTextBytes: 4))
            Issue.record("column-text limit was ignored")
        } catch PageV1DecodeError.columnTextLimitExceeded {
        } catch { Issue.record("unexpected text error: \(error)") }
    }

    @Test("nonzero, descending, and out-of-arena offsets are rejected")
    func invalidOffsets() {
        for offsets in [[1, 3], [2, 1], [0, 4]] as [[UInt64]] {
            do {
                _ = try PageV1.decodeTable(pageBytes(offsets: offsets))
                Issue.record("invalid offsets were accepted: \(offsets)")
            } catch PageV1DecodeError.invalidOffsets {
            } catch {
                Issue.record("unexpected error for \(offsets): \(error)")
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
