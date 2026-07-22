import Foundation
import XCTest
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

@MainActor
private final class RecordingFilePanels: AppFilePanelPort {
    var openRequests: [AppFilePanelRequest] = []
    var saveRequests: [AppFilePanelRequest] = []
    let selected: URL

    init(selected: URL) { self.selected = selected }

    func chooseOpenFile(_ request: AppFilePanelRequest) -> URL? {
        openRequests.append(request)
        return selected
    }

    func chooseSaveFile(_ request: AppFilePanelRequest) -> URL? {
        saveRequests.append(request)
        return selected
    }
}

@MainActor
private final class RecordingPasteboard: AppPasteboardPort {
    var writes: [[AppPasteboardRepresentation]] = []
    func write(_ representations: [AppPasteboardRepresentation]) throws {
        writes.append(representations)
    }
}

@MainActor
private final class RecordingKeychain: AppKeychainPort {
    var stored: [(Data, String)] = []
    var reads: [Data] = []
    var removals: [Data] = []
    let reference = Data([1, 2, 3])
    let secret = Data("secret".utf8)

    func store(secret: Data, account: String) throws -> Data {
        stored.append((secret, account))
        return reference
    }
    func read(reference: Data) throws -> Data {
        reads.append(reference)
        return secret
    }
    func remove(reference: Data) throws { removals.append(reference) }
}

@MainActor
final class AppDependenciesTests: XCTestCase {
    func testDeterministicPorts() {
        let first = UUID(uuidString: "00000000-0000-0000-0000-000000000001")!
        let second = UUID(uuidString: "00000000-0000-0000-0000-000000000002")!
        let dependencies = AppDependencies(
            clock: FixedClock(value: 42),
            identifiers: SequenceIdentifiers([first, second])
        )

        XCTAssertEqual(dependencies.clock.nowMilliseconds(), 42)
        XCTAssertEqual(dependencies.identifiers.next(), first)
        XCTAssertEqual(dependencies.identifiers.next(), second)
    }

    func testIsolatedPlatformPorts() throws {
        let url = URL(fileURLWithPath: "/private/tmp/result.csv")
        let panels = RecordingFilePanels(selected: url)
        let pasteboard = RecordingPasteboard()
        let dependencies = AppDependencies(filePanels: panels, pasteboard: pasteboard)
        let request = AppFilePanelRequest(
            title: "Export", prompt: "Save", suggestedFilename: "result.csv",
            allowedExtensions: ["csv"]
        )
        let payload = AppPasteboardRepresentation(type: "public.utf8-plain-text", value: "x")

        XCTAssertEqual(dependencies.filePanels.chooseSaveFile(request), url)
        try dependencies.pasteboard.write([payload])
        XCTAssertEqual(panels.saveRequests, [request])
        XCTAssertEqual(pasteboard.writes, [[payload]])
    }

    func testIsolatedKeychainPort() throws {
        let keychain = RecordingKeychain()
        let dependencies = AppDependencies(keychain: keychain)
        let secret = Data("secret".utf8)

        let reference = try dependencies.keychain.store(secret: secret, account: "profile-1")
        XCTAssertEqual(reference, Data([1, 2, 3]))
        XCTAssertEqual(try dependencies.keychain.read(reference: reference), secret)
        try dependencies.keychain.remove(reference: reference)
        XCTAssertEqual(keychain.stored.count, 1)
        XCTAssertEqual(keychain.stored[0].1, "profile-1")
        XCTAssertEqual(keychain.reads, [reference])
        XCTAssertEqual(keychain.removals, [reference])
    }

    func testIsolatedVimPreferenceDefaultsOffAndPersistsInPort() {
        let preferences = MemoryAppPreferencesPort()
        let dependencies = AppDependencies(preferences: preferences)

        XCTAssertFalse(dependencies.preferences.vimModeEnabled())
        dependencies.preferences.setVimModeEnabled(true)
        XCTAssertTrue(dependencies.preferences.vimModeEnabled())

        let independent = MemoryAppPreferencesPort()
        XCTAssertFalse(independent.vimModeEnabled())
    }
}
