import Foundation
import Testing
@testable import TableRockBridge

@Suite("PageV1 hostile boundaries")
struct PageV1BoundaryTests {
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
}

private extension FixedWidthInteger {
    var littleEndianBytes: [UInt8] {
        withUnsafeBytes(of: littleEndian, Array.init)
    }
}
