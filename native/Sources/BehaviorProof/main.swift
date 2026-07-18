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

let host = ProcessInfo.processInfo.environment["TABLEROCK_HOST"] ?? "127.0.0.1"
let port = UInt16(ProcessInfo.processInfo.environment["TABLEROCK_PORT"] ?? "5433") ?? 5433
let engine = ProcessInfo.processInfo.environment["TABLEROCK_ENGINE"] ?? "postgresql"
let statement = ProcessInfo.processInfo.environment["TABLEROCK_QUERY"] ?? "SELECT 1 AS n"
let database = ProcessInfo.processInfo.environment["TABLEROCK_DB"] ?? "db"
let user = ProcessInfo.processInfo.environment["TABLEROCK_USER"] ?? "u"
let password = ProcessInfo.processInfo.environment["TABLEROCK_PASSWORD"] ?? "secret"
let catalogMode = ProcessInfo.processInfo.environment["TABLEROCK_CATALOG"] != nil
let cancelMode = ProcessInfo.processInfo.environment["TABLEROCK_CANCEL"] != nil

let session = try bridge.open(params: OpenParams(
    engine: engine,
    host: host,
    port: port,
    database: database,
    user: user,
    password: password
))
print("opened \(engine) session against \(host):\(port)")

if catalogMode {
    var level = try bridge.refreshCatalog(sessionId: session, parentNodeId: nil)
    var total = level.count
    for _ in 0..<2 {
        guard let parent = level.first(where: \.expandable) else { break }
        level = try bridge.refreshCatalog(
            sessionId: session,
            parentNodeId: parent.idBytes
        )
        total += level.count
    }
    _ = try bridge.shutdown(cancelActive: false, deadlineMs: 0)
    guard total > 0 else {
        FileHandle.standardError.write("FAIL: typed catalog returned no nodes\n".data(using: .utf8)!)
        exit(1)
    }
    print("CATALOG PROOF PASSED: \(engine) typed nodes=\(total)")
    exit(0)
}

let spec = SubmitSpec(
    intent: "execute",
    sessionId: session,
    statement: cancelMode ? "SELECT pg_sleep(10)" : statement,
    resultId: nil,
    startRow: nil,
    rowCount: 500,
    expectedRevision: 0
)
let opId = try bridge.submit(spec: spec)

if cancelMode {
    let started = Date()
    let pump = Task.detached { try bridge.pump(operationId: opId) }
    try await Task.sleep(for: .milliseconds(150))
    let cancellation = try bridge.cancel(operationId: opId)
    try await pump.value
    let elapsed = Date().timeIntervalSince(started)
    let batch = try bridge.nextEvents(cursor: 0, maximum: 64)
    let operationEvents = batch.events.filter { $0.operationId == opId }
    let terminal = operationEvents.last { $0.kind == "terminal" }?.outcome
    let dispatched = operationEvents.contains { $0.kind == "cancel_dispatched" }
    _ = try bridge.shutdown(cancelActive: false, deadlineMs: 0)
    guard cancellation.runtime != nil else {
        FileHandle.standardError.write("FAIL: cancel had no runtime dispatch\n".data(using: .utf8)!)
        exit(1)
    }
    guard dispatched else {
        FileHandle.standardError.write("FAIL: no cancel_dispatched event\n".data(using: .utf8)!)
        exit(1)
    }
    guard terminal == "server_confirmed_cancelled" || terminal == "client_stopped"
            || terminal == "completed_before_cancel"
    else {
        FileHandle.standardError.write("FAIL: cancel terminal \(terminal ?? "nil")\n".data(using: .utf8)!)
        exit(1)
    }
    guard elapsed < 3.0 else {
        FileHandle.standardError.write("FAIL: cancel took \(elapsed)s\n".data(using: .utf8)!)
        exit(1)
    }
    print("CANCEL PROOF PASSED: core=\(cancellation.core) runtime=\(cancellation.runtime ?? "nil") terminal=\(terminal ?? "nil") elapsed=\(String(format: "%.3f", elapsed))s")
    exit(0)
}

try bridge.pump(operationId: opId)

