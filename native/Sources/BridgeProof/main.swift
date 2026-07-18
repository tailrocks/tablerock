import Foundation
import TableRockBridge

@main
struct BridgeProof {
    static func main() {
        var failures = 0

        failures += run("panic_probe_contained") {
            let bridge = TableRockBridge.create()
            do {
                try bridge.panicProbe()
                throw ProofError.message("panic_probe returned without error")
            } catch let error as BridgeError {
                if case .ContainedPanic = error {
                    // ok
                } else {
                    throw ProofError.message("expected ContainedPanic, got \(error)")
                }
            }
            try bridge.ensureRuntime()
            try bridge.destroyRuntime()
            try bridge.destroyRuntime()
        }

        failures += run("open_params_rejects_unreachable_host") {
            let bridge = TableRockBridge.create()
            // Port 1 is almost never a PostgreSQL listener; expect typed reject.
            let params = OpenParams(
                engine: "postgresql",
                host: "127.0.0.1",
                port: 1,
                database: "db",
                user: "u",
                password: "secret"
            )
            do {
                _ = try bridge.open(params: params)
                throw ProofError.message("open should reject unreachable host")
            } catch let error as BridgeError {
                if case let .Rejected(code, _) = error {
                    guard code == "connect" else {
                        throw ProofError.message("unexpected reject code \(code)")
                    }
                } else {
                    throw ProofError.message("expected Rejected, got \(error)")
                }
            }
        }

        failures += run("runtime_ensure_destroy_idempotent") {
            let bridge = TableRockBridge.create()
            try bridge.ensureRuntime()
            try bridge.ensureRuntime()
            try bridge.destroyRuntime()
            try bridge.destroyRuntime()
        }

        failures += run("page_v1_decode_rejects_bad_magic_before_body") {
            do {
                _ = try PageV1.decodeEnvelope(Data([0x00, 0x01, 0x02, 0x03]))
                throw ProofError.message("expected invalidMagic")
            } catch PageV1DecodeError.invalidMagic {
                // ok
            } catch {
                throw ProofError.message("unexpected \(error)")
            }
        }

        failures += run("page_v1_decode_rejects_oversized_arena_before_alloc") {
            // Minimal valid-looking header with huge arena length.
            var bytes = Data()
            bytes.append(contentsOf: [0x54, 0x52, 0x50, 0x31]) // TRP1
            bytes.append(contentsOf: UInt16(1).littleEndianBytes)
            bytes.append(contentsOf: Data(repeating: 1, count: 16)) // result id
            bytes.append(contentsOf: UInt64(0).littleEndianBytes) // revision
            bytes.append(0) // engine
            bytes.append(contentsOf: UInt64(0).littleEndianBytes) // start_row
            bytes.append(contentsOf: UInt32(1).littleEndianBytes) // row_count
            bytes.append(contentsOf: UInt32(1).littleEndianBytes) // column_count
            bytes.append(0) // total tag unknown
            bytes.append(contentsOf: UInt64(0).littleEndianBytes)
            bytes.append(contentsOf: UInt64(9_000_000).littleEndianBytes) // arena
            bytes.append(contentsOf: UInt64(0).littleEndianBytes) // column text
            bytes.append(0) // delivery
            bytes.append(contentsOf: UInt16(0).littleEndianBytes) // warnings
            do {
                _ = try PageV1.decodeEnvelope(
                    bytes, limits: PageV1Limits(maxArenaBytes: 1024))
                throw ProofError.message("expected arenaLimitExceeded")
            } catch PageV1DecodeError.arenaLimitExceeded {
                // ok — rejected before body allocation
            } catch {
                throw ProofError.message("unexpected \(error)")
            }
        }

        if failures == 0 {
            print("bridge-proof: all checks passed")
            exit(0)
        } else {
            print("bridge-proof: \(failures) check(s) failed")
            exit(1)
        }
    }

    @discardableResult
    static func run(_ name: String, body: () throws -> Void) -> Int {
        do {
            try body()
            print("ok  \(name)")
            return 0
        } catch {
            print("FAIL \(name): \(error)")
            return 1
        }
    }
}

enum ProofError: Error, CustomStringConvertible {
    case message(String)
    var description: String {
        switch self {
        case let .message(text): text
        }
    }
}

private extension FixedWidthInteger {
    var littleEndianBytes: [UInt8] {
        withUnsafeBytes(of: self.littleEndian, Array.init)
    }
}
