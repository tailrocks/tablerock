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

/// A decoded page rendered as columns + display-string cells.
public struct PageV1Column: Sendable, Equatable {
    public let name: String
    public let engine: UInt8
    public let engineType: String
    public let nullable: Bool

    public init(name: String, engine: UInt8, engineType: String, nullable: Bool) {
        self.name = name
        self.engine = engine
        self.engineType = engineType
        self.nullable = nullable
    }
}

public struct PageV1Cell: Sendable, Equatable {
    public let display: String
    public let kind: UInt8
    public let truncation: UInt8
    public let originalByteCount: UInt64?
    public let bytes: Data

    public init(
        display: String, kind: UInt8, truncation: UInt8,
        originalByteCount: UInt64?, bytes: Data
    ) {
        self.display = display
        self.kind = kind
        self.truncation = truncation
        self.originalByteCount = originalByteCount
        self.bytes = bytes
    }

    public var kindLabel: String {
        switch kind {
        case 0: "NULL"
        case 1: "Boolean"
        case 2: "Signed integer"
        case 3: "Unsigned integer"
        case 4: "Float"
        case 5: "Decimal"
        case 6: "Temporal"
        case 7: "Text"
        case 8: "Structured"
        case 9: "Binary"
        case 10: "Invalid"
        case 11: "Unknown"
        default: "Kind \(kind)"
        }
    }

    public var isTruncated: Bool { truncation != 0 }
}

public struct PageV1Table: Sendable, Equatable {
    public let columns: [String]
    /// One display string per cell. `∅` for NULL; `<kind N>` for non-text kinds.
    public var rows: [[String]]
    public let columnMetadata: [PageV1Column]
    public let cells: [[PageV1Cell]]

    public init(
        columns: [String], rows: [[String]],
        columnMetadata: [PageV1Column]? = nil,
        cells: [[PageV1Cell]]? = nil
    ) {
        self.columns = columns
        self.rows = rows
        self.columnMetadata = columnMetadata ?? columns.map {
            PageV1Column(name: $0, engine: 0, engineType: "unknown", nullable: true)
        }
        self.cells = cells ?? rows.map { row in
            row.map {
                PageV1Cell(
                    display: $0, kind: 7, truncation: 0,
                    originalByteCount: nil, bytes: Data($0.utf8)
                )
            }
        }
    }
}

