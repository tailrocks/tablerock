import Testing
@testable import TableRockDesignLab

@Test func conceptAndSurfaceMatricesAreComplete() {
    #expect(LabConcept.allCases.count == 5)
    #expect(LabSurface.allCases.count == 5)
    #expect(Set(LabConcept.allCases.map(\.rawValue)).count == 5)
    #expect(Set(LabSurface.allCases.map(\.rawValue)).count == 5)
}

@Test func launchArgumentsSelectDeterministicCapture() {
    let configuration = LabLaunchConfiguration.parse([
        "TableRockDesignLab",
        "--concept", "grid-canvas",
        "--surface", "sql-results",
        "--appearance", "dark",
        "--accessibility", "reduce-transparency",
        "--engine", "clickhouse",
        "--fixture", "long-identifiers",
        "--window-size", "expanded",
        "--presentation", "query-history",
        "--inactive",
        "--capture",
    ])

    #expect(configuration.concept == .gridCanvas)
    #expect(configuration.surface == .sqlResults)
    #expect(configuration.appearance == .dark)
    #expect(configuration.accessibility == .reduceTransparency)
    #expect(configuration.engine == .clickHouse)
    #expect(configuration.fixture == .longIdentifiers)
    #expect(configuration.windowSize == .expanded)
    #expect(configuration.presentation == .queryHistory)
    #expect(configuration.windowSizeExplicit)
    #expect(configuration.inactiveCapture)
    #expect(configuration.captureMode)
}

@Test @MainActor func refinedPresentationRoutesExposeDeterministicGateStates() {
    for route in LabPresentationRoute.allCases {
        var launch = LabLaunchConfiguration()
        launch.presentation = route
        let session = LabSession(launch: launch)

        switch route {
        case .standard:
            #expect(session.surface == .dataGrid)
        case .structure:
            #expect(session.objectMode == .structure)
        case .connectionSheet:
            #expect(session.connectionSheetPresented)
        case .safeEdit:
            #expect(session.editSheetPresented)
        case .queryHistory:
            #expect(session.historySheetPresented)
        case .safeReview:
            #expect(session.reviewSheetPresented)
            #expect(!session.hasDestructiveChanges)
        case .queryError:
            #expect(session.queryErrorText == "Enter SQL before running.")
        }
    }
}

@Test func invalidLaunchArgumentsUseDocumentedFallbacks() {
    let configuration = LabLaunchConfiguration.parse([
        "TableRockDesignLab",
        "--concept", "unknown",
        "--surface", "unknown",
    ])

    #expect(configuration == LabLaunchConfiguration())
}

@Test func fixturesAreInventedStableAndInternallyAligned() {
    #expect(LabFixtures.connections.count == 3)
    #expect(LabFixtures.catalog.count == 6)
    #expect(LabFixtures.columns.count == 8)
    #expect(LabFixtures.rows.count == 12)
    #expect(LabFixtures.largeResultRows.count == 240)
    #expect(LabFixtures.changes.count == 4)
    #expect(LabFixtures.rows.allSatisfy { $0.values.count == LabFixtures.columns.count })
    #expect(LabFixtures.largeResultRows.allSatisfy {
        $0.values.count == LabFixtures.columns.count
    })
    #expect(Set(LabFixtures.rows.map(\.id)).count == LabFixtures.rows.count)
    #expect(Set(LabFixtures.largeResultRows.map(\.id)).count == LabFixtures.largeResultRows.count)
    #expect(LabFixtures.queryHistory.count == 3)
}

@Test func deterministicRoutesCoverRequiredEnginesStatesAndWindowSizes() {
    #expect(Set(LabEngine.allCases) == [.postgresql, .clickHouse, .redis])
    #expect(LabFixtureScenario.allCases.count == 9)
    #expect(Set(LabFixtureScenario.allCases) == [
        .populated,
        .empty,
        .loading,
        .connectionError,
        .largeResult,
        .longIdentifiers,
        .selectedCell,
        .pendingChange,
        .destructiveReview,
    ])
    #expect(Set(LabWindowSize.allCases) == [.minimum, .typical, .expanded])
    #expect(LabWindowSize.minimum.dimensions == .init(width: 1_280, height: 760))
    #expect(LabWindowSize.expanded.dimensions.width > LabWindowSize.typical.dimensions.width)
}

@Test @MainActor func nativeWorkbenchObjectSelectionStructureAndSortAreStateOwned() {
    let session = LabSession(launch: LabLaunchConfiguration())

    session.openObject("orders")
    #expect(session.selectedCatalogItemID == "orders")
    #expect(session.surface == .dataGrid)
    #expect(session.objectMode == .data)

    session.showStructure()
    #expect(session.objectMode == .structure)
    #expect(session.inspectorPresented)

    session.setSort(columnID: "name", ascending: true)
    #expect(session.displayedRows.first?.values[1] == "Aster Works")
    #expect(session.sortDescription == "company_name ascending")
}

@Test @MainActor func nativeWorkbenchSafeEditStagesExactReviewState() {
    let session = LabSession(launch: LabLaunchConfiguration())
    let original = session.selectedRow

    session.stageSafeEdit(plan: "Team", seats: "125")

    #expect(original?.values[3] == "Scale")
    #expect(session.stagedChanges.map(\.field) == ["plan", "seats"])
    #expect(session.pendingChangeCount == 2)
    #expect(!session.hasDestructiveChanges)

    session.presentReview()
    #expect(session.reviewSheetPresented)
    session.applyChanges()
    #expect(session.stagedChanges.isEmpty)
    #expect(session.surface == .dataGrid)
}

@Test @MainActor func nativeWorkbenchQueryRunAndHistoryStayLocal() {
    let session = LabSession(launch: LabLaunchConfiguration())
    let initialHistoryCount = session.queryHistory.count

    session.createQuery()
    session.queryText = "SELECT 1;"
    session.runQuery()

    #expect(session.queryExecuted)
    #expect(session.queryRunCount == 1)
    #expect(session.queryHistory.count == initialHistoryCount + 1)
    #expect(session.queryHistory.first?.statement == "SELECT 1;")

    let history = LabFixtures.queryHistory[1]
    session.openHistoryEntry(history)
    #expect(session.queryTitle == history.title)
    #expect(session.queryText == history.statement)
    #expect(session.engine == history.engine)
}

@Test @MainActor func nativeWorkbenchEmptyQueryReportsPresentationError() {
    let session = LabSession(launch: LabLaunchConfiguration())
    session.createQuery()

    session.runQuery()

    #expect(!session.queryExecuted)
    #expect(session.queryErrorText == "Enter SQL before running.")
    #expect(session.queryRunCount == 0)
}
