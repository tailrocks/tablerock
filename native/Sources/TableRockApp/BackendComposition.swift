import TableRockBridge
import TableRockFeature

func makeConfiguredWorkbenchBackend(_ configuration: AppConfiguration) throws
  -> any WorkbenchBackend
{
  switch configuration.backend {
  case .live:
    return try makeLiveWorkbenchBackend(
      persistencePath: configuration.paths.profilesDatabase.path
    )
  #if TABLEROCK_DEVELOPMENT_SUPPORT
  case .scripted(let scenario):
    return makeDevelopmentWorkbenchBackend(scenario: scenario)
  #endif
  }
}
