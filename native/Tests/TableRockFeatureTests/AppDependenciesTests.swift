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

    @Test("file and pasteboard capabilities are isolated ports")
    func isolatedPlatformPorts() throws {
        let url = URL(fileURLWithPath: "/private/tmp/result.csv")
        let panels = RecordingFilePanels(selected: url)
        let pasteboard = RecordingPasteboard()
        let dependencies = AppDependencies(filePanels: panels, pasteboard: pasteboard)
        let request = AppFilePanelRequest(
            title: "Export", prompt: "Save", suggestedFilename: "result.csv",
            allowedExtensions: ["csv"]
        )
        let payload = AppPasteboardRepresentation(type: "public.utf8-plain-text", value: "x")

        #expect(dependencies.filePanels.chooseSaveFile(request) == url)
        try dependencies.pasteboard.write([payload])
        #expect(panels.saveRequests == [request])
        #expect(pasteboard.writes == [[payload]])
    }
}
