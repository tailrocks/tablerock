#if TABLEROCK_DEVELOPMENT_SUPPORT

  import XCTest
  import TableRockFeature

  @testable import TableRockPresentation

  final class NativeWorkbenchFixtureConfigurationTests: XCTestCase {
    @MainActor
    func testEmptyQueryRunSelectsExplicitMessagesState() async {
      let configuration = NativeWorkbenchFixtureConfiguration.from(environment: [:])
      let store = WorkbenchPresentationStore(client: nil, fixtures: configuration)
      store.queryText = "  \n"

      await store.runQuery()

      XCTAssertEqual(store.queryError, "Enter SQL before running.")
      XCTAssertEqual(store.activeQueryTab.selectedResultSection, "messages")
    }

    @MainActor
    func testMixedWorkspaceOrderReconcilesLiveTabsWithoutDuplicates() {
      let configuration = NativeWorkbenchFixtureConfiguration.from(environment: [:])
      let store = WorkbenchPresentationStore(client: nil, fixtures: configuration)
      let query = store.queryTabs[0]
      let object = NativeObjectTab(
        id: UUID(),
        node: WorkbenchCatalogNode(
          idBytes: Data(repeating: 1, count: 16), parentIdBytes: nil,
          depth: 0, name: "customers", kind: "postgresql_table",
          childrenState: "not_applicable", expandable: false))
      let trailingQuery = NativeQueryTab(id: UUID(), title: "Trailing", statementText: "")
      store.objectTabs = [object]
      store.queryTabs.append(trailingQuery)
      store.workspaceTabOrder = [
        .object(object.id), .query(query.id), .object(object.id), .query(UUID()),
      ]

      XCTAssertEqual(
        store.orderedWorkspaceTabs,
        [.object(object.id), .query(query.id), .query(trailingQuery.id)])
    }

    func testEmptyEnvironmentDisablesEveryFixture() {
      let configuration = NativeWorkbenchFixtureConfiguration.from(environment: [:])

      XCTAssertFalse(configuration.nativeWorkbench)
      XCTAssertFalse(configuration.nativeWorkbenchQuery)
      XCTAssertFalse(configuration.nativeWorkbenchConnections)
      XCTAssertFalse(configuration.nativeWorkbenchSetup)
      XCTAssertFalse(configuration.nativeWorkbenchStructure)
      XCTAssertFalse(configuration.nativeWorkbenchSafeReview)
      XCTAssertFalse(configuration.nativeWorkbenchDestructiveReview)
      XCTAssertFalse(configuration.multiWindow)
      XCTAssertFalse(configuration.objectTabs)
      XCTAssertFalse(configuration.dataMovementUI)
      XCTAssertFalse(configuration.valueInspector)
      XCTAssertFalse(configuration.selectableInspector)
      XCTAssertFalse(configuration.resultPaging)
      XCTAssertFalse(configuration.quickFilter)
      XCTAssertFalse(configuration.inputMethodEditor)
      XCTAssertFalse(configuration.structure)
      XCTAssertFalse(configuration.clickHouseStructure)
      XCTAssertFalse(configuration.redisOverview)
      XCTAssertFalse(configuration.redisPubSubUI)
      XCTAssertFalse(configuration.redisKeyView)
      XCTAssertNil(configuration.csvImportPath)
      XCTAssertFalse(configuration.resultCopy)
      XCTAssertNil(configuration.resultExportPath)
      XCTAssertNil(configuration.streamExportPath)
      XCTAssertFalse(configuration.queryTabs)
      XCTAssertFalse(configuration.sqlFiles)
      XCTAssertFalse(configuration.savedQueries)
      XCTAssertFalse(configuration.history)
      XCTAssertFalse(configuration.profileGroups)
      XCTAssertFalse(configuration.activeQuery)
      XCTAssertNil(configuration.performanceGridRows)
      XCTAssertFalse(configuration.performanceAutoScroll)
      XCTAssertNil(configuration.externalURL)
    }

    func testEnvironmentProjectsEveryFixtureValue() {
      let booleanKeys = [
        "TABLEROCK_FIXTURE_NATIVE_WORKBENCH",
        "TABLEROCK_FIXTURE_NATIVE_WORKBENCH_QUERY",
        "TABLEROCK_FIXTURE_NATIVE_WORKBENCH_CONNECTIONS",
        "TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SETUP",
        "TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STRUCTURE",
        "TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SAFE_REVIEW",
        "TABLEROCK_FIXTURE_NATIVE_WORKBENCH_DESTRUCTIVE_REVIEW",
        "TABLEROCK_FIXTURE_MULTI_WINDOW",
        "TABLEROCK_FIXTURE_OBJECT_TABS",
        "TABLEROCK_FIXTURE_DATA_MOVEMENT_UI",
        "TABLEROCK_FIXTURE_VALUE_INSPECTOR",
        "TABLEROCK_FIXTURE_SELECTABLE_INSPECTOR",
        "TABLEROCK_FIXTURE_RESULT_PAGING",
        "TABLEROCK_FIXTURE_QUICK_FILTER",
        "TABLEROCK_FIXTURE_IME",
        "TABLEROCK_FIXTURE_STRUCTURE",
        "TABLEROCK_FIXTURE_CLICKHOUSE_STRUCTURE",
        "TABLEROCK_FIXTURE_REDIS_OVERVIEW",
        "TABLEROCK_FIXTURE_REDIS_PUBSUB_UI",
        "TABLEROCK_FIXTURE_REDIS_KEY_VIEW",
        "TABLEROCK_FIXTURE_RESULT_COPY",
        "TABLEROCK_FIXTURE_QUERY_TABS",
        "TABLEROCK_FIXTURE_SQL_FILES",
        "TABLEROCK_FIXTURE_SAVED_QUERIES",
        "TABLEROCK_FIXTURE_HISTORY",
        "TABLEROCK_FIXTURE_PROFILE_GROUPS",
        "TABLEROCK_FIXTURE_ACTIVE_QUERY",
        "TABLEROCK_FIXTURE_AUTOSCROLL",
      ]
      var environment = Dictionary(uniqueKeysWithValues: booleanKeys.map { ($0, "1") })
      environment["TABLEROCK_FIXTURE_CSV_IMPORT_PATH"] = "/tmp/input.csv"
      environment["TABLEROCK_FIXTURE_RESULT_EXPORT_PATH"] = "/tmp/result.json"
      environment["TABLEROCK_FIXTURE_STREAM_EXPORT_PATH"] = "/tmp/stream.csv"
      environment["TABLEROCK_FIXTURE_GRID_ROWS"] = "4096"
      environment["TABLEROCK_FIXTURE_EXTERNAL_URL"] = "tablerock://connect?url=fixture"

      let configuration = NativeWorkbenchFixtureConfiguration.from(environment: environment)

      XCTAssertTrue(configuration.nativeWorkbench)
      XCTAssertTrue(configuration.nativeWorkbenchQuery)
      XCTAssertTrue(configuration.nativeWorkbenchConnections)
      XCTAssertTrue(configuration.nativeWorkbenchSetup)
      XCTAssertTrue(configuration.nativeWorkbenchStructure)
      XCTAssertTrue(configuration.nativeWorkbenchSafeReview)
      XCTAssertTrue(configuration.nativeWorkbenchDestructiveReview)
      XCTAssertTrue(configuration.multiWindow)
      XCTAssertTrue(configuration.objectTabs)
      XCTAssertTrue(configuration.dataMovementUI)
      XCTAssertTrue(configuration.valueInspector)
      XCTAssertTrue(configuration.selectableInspector)
      XCTAssertTrue(configuration.resultPaging)
      XCTAssertTrue(configuration.quickFilter)
      XCTAssertTrue(configuration.inputMethodEditor)
      XCTAssertTrue(configuration.structure)
      XCTAssertTrue(configuration.clickHouseStructure)
      XCTAssertTrue(configuration.redisOverview)
      XCTAssertTrue(configuration.redisPubSubUI)
      XCTAssertTrue(configuration.redisKeyView)
      XCTAssertEqual(configuration.csvImportPath, "/tmp/input.csv")
      XCTAssertTrue(configuration.resultCopy)
      XCTAssertEqual(configuration.resultExportPath, "/tmp/result.json")
      XCTAssertEqual(configuration.streamExportPath, "/tmp/stream.csv")
      XCTAssertTrue(configuration.queryTabs)
      XCTAssertTrue(configuration.sqlFiles)
      XCTAssertTrue(configuration.savedQueries)
      XCTAssertTrue(configuration.history)
      XCTAssertTrue(configuration.profileGroups)
      XCTAssertTrue(configuration.activeQuery)
      XCTAssertEqual(configuration.performanceGridRows, 4096)
      XCTAssertTrue(configuration.performanceAutoScroll)
      XCTAssertEqual(configuration.externalURL, "tablerock://connect?url=fixture")
    }

    func testOnlyExactOneEnablesBooleanFixture() {
      let configuration = NativeWorkbenchFixtureConfiguration.from(
        environment: ["TABLEROCK_FIXTURE_QUERY_TABS": "true"])

      XCTAssertFalse(configuration.queryTabs)
    }
  }

#endif
