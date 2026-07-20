import Foundation
import Testing
@testable import TableRockFeature

@MainActor
private struct FixedClock: AppClock {
    let value: UInt64
    func nowMilliseconds() -> UInt64 { value }
}

@MainActor
private final class SequenceIdentifiers: AppIdentifierGenerator {
    private var values: [UUID]

    init(_ values: [UUID]) { self.values = values }

    func next() -> UUID { values.removeFirst() }
}

@Suite("Application dependency injection")
@MainActor
struct AppDependenciesTests {
    @Test("clock and identifiers are deterministic ports")
    func deterministicPorts() {
        let first = UUID(uuidString: "00000000-0000-0000-0000-000000000001")!
        let second = UUID(uuidString: "00000000-0000-0000-0000-000000000002")!
        let dependencies = AppDependencies(
            clock: FixedClock(value: 42),
            identifiers: SequenceIdentifiers([first, second])
        )

        #expect(dependencies.clock.nowMilliseconds() == 42)
        #expect(dependencies.identifiers.next() == first)
        #expect(dependencies.identifiers.next() == second)
    }
}
