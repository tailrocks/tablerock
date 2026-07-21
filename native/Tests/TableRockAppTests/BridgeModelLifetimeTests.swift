import XCTest

@testable import TableRock

@MainActor
final class BridgeModelLifetimeTests: XCTestCase {
  func testModelDeallocatesAfterActiveOperationCancellationCompletes() async throws {
    let backend = ScriptedWorkbenchBackend(scenario: "slow-until-cancelled")
    var model: BridgeModel? = BridgeModel(client: backend)
    model?.sessionData = Data(repeating: 1, count: 16)
    model?.queryText = "SELECT pg_sleep(30);"
    weak var weakModel = model

    let operation = startQuery(on: model!)
    model = nil

    let operationID = Data(repeating: 2, count: 16)
    for _ in 0..<100 where weakModel?.isRunning != true {
      try await Task.sleep(for: .milliseconds(10))
    }
    XCTAssertEqual(weakModel?.isRunning, true)
    _ = try await backend.cancel(operationId: operationID)
    await operation.value

    for _ in 0..<100 where weakModel != nil {
      await Task.yield()
    }
    XCTAssertNil(weakModel)
  }

  private func startQuery(on model: BridgeModel) -> Task<Void, Never> {
    Task { await model.runQuery() }
  }
}
