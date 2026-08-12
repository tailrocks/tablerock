import Foundation
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  func restoreSessionIntent(profileId: Data) async {
    guard let client else { return }
    do {
      guard
        let record = try await client.nativeWindowIntent(
          windowId: windowId.uuidString.lowercased()
        ), record.profileId == profileId
      else {
        let tab = NativeQueryTab(
          id: dependencies.identifiers.next(), title: "Query 1", statementText: ""
        )
        queryTabs = [tab]
        selectedQueryTabId = tab.id
        return
      }
      guard applySessionIntent(record.intent) else {
        profileActionError = "Restored workspace intent was invalid"
        return
      }
    } catch { profileActionError = "Restore workspace intent failed: \(error)" }
  }

  func restoreWindowIntentOnLaunch() async {
    guard let client else { return }
    do {
      guard
        let record = try await client.nativeWindowIntent(
          windowId: windowId.uuidString.lowercased()
        ), let profile = profiles.first(where: { $0.idBytes == record.profileId })
      else { return }
      guard applySessionIntent(record.intent) else {
        profileActionError = "Restored workspace intent was invalid"
        return
      }
      activeProfileId = record.profileId
      profileActionOutcome = "Restored \(profile.name) workspace; connect to resume"
    } catch { profileActionError = "Restore window intent failed: \(error)" }
  }

  @discardableResult
  private func applySessionIntent(_ intent: WorkbenchSessionIntent) -> Bool {
    let restored = intent.tabs.map {
      NativeQueryTab(
        id: dependencies.identifiers.next(),
        title: $0.title,
        statementText: $0.statementText
      )
    }
    guard !restored.isEmpty, Int(intent.selectedTab) < restored.count else { return false }
    queryTabs = restored
    selectedQueryTabId = restored[Int(intent.selectedTab)].id
    formDatabase = intent.database
    return true
  }

  func clearVolatileTabState() {
    for tab in queryTabs {
      tab.resultTable = nil
      tab.resultIdData = nil
      tab.resultRevision = 0
      tab.nextStartRow = nil
      tab.writeOutcome = nil
      tab.cancelOutcome = nil
      tab.reviewOutcome = nil
      tab.reviewError = nil
      tab.querySummary = nil
      tab.queryError = nil
      tab.activeOperationId = nil
      tab.isRunning = false
    }
    objectTabs = []
    selectedObjectTabId = nil
    selectedWorkbenchKind = "query"
    queryStateRevision &+= 1
  }
}
