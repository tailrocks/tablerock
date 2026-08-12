#if TABLEROCK_DEVELOPMENT_SUPPORT

func containsNativeFixtureRoute(_ environment: [String: String]) -> Bool {
    environment.keys.contains { $0.hasPrefix("TABLEROCK_FIXTURE_") }
}

#endif
