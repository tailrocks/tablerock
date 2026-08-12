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
    #expect(configuration.inactiveCapture)
    #expect(configuration.captureMode)
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
