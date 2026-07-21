import XCTest

@testable import TableRock

@MainActor
final class BridgeModelScenarioTests: XCTestCase {
  func testDirtyAndRunningTabsRequireExplicitResolution() {
    let model = BridgeModel()
    model.addQueryTab()
    let tab = model.queryTabs.last!

    tab.statementText = "SELECT 2;"
    model.requestCloseQueryTab(tab)
    XCTAssertTrue(model.pendingQueryTabClose === tab)
    XCTAssertEqual(model.queryTabs.count, 2)

    model.pendingQueryTabClose = nil
    tab.isRunning = true
    model.requestCloseQueryTab(tab)
    XCTAssertNil(model.pendingQueryTabClose)
    XCTAssertEqual(model.queryTabs.count, 2)
    XCTAssertEqual(model.profileActionError, "Cancel the running query before closing its tab")
  }

  func testCorruptRestorationFailsClosed() async {
    let backend = ScriptedWorkbenchBackend(scenario: "restoration-corrupt")
    let model = BridgeModel(client: backend)

    await model.initialize()

    XCTAssertEqual(model.profileActionError, "Restored workspace intent was invalid")
    XCTAssertNil(model.profileActionOutcome)
    XCTAssertEqual(model.queryTabs.count, 1)
    XCTAssertEqual(model.queryText, "SELECT 1;")
  }

  func testScriptedFailureMatrixRejectsNamedFaults() async throws {
    let id = Data(repeating: 1, count: 16)
    let connection = ScriptedWorkbenchBackend(scenario: "connection-failure")
    let authentication = ScriptedWorkbenchBackend(scenario: "authentication-failure")
    let staleRevision = ScriptedWorkbenchBackend(scenario: "stale-result-revision")
    let staleEvent = ScriptedWorkbenchBackend(scenario: "stale-event")
    let cursor = ScriptedWorkbenchBackend(scenario: "cursor-resync")
    let columns = ScriptedWorkbenchBackend(scenario: "mismatched-next-page-columns")

    await XCTAssertThrowsErrorAsync {
      try await connection.openProfile(id: id, secretOverride: nil)
    }
    await XCTAssertThrowsErrorAsync {
      try await authentication.openProfile(id: id, secretOverride: nil)
    }
    await XCTAssertThrowsErrorAsync {
      try await staleRevision.fetchPage(resultId: id, startRow: 0, revision: 1)
    }
    await XCTAssertThrowsErrorAsync { try await staleEvent.finish(operationId: id) }
    await XCTAssertThrowsErrorAsync { try await cursor.finish(operationId: id) }
    await XCTAssertThrowsErrorAsync {
      try await columns.fetchPage(resultId: id, startRow: 0, revision: 1)
    }
  }

  func testHistoryFailureRemainsVisibleAfterSuccessfulOperation() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "history-failure-after-page")
    let projection = try await backend.finish(operationId: Data(repeating: 1, count: 16))

    XCTAssertEqual(projection.outcome, "ok")
    XCTAssertTrue(projection.historyFailed)
  }

  func testWindowsShareBackendButOwnPresentationState() {
    let backend = ScriptedWorkbenchBackend(scenario: "success")
    let first = BridgeModel(client: backend)
    let second = BridgeModel(client: backend)

    first.queryText = "SELECT first;"
    second.queryText = "SELECT second;"

    XCTAssertNotEqual(first.windowId, second.windowId)
    XCTAssertEqual(first.queryText, "SELECT first;")
    XCTAssertEqual(second.queryText, "SELECT second;")
    XCTAssertFalse(first.queryTabs[0] === second.queryTabs[0])
  }
}

@MainActor
private func XCTAssertThrowsErrorAsync<T>(
  _ expression: () async throws -> T,
  file: StaticString = #filePath,
  line: UInt = #line
) async {
  do {
    _ = try await expression()
    XCTFail("Expected error", file: file, line: line)
  } catch {}
}
