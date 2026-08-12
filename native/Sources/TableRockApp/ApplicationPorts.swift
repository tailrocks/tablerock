import AppKit
import Security
import TableRockFeature
import UniformTypeIdentifiers

func nativeApplicationSupportRoot() throws -> URL {
  try FileManager.default.url(
    for: .applicationSupportDirectory,
    in: .userDomainMask,
    appropriateFor: nil,
    create: true
  )
}

@MainActor
struct SystemFilePanelPort: AppFilePanelPort {
  func chooseOpenFile(_ request: AppFilePanelRequest) -> URL? {
    let panel = NSOpenPanel()
    configure(panel, request: request)
    panel.allowsMultipleSelection = false
    panel.canChooseDirectories = false
    panel.canChooseFiles = true
    return panel.runModal() == .OK ? panel.url : nil
  }

  func chooseSaveFile(_ request: AppFilePanelRequest) -> URL? {
    let panel = NSSavePanel()
    configure(panel, request: request)
    return panel.runModal() == .OK ? panel.url : nil
  }

  private func configure(_ panel: NSSavePanel, request: AppFilePanelRequest) {
    panel.title = request.title
    panel.prompt = request.prompt
    if let suggestedFilename = request.suggestedFilename {
      panel.nameFieldStringValue = suggestedFilename
    }
    panel.allowedContentTypes = request.allowedExtensions.map {
      UTType(filenameExtension: $0) ?? .plainText
    }
  }
}

@MainActor
struct TestFilePanelPort: AppFilePanelPort {
  let root: URL
  let openPath: String?
  let savePath: String?

  func chooseOpenFile(_ request: AppFilePanelRequest) -> URL? {
    confined(openPath)
  }

  func chooseSaveFile(_ request: AppFilePanelRequest) -> URL? {
    confined(savePath)
  }

  private func confined(_ path: String?) -> URL? {
    guard let path else { return nil }
    let candidate = URL(fileURLWithPath: path).standardizedFileURL
    let root = root.resolvingSymlinksInPath().standardizedFileURL
    let parent = candidate.deletingLastPathComponent().resolvingSymlinksInPath()
    let resolved = parent.appendingPathComponent(candidate.lastPathComponent).standardizedFileURL
    guard resolved.path.hasPrefix(root.path + "/") else { return nil }
    return resolved
  }
}

@MainActor
struct SystemPasteboardPort: AppPasteboardPort {
  func write(_ representations: [AppPasteboardRepresentation]) throws {
    let item = NSPasteboardItem()
    for representation in representations {
      item.setString(
        representation.value,
        forType: NSPasteboard.PasteboardType(representation.type)
      )
    }
    NSPasteboard.general.clearContents()
    guard NSPasteboard.general.writeObjects([item]) else {
      throw AppCapabilityError.rejected("pasteboard")
    }
  }
}

@MainActor
struct SystemKeychainPort: AppKeychainPort {
  let namespace: String

  func store(secret: Data, account: String) throws -> Data {
    let query: [CFString: Any] = [
      kSecClass: kSecClassGenericPassword,
      kSecAttrService: namespace,
      kSecAttrAccount: account,
      kSecValueData: secret,
      kSecReturnPersistentRef: true,
    ]
    var result: CFTypeRef?
    let status = SecItemAdd(query as CFDictionary, &result)
    guard status == errSecSuccess, let reference = result as? Data else {
      throw AppCapabilityError.rejected("keychain-store-\(status)")
    }
    return reference
  }

  func read(reference: Data) throws -> Data {
    let query: [CFString: Any] = [
      kSecClass: kSecClassGenericPassword,
      kSecAttrService: namespace,
      kSecMatchItemList: [reference] as CFArray,
      kSecReturnData: true,
      kSecMatchLimit: kSecMatchLimitOne,
    ]
    var result: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    guard status == errSecSuccess, let secret = result as? Data, !secret.isEmpty else {
      throw AppCapabilityError.rejected("keychain-read-\(status)")
    }
    return secret
  }

  func remove(reference: Data) throws {
    let status = SecItemDelete(
      [
        kSecClass: kSecClassGenericPassword,
        kSecAttrService: namespace,
        kSecMatchItemList: [reference] as CFArray,
      ] as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else {
      throw AppCapabilityError.rejected("keychain-remove-\(status)")
    }
  }
}
