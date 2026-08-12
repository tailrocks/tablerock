#if TABLEROCK_DEVELOPMENT_SUPPORT

func containsNativeFixtureRoute(_ environment: [String: String]) -> Bool {
    environment.keys.contains { $0.hasPrefix("TABLEROCK_FIXTURE_") }
}

public enum AppConfigurationError: Error, Equatable {
    case absoluteTestRootRequired
    case scriptedScenarioRequired
    case unsupportedBackend(String)
}

#endif
