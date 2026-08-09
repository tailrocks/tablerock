import Foundation
import Testing

@testable import TableRockFeature

@Test func valueInspectorLocationFactIsOneBased() {
  #expect(ValueInspectorProjection.locationFact(row: 0, columnIndex: 0) == "R1 C1")
  #expect(ValueInspectorProjection.locationFact(row: 2, columnIndex: 4) == "R3 C5")
}

@Test func valueInspectorHexLinearSpaceSeparated() {
  let data = Data([0x7b, 0x22, 0x6f, 0x6b, 0x22, 0x3a, 0x74, 0x72, 0x75, 0x65, 0x7d])
  #expect(ValueInspectorProjection.hexLinear(data) == "7b 22 6f 6b 22 3a 74 72 75 65 7d")
}

@Test func valueInspectorHexDumpEmptyAndAsciiGutter() {
  #expect(ValueInspectorProjection.hexDump(Data()) == "")
  let hello = Data("Hello".utf8)
  let dump = ValueInspectorProjection.hexDump(hello)
  #expect(dump.hasPrefix("0000  "))
  #expect(dump.contains("48 65 6c 6c 6f"))
  #expect(dump.contains("|Hello|"))
}

@Test func valueInspectorHexDumpCapsLargePayload() {
  let big = Data(repeating: 0x61, count: 5_000)
  let dump = ValueInspectorProjection.hexDump(big, maxBytes: 64)
  #expect(dump.contains("more bytes"))
  #expect(dump.contains("0000  "))
}

@Test func valueInspectorMetadataFactIncludesTruncation() {
  let fact = ValueInspectorProjection.metadataFact(
    engineType: "jsonb",
    nullable: true,
    byteCount: 11,
    originalByteCount: 128,
    isTruncated: true)
  #expect(fact == "jsonb · nullable · 11 B · truncated from 128 B")
}

@Test func valueInspectorKindGlyphNeverColorAlone() {
  let null = GridCellPresentation.project(
    WorkbenchCell(display: "", kind: 0, truncation: 0, originalByteCount: nil, bytes: Data()))
  #expect(null.kindGlyph == "∅")
  let structured = GridCellPresentation.project(
    WorkbenchCell(
      display: #"{"ok":true}"#, kind: 8, truncation: 0, originalByteCount: nil,
      bytes: Data(#"{"ok":true}"#.utf8)))
  #expect(structured.kindGlyph == "{}")
  let binary = GridCellPresentation.project(
    WorkbenchCell(
      display: "⟨b 3⟩", kind: 9, truncation: 0, originalByteCount: nil,
      bytes: Data([1, 2, 3])))
  #expect(binary.kindGlyph == "⟨b⟩")
}
