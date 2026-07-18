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
