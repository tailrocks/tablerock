import Foundation

/// Bounded decode of TableRock page wire version 1 (magic `TRP1`).
/// Intended to run off `MainActor` before publishing an immutable snapshot.
public struct PageV1Envelope: Sendable, Equatable {
    public let encodingVersion: UInt16
    public let resultId: Data
    public let revision: UInt64
    public let engine: UInt8
    public let startRow: UInt64
    public let rowCount: UInt32
    public let columnCount: UInt32
    public let arenaByteLen: UInt64
    public let columnTextByteLen: UInt64
    public let delivery: UInt8
    public let warnings: UInt16
}

public enum PageV1DecodeError: Error, Equatable {
    case truncated
    case invalidMagic
    case unsupportedVersion(UInt16)
    case rowLimitExceeded(actual: UInt32, limit: UInt32)
    case columnLimitExceeded(actual: UInt32, limit: UInt32)
    case arenaLimitExceeded(actual: UInt64, limit: UInt64)
    case columnTextLimitExceeded(actual: UInt64, limit: UInt64)
}

public struct PageV1Limits: Sendable {
    public var maxRows: UInt32
    public var maxColumns: UInt32
    public var maxArenaBytes: UInt64
    public var maxColumnTextBytes: UInt64

    public init(
        maxRows: UInt32 = 500,
        maxColumns: UInt32 = 64,
        maxArenaBytes: UInt64 = 4 * 1024 * 1024,
        maxColumnTextBytes: UInt64 = 64 * 1024
    ) {
        self.maxRows = maxRows
        self.maxColumns = maxColumns
        self.maxArenaBytes = maxArenaBytes
        self.maxColumnTextBytes = maxColumnTextBytes
    }
}

public enum PageV1 {
    public static let magic = Data([0x54, 0x52, 0x50, 0x31]) // TRP1
    public static let encodingVersion: UInt16 = 1

    /// Validates the fixed header against limits **before** allocating body buffers.
    public static func decodeEnvelope(_ data: Data, limits: PageV1Limits = PageV1Limits()) throws
        -> PageV1Envelope
    {
        var cursor = 0
        func need(_ n: Int) throws {
            if cursor + n > data.count { throw PageV1DecodeError.truncated }
        }
        func u8() throws -> UInt8 {
            try need(1)
            defer { cursor += 1 }
            return data[cursor]
        }
        func u16() throws -> UInt16 {
            try need(2)
            let v = data.subdata(in: cursor..<(cursor + 2)).withUnsafeBytes {
                $0.loadUnaligned(as: UInt16.self)
            }
            cursor += 2
            return UInt16(littleEndian: v)
        }
        func u32() throws -> UInt32 {
            try need(4)
            let v = data.subdata(in: cursor..<(cursor + 4)).withUnsafeBytes {
                $0.loadUnaligned(as: UInt32.self)
            }
            cursor += 4
            return UInt32(littleEndian: v)
        }
        func u64() throws -> UInt64 {
            try need(8)
            let v = data.subdata(in: cursor..<(cursor + 8)).withUnsafeBytes {
                $0.loadUnaligned(as: UInt64.self)
            }
            cursor += 8
            return UInt64(littleEndian: v)
        }
        func bytes(_ n: Int) throws -> Data {
            try need(n)
            let slice = data.subdata(in: cursor..<(cursor + n))
            cursor += n
            return slice
        }

        let magicBytes = try bytes(4)
        guard magicBytes == magic else { throw PageV1DecodeError.invalidMagic }
        let version = try u16()
        guard version == encodingVersion else {
            throw PageV1DecodeError.unsupportedVersion(version)
        }
        let resultId = try bytes(16)
        let revision = try u64()
        let engine = try u8()
        let startRow = try u64()
        let rowCount = try u32()
        let columnCount = try u32()
        _ = try u8() // total_rows tag
        _ = try u64() // total_rows value
        let arenaByteLen = try u64()
        let columnTextByteLen = try u64()
        let delivery = try u8()
        let warnings = try u16()

        if rowCount > limits.maxRows {
            throw PageV1DecodeError.rowLimitExceeded(actual: rowCount, limit: limits.maxRows)
        }
        if columnCount > limits.maxColumns {
            throw PageV1DecodeError.columnLimitExceeded(
                actual: columnCount, limit: limits.maxColumns)
        }
        if arenaByteLen > limits.maxArenaBytes {
            throw PageV1DecodeError.arenaLimitExceeded(
                actual: arenaByteLen, limit: limits.maxArenaBytes)
        }
        if columnTextByteLen > limits.maxColumnTextBytes {
            throw PageV1DecodeError.columnTextLimitExceeded(
                actual: columnTextByteLen, limit: limits.maxColumnTextBytes)
        }

        return PageV1Envelope(
            encodingVersion: version,
            resultId: resultId,
            revision: revision,
            engine: engine,
            startRow: startRow,
            rowCount: rowCount,
            columnCount: columnCount,
            arenaByteLen: arenaByteLen,
            columnTextByteLen: columnTextByteLen,
            delivery: delivery,
            warnings: warnings
        )
    }
}
