import Foundation
import TableRockFeature

@MainActor
final class NativeApplicationModel {
  let client: (any WorkbenchBackend)?
  let bridgeError: String?
  let dependencies: AppDependencies
  let launchConfiguration: NativeLaunchConfiguration
  let appearanceFixture: NativeAppearanceFixture
  /// Operator data root (Application Support/TableRock or test root).
  let dataRootPath: String
  private var fixtureWindowOpened = false

  init() {
    launchConfiguration = .current
    appearanceFixture = .current
    var configuredDependencies = AppDependencies(
      filePanels: SystemFilePanelPort(),
      pasteboard: SystemPasteboardPort()
    )
    do {
      let configuration = try AppConfiguration.resolve(
        environment: ProcessInfo.processInfo.environment,
        applicationSupportRoot: nativeApplicationSupportRoot(),
        temporaryRoot: FileManager.default.temporaryDirectory,
        processIdentifier: ProcessInfo.processInfo.processIdentifier
      )
      let filePanels: any AppFilePanelPort =
        configuration.isTestMode
        ? TestFilePanelPort(
          root: configuration.paths.dataRoot,
          openPath: ProcessInfo.processInfo.environment["TABLEROCK_TEST_OPEN_FILE"],
          savePath: ProcessInfo.processInfo.environment["TABLEROCK_TEST_SAVE_FILE"]
        ) : SystemFilePanelPort()
      configuredDependencies = AppDependencies(
        filePanels: filePanels,
        pasteboard: SystemPasteboardPort(),
        keychain: SystemKeychainPort(namespace: configuration.keychainNamespace)
      )
      try configuration.paths.prepare()
      let configuredClient = try makeConfiguredWorkbenchBackend(configuration)
      dependencies = configuredDependencies
      client = configuredClient
      dataRootPath = configuration.paths.dataRoot.path
      bridgeError = nil
    } catch {
      dependencies = configuredDependencies
      client = nil
      dataRootPath = FileManager.default.temporaryDirectory.path
      bridgeError = "Bridge init failed: \(error)"
    }
  }

  func claimMultiWindowFixtureOpen() -> Bool {
    guard !fixtureWindowOpened else { return false }
    fixtureWindowOpened = true
    return true
  }
}