extension PageV1 {
    /// Decode the full page body: header + column names + cell display strings.
    /// Text cells render from the arena; non-text kinds render as a label.
    public static func decodeTable(_ data: Data) throws -> PageV1Table {
        _ = try decodeEnvelope(data)
        var pos = 0
        func need(_ n: Int) throws {
            if pos + n > data.count { throw PageV1DecodeError.truncated }
        }
        func u8() throws -> UInt8 { try need(1); let v = data[pos]; pos += 1; return v }
        func u16() throws -> UInt16 {
            try need(2)
            let v = UInt16(data[pos]) | (UInt16(data[pos + 1]) << 8)
            pos += 2; return v
        }
        func u32() throws -> UInt32 {
            try need(4)
            let v = UInt32(data[pos]) | (UInt32(data[pos + 1]) << 8)
                | (UInt32(data[pos + 2]) << 16) | (UInt32(data[pos + 3]) << 24)
            pos += 4; return v
        }
        func u64() throws -> UInt64 {
            try need(8)
            var v: UInt64 = 0
            for i in 0..<8 { v |= UInt64(data[pos + i]) << (8 * i) }
            pos += 8; return v
        }
        func bytes(_ n: Int) throws -> Data {
            try need(n); let d = data.subdata(in: pos..<(pos + n)); pos += n; return d
        }
        func boundedStr() throws -> String {
            let len = Int(try u32())
            return String(data: try bytes(len), encoding: .utf8) ?? "<utf8?>"
        }

        // Header (matches encode_v1).
        _ = try bytes(4)                 // magic
        _ = try u16()                    // encoding_version
        _ = try bytes(16)                // result_id
        _ = try u64()                    // revision
        _ = try u8()                     // engine
        _ = try u64()                    // start_row
        let rowCount = try u32()
        let columnCount = try u32()
        _ = try u8()                     // total_tag
        _ = try u64()                    // total_value
        let arenaByteLen = Int(try u64())
        _ = try u64()                    // column_text_byte_len
        _ = try u8()                     // delivery
        _ = try u16()                    // warnings (u16 bitset)

        // Columns: bounded_str(name) + u8(engine) + bounded_str(engine_name) + u8(nullable).
        var columns: [String] = []
        var columnMetadata: [PageV1Column] = []
        for _ in 0..<columnCount {
            let name = try boundedStr()
            let engine = try u8()
            let engineType = try boundedStr()
            let nullable = try u8() != 0
            columns.append(name)
            columnMetadata.append(PageV1Column(
                name: name, engine: engine, engineType: engineType, nullable: nullable
            ))
        }

        let cells = Int(rowCount) * Int(columnCount)
        var offsets: [UInt64] = []
        for _ in 0..<(cells + 1) { offsets.append(try u64()) }
        let bitmap = try bytes((cells + 7) / 8)
        var kinds: [UInt8] = []
        for _ in 0..<cells { kinds.append(try u8()) }
        // Truncations are variable-length (Complete=0, Truncated(None)=1, Truncated(Some)=2+u64).
        var truncations: [(tag: UInt8, originalByteCount: UInt64?)] = []
        for _ in 0..<cells {
            let tag = try u8()
            truncations.append((tag, tag == 2 ? try u64() : nil))
        }
        let arena = try bytes(arenaByteLen)

        var rows: [[String]] = []
        var decodedCells: [[PageV1Cell]] = []
        let cols = Int(columnCount)
        let nRows = Int(rowCount)
        for r in 0..<nRows {
            var row: [String] = []
            var decodedRow: [PageV1Cell] = []
            for c in 0..<cols {
                // Columnar layout: column c's values are contiguous, so cell
                // (r, c) is at c * rowCount + r (not row-major r * cols + c).
                let i = c * nRows + r
                let isNull = (bitmap[i / 8] & (1 << (i % 8))) != 0
                let start = Int(offsets[i])
                let end = Int(offsets[i + 1])
                let slice = (start <= end && end <= arena.count)
                    ? arena.subdata(in: start..<end) : Data()
                let display: String
                if isNull {
                    display = "∅"
                } else {
                    display = switch kinds[i] {
                case 0: "∅"
                case 1: slice.first.map { $0 != 0 ? "true" : "false" } ?? "false"
                case 2: formatSigned(slice)
                case 3: formatUnsigned(slice)
                case 4: formatFloat(slice)
                // Decimal(5), Temporal(6), Text(7), Structured(8) are all stored
                // as their UTF-8 text representation in the arena (page.rs
                // append_value: BoundedText::as_bytes).
                case 5, 6, 7, 8:
                    String(data: slice, encoding: .utf8) ?? "<text>"
                case 9:
                    // Binary — often UTF-8 (e.g., Redis keys); fall back to hex.
                    String(data: slice, encoding: .utf8)
                        ?? "0x" + slice.map { String(format: "%02x", $0) }.joined()
                case 10: "<invalid \(slice.count) bytes>"
                case 11: "<unknown \(slice.count) bytes>"
                default: "<kind \(kinds[i])>"
                    }
                }
                row.append(display)
                decodedRow.append(PageV1Cell(
                    display: display, kind: kinds[i], truncation: truncations[i].tag,
                    originalByteCount: truncations[i].originalByteCount, bytes: slice
                ))
            }
            rows.append(row)
            decodedCells.append(decodedRow)
        }
        return PageV1Table(
            columns: columns, rows: rows,
            columnMetadata: columnMetadata, cells: decodedCells
        )
    }

    /// Big-endian signed integer of the slice width (PostgreSQL network order).
    private static func formatSigned(_ slice: Data) -> String {
        guard slice.count <= 8 else { return "<int>" }
        var v: Int64 = 0
        let n = slice.count
        for (i, b) in slice.enumerated() { v |= Int64(b) << (8 * (n - 1 - i)) }
        return String(v)
    }

    private static func formatUnsigned(_ slice: Data) -> String {
        guard slice.count <= 8 else { return "<uint>" }
        var v: UInt64 = 0
        let n = slice.count
        for (i, b) in slice.enumerated() { v |= UInt64(b) << (8 * (n - 1 - i)) }
        return String(v)
    }

    private static func formatFloat(_ slice: Data) -> String {
        guard slice.count == 8 else { return "<float>" }
        var bits: UInt64 = 0
        for byte in slice { bits = (bits << 8) | UInt64(byte) }
        return String(Double(bitPattern: bits))
    }
}
