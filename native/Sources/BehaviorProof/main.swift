// Behavioral proof: open a real PostgreSQL, execute SELECT 1, decode the page,
// and assert the result. Verifies the full Swift chain (bridge + PageV1 decode)
// that the SwiftUI grid renders against a live database.
//
// Run: docker run -d --name pg -e POSTGRES_PASSWORD=secret -e POSTGRES_USER=u \
//        -e POSTGRES_DB=db -p 5433:5432 postgres:18.4-alpine
//      ./scripts/build-native-app.sh  # builds bridge module + dylib
//      (then compile this + run — see scripts/verify-native-behavior.sh)

import Foundation
import TableRockBridge

let bridge = TableRockBridge.create()
try bridge.ensureRuntime()

let host = ProcessInfo.processInfo.environment["TABLEROCK_PG_HOST"] ?? "127.0.0.1"
let port = UInt16(ProcessInfo.processInfo.environment["TABLEROCK_PG_PORT"] ?? "5433") ?? 5433

let session = try bridge.open(params: OpenParams(
    engine: "postgresql",
    host: host,
    port: port,
    database: "db",
    user: "u",
    password: "secret"
))
print("opened session against \(host):\(port)")

let spec = SubmitSpec(
    intent: "execute",
    sessionId: session,
    statement: "SELECT 1 AS n",
    resultId: nil,
    startRow: nil,
    rowCount: 10,
    expectedRevision: 0
)
let opId = try bridge.submit(spec: spec)
try bridge.pump(operationId: opId)

var decoded: PageV1Table?
var cursor: UInt64 = 0
for _ in 0..<64 {
    let batch = try bridge.nextEvents(cursor: cursor, maximum: 64)
    if batch.events.isEmpty { break }
    for event in batch.events {
        if event.kind == "page", let page = event.pageBytes {
            decoded = try PageV1.decodeTable(page)
        }
        if event.kind == "terminal" { break }
    }
    cursor = batch.nextCursor
}
_ = try bridge.shutdown(cancelActive: false, deadlineMs: 0)

guard let table = decoded else {
    FileHandle.standardError.write("FAIL: no page event decoded\n".data(using: .utf8)!)
    exit(1)
}
print("columns: \(table.columns)")
print("rows: \(table.rows)")
guard table.columns == ["n"] else {
    FileHandle.standardError.write("FAIL columns: \(table.columns)\n".data(using: .utf8)!)
    exit(1)
}
guard table.rows == [["1"]] else {
    FileHandle.standardError.write("FAIL rows: \(table.rows)\n".data(using: .utf8)!)
    exit(1)
}
print("BEHAVIOR PROOF PASSED: real PostgreSQL SELECT 1 -> decoded columns [n], rows [[1]]")
