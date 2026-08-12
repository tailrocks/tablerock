import TableRockBridge
import TableRockFeature

enum NativeBackendCompositionError: Error {
  case developmentBackendUnavailable
}

func makeConfiguredWorkbenchBackend(_ configuration: AppConfiguration) throws
  -> any WorkbenchBackend
{
  switch configuration.backend {
  case .live:
    return try makeLiveWorkbenchBackend(
      persistencePath: configuration.paths.profilesDatabase.path
    )
  case .scripted(let scenario):
    #if TABLEROCK_DEVELOPMENT_SUPPORT
      return makeDevelopmentWorkbenchBackend(scenario: scenario)
    #else
      throw NativeBackendCompositionError.developmentBackendUnavailable
    #endif
  }
}
