import Foundation
import TableRockBridge

let environment = ProcessInfo.processInfo.environment
let host = environment["TABLEROCK_HOST"] ?? "127.0.0.1"
let port = UInt16(environment["TABLEROCK_PORT"] ?? "5433") ?? 5433
let statement = environment["TABLEROCK_QUERY"]
  ?? "SELECT i, repeat('x', 64) AS payload FROM generate_series(1, 500) AS i"
let requestedIterations = Int(environment["TABLEROCK_DECODE_BENCH"] ?? "2000") ?? 2_000
let iterations = min(max(requestedIterations, 1), 100_000)

let bridge = TableRockBridge.create()
try bridge.ensureRuntime()
let session = try bridge.open(params: OpenParams(
  engine: "postgresql",
  host: host,
  port: port,
  database: environment["TABLEROCK_DB"] ?? "db",
  user: environment["TABLEROCK_USER"] ?? "u",
  password: environment["TABLEROCK_PASSWORD"] ?? "secret",
  tlsMode: "off"
))
let operation = try bridge.submit(spec: SubmitSpec(
  intent: "execute",
  sessionId: session,
  statement: statement,
  resultId: nil,
  startRow: nil,
  rowCount: 500,
  expectedRevision: 0
))
try bridge.pump(operationId: operation)
let events = try bridge.nextEvents(cursor: 0, maximum: 64).events
guard let encodedPage = events.last(where: {
  $0.operationId == operation && $0.kind == "page"
})?.pageBytes else {
  fatalError("benchmark operation emitted no page")
}
let table = try PageV1.decodeTable(encodedPage)
_ = try bridge.shutdown(cancelActive: false, deadlineMs: 0)

let started = ContinuousClock.now
for _ in 0..<iterations {
  _ = try PageV1.decodeTable(encodedPage)
}
let elapsed = ContinuousClock.now - started
let seconds = Double(elapsed.components.seconds)
  + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000_000
let meanMicroseconds = seconds * 1_000_000 / Double(iterations)
let metric = "PERF_PAGE_DECODE bytes=\(encodedPage.count) rows=\(table.rows.count) columns=\(table.columns.count) iterations=\(iterations) total_seconds=\(String(format: "%.6f", seconds)) mean_microseconds=\(String(format: "%.3f", meanMicroseconds))\n"
FileHandle.standardError.write(Data(metric.utf8))

if let rawHold = environment["TABLEROCK_BENCH_HOLD_SECONDS"],
  let holdSeconds = UInt64(rawHold), holdSeconds > 0
{
  try await Task.sleep(for: .seconds(min(holdSeconds, 60)))
}
