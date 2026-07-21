import Foundation
import XCTest
@testable import TableRockBridge

final class BridgeLifecycleTests: XCTestCase {
    private static let lifecycleLock = NSLock()

    override func setUp() {
        super.setUp()
        Self.lifecycleLock.lock()
    }

    override func tearDown() {
        Self.lifecycleLock.unlock()
        super.tearDown()
    }

    func testPanicContainment() throws {
        let bridge = TableRockBridge.create()
        do {
            try bridge.panicProbe()
            XCTFail("panic probe returned without a typed error")
        } catch let error as BridgeError {
            guard case .ContainedPanic = error else {
                XCTFail("expected ContainedPanic, got \(error)")
                return
            }
        }
        try bridge.ensureRuntime()
        try bridge.destroyRuntime()
    }

    func testIdempotentRuntimeLifecycle() throws {
        let bridge = TableRockBridge.create()
        try bridge.ensureRuntime()
        try bridge.ensureRuntime()
        try bridge.destroyRuntime()
        try bridge.destroyRuntime()
    }

    func testUnreachableServerIsTypedAndRedacted() throws {
        let bridge = TableRockBridge.create()
        let secret = "bridge-test-secret"
        let params = OpenParams(
            engine: "postgresql", host: "127.0.0.1", port: 1,
            database: "db", user: "u", password: secret, tlsMode: "off"
        )
        do {
            _ = try bridge.open(params: params)
            XCTFail("open unexpectedly succeeded")
        } catch let error as BridgeError {
            guard case let .Rejected(code, message) = error else {
                XCTFail("expected Rejected, got \(error)")
                return
            }
            XCTAssertEqual(code, "connect")
            XCTAssertFalse(message.contains(secret))
            XCTAssertFalse(String(describing: error).contains(secret))
        }
    }

    func testMalformedOperationId() throws {
        let bridge = TableRockBridge.create()
        do {
            _ = try bridge.cancel(operationId: Data(repeating: 0, count: 15))
            XCTFail("malformed operation ID was accepted")
        } catch let error as BridgeError {
            guard case let .Rejected(code, _) = error else {
                XCTFail("expected Rejected, got \(error)")
                return
            }
            XCTAssertEqual(code, "bad-operation-id")
        }
    }

    func testCallAfterRuntimeDestruction() throws {
        let bridge = TableRockBridge.create()
        try bridge.destroyRuntime()
        do {
            _ = try bridge.nextEvents(cursor: 0, maximum: 1)
            XCTFail("call after runtime destruction succeeded")
        } catch let error as BridgeError {
            guard case .RuntimeUnavailable = error else {
                XCTFail("expected RuntimeUnavailable, got \(error)")
                return
            }
        }
    }

    func testRepeatedCreateDestroy() throws {
        for _ in 0..<64 {
            let bridge = TableRockBridge.create()
            try bridge.ensureRuntime()
            _ = try bridge.nextEvents(cursor: 0, maximum: 1)
            try bridge.destroyRuntime()
        }
    }
}
