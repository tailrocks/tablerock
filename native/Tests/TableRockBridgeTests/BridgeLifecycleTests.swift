import Foundation
import Testing
@testable import TableRockBridge

@Suite("UniFFI bridge lifecycle", .serialized)
struct BridgeLifecycleTests {
    @Test("panic is typed and runtime remains usable")
    func panicContainment() throws {
        let bridge = TableRockBridge.create()
        do {
            try bridge.panicProbe()
            Issue.record("panic probe returned without a typed error")
        } catch let error as BridgeError {
            guard case .ContainedPanic = error else {
                Issue.record("expected ContainedPanic, got \(error)")
                return
            }
        }
        try bridge.ensureRuntime()
        try bridge.destroyRuntime()
    }

    @Test("runtime ensure and destroy are idempotent")
    func idempotentRuntimeLifecycle() throws {
        let bridge = TableRockBridge.create()
        try bridge.ensureRuntime()
        try bridge.ensureRuntime()
        try bridge.destroyRuntime()
        try bridge.destroyRuntime()
    }

    @Test("unreachable server returns typed redacted connection rejection")
    func unreachableServerIsTypedAndRedacted() throws {
        let bridge = TableRockBridge.create()
        let secret = "bridge-test-secret"
        let params = OpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 1,
            database: "db", user: "u", password: secret, tlsMode: "off"
        )
        do {
            _ = try bridge.open(params: params)
            Issue.record("open unexpectedly succeeded")
        } catch let error as BridgeError {
            guard case let .Rejected(code, message) = error else {
                Issue.record("expected Rejected, got \(error)")
                return
            }
            #expect(code == "connect")
            #expect(!message.contains(secret))
            #expect(!String(describing: error).contains(secret))
        }
    }

    @Test("malformed operation IDs are rejected before lookup")
    func malformedOperationId() throws {
        let bridge = TableRockBridge.create()
        do {
            _ = try bridge.cancel(operationId: Data(repeating: 0, count: 15))
            Issue.record("malformed operation ID was accepted")
        } catch let error as BridgeError {
            guard case let .Rejected(code, _) = error else {
                Issue.record("expected Rejected, got \(error)")
                return
            }
            #expect(code == "bad-operation-id")
        }
    }

    @Test("calls after runtime destruction return typed unavailable")
    func callAfterRuntimeDestruction() throws {
        let bridge = TableRockBridge.create()
        try bridge.destroyRuntime()
        do {
            _ = try bridge.nextEvents(cursor: 0, maximum: 1)
            Issue.record("call after runtime destruction succeeded")
        } catch let error as BridgeError {
            guard case .RuntimeUnavailable = error else {
                Issue.record("expected RuntimeUnavailable, got \(error)")
                return
            }
        }
    }

    @Test("repeated bridge create and destroy remains usable")
    func repeatedCreateDestroy() throws {
        for _ in 0..<64 {
            let bridge = TableRockBridge.create()
            try bridge.ensureRuntime()
            _ = try bridge.nextEvents(cursor: 0, maximum: 1)
            try bridge.destroyRuntime()
        }
    }
}