var decoded: PageV1Table?
var encodedPage: Data?
var cursor: UInt64 = 0
for _ in 0..<64 {
    let batch = try bridge.nextEvents(cursor: cursor, maximum: 64)
    if batch.events.isEmpty { break }
    for event in batch.events {
        if event.kind == "page", let page = event.pageBytes {
            encodedPage = page
            decoded = try PageV1.decodeTable(page)
        }
        if event.kind == "terminal" { break }
    }
    cursor = batch.nextCursor
}
_ = try bridge.shutdown(cancelActive: false, deadlineMs: 0)

if let rawIterations = ProcessInfo.processInfo.environment["TABLEROCK_DECODE_BENCH"],
   let requestedIterations = Int(rawIterations), requestedIterations > 0
{
    guard let page = encodedPage, let table = decoded else {
        FileHandle.standardError.write("FAIL: decode benchmark has no page\n".data(using: .utf8)!)
        exit(1)
    }
    let iterations = min(requestedIterations, 100_000)
    let started = ContinuousClock.now
    for _ in 0..<iterations {
        _ = try PageV1.decodeTable(page)
    }
    let elapsed = ContinuousClock.now - started
    let seconds = Double(elapsed.components.seconds)
        + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000_000
    let meanMicroseconds = seconds * 1_000_000 / Double(iterations)
    let metric = "PERF_PAGE_DECODE bytes=\(page.count) rows=\(table.rows.count) columns=\(table.columns.count) iterations=\(iterations) total_seconds=\(String(format: "%.6f", seconds)) mean_microseconds=\(String(format: "%.3f", meanMicroseconds))\n"
    FileHandle.standardError.write(Data(metric.utf8))
    if let rawHold = ProcessInfo.processInfo.environment["TABLEROCK_BENCH_HOLD_SECONDS"],
       let holdSeconds = UInt64(rawHold), holdSeconds > 0
    {
        try await Task.sleep(for: .seconds(min(holdSeconds, 60)))
    }
    exit(0)
}

// Edit/review flow test (if requested).
if ProcessInfo.processInfo.environment["TABLEROCK_REVIEW"] != nil {
    let bridge2 = TableRockBridge.create()
    try bridge2.ensureRuntime()
    let session2 = try bridge2.open(params: OpenParams(
        engine: engine, host: host, port: port, database: database, user: user, password: password))
    let now = UInt64(Date().timeIntervalSince1970 * 1000)
    let token = try bridge2.stageProbeReview(sessionId: session2, nowMs: now)
    // applyReviewToken does the authorize internally (consume-once); calling
    // authorizeReviewToken separately would consume the token first.
    let outcome = try bridge2.applyReviewToken(tokenId: token, nowMs: now, sessionId: session2, expectedRevision: 0)
    print("review: \(outcome.transaction) applied=\(outcome.appliedCount) conflict=\(outcome.conflictCount) failed=\(outcome.failedCount)")
    guard outcome.appliedCount > 0 else {
        FileHandle.standardError.write("FAIL: review applied 0\n".data(using: .utf8)!)
        exit(1)
    }
    print("REVIEW PROOF PASSED: stage → authorize → apply succeeded")
    exit(0)
}

guard let table = decoded else {
    FileHandle.standardError.write("FAIL: no page event decoded\n".data(using: .utf8)!)
    exit(1)
}
print("columns: \(table.columns)")
print("rows: \(table.rows)")
let env = ProcessInfo.processInfo.environment
if let expectCols = env["TABLEROCK_EXPECT_COLS"] {
    let actual = table.columns.joined(separator: ",")
    guard actual == expectCols else {
        FileHandle.standardError.write("FAIL columns: \(actual) != \(expectCols)\n".data(using: .utf8)!)
        exit(1)
    }
    if let expectRow = env["TABLEROCK_EXPECT_ROW"] {
        guard table.rows.first.map({ $0.joined(separator: ",") }) == expectRow else {
            FileHandle.standardError.write("FAIL rows: \(table.rows)\n".data(using: .utf8)!)
            exit(1)
        }
    }
}
print("BEHAVIOR PROOF PASSED: \(engine) \(statement) -> \(table.columns.count) col(s), \(table.rows.count) row(s) decoded")
