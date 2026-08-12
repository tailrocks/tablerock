import Foundation
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  func beginCreateGroup() {
    groupDialog = ProfileGroupDialog(
      id: dependencies.identifiers.next(), oldName: nil, name: ""
    )
  }

  func beginRenameGroup(_ name: String) {
    groupDialog = ProfileGroupDialog(
      id: dependencies.identifiers.next(), oldName: name, name: name
    )
  }

  func saveGroup(_ dialog: ProfileGroupDialog) async -> Bool {
    guard let client else { return false }
    profileActionError = nil
    do {
      if let oldName = dialog.oldName {
        let moved = try await client.renameProfileGroup(oldName, dialog.name)
        collapsedProfileGroups.remove(oldName)
        profileActionOutcome = "Group renamed · \(moved) connection(s) moved"
      } else {
        try await client.createProfileGroup(dialog.name)
        profileActionOutcome = "Group created"
      }
      groupDialog = nil
      await refreshProfiles()
      return true
    } catch {
      profileActionError = "Group change failed: \(error)"
      return false
    }
  }

  func removePendingGroup() async {
    guard let client, let name = pendingGroupRemoval else { return }
    pendingGroupRemoval = nil
    profileActionError = nil
    do {
      let moved = try await client.deleteProfileGroup(name)
      collapsedProfileGroups.remove(name)
      profileActionOutcome = "Group removed · \(moved) connection(s) moved to Ungrouped"
      await refreshProfiles()
    } catch { profileActionError = "Remove group failed: \(error)" }
  }

  func setGroupAlphabetical(_ section: ProfileSection, _ alphabetical: Bool) async {
    guard let client, section.id != "ungrouped" else { return }
    profileActionError = nil
    do {
      try await client.setGroupAlphabetical(section.title, alphabetical)
      profileActionOutcome =
        alphabetical
        ? "\(section.title) sorted alphabetically"
        : "\(section.title) uses manual order"
      await refreshProfiles()
    } catch { profileActionError = "Group ordering failed: \(error)" }
  }

  func toggleFavorite(_ item: WorkbenchProfileItem) async {
    guard let client else { return }
    profileActionError = nil
    do {
      try await client.setProfileFavorite(item, !item.favorite)
      profileActionOutcome =
        item.favorite
        ? "Removed from favorites: \(item.name)"
        : "Added to favorites: \(item.name)"
      await refreshProfiles()
    } catch { profileActionError = "Favorite change failed: \(error)" }
  }

  func canMove(_ item: WorkbenchProfileItem, in section: ProfileSection, offset: Int) -> Bool {
    guard !section.alphabetical,
      let index = section.profiles.firstIndex(where: { $0.idBytes == item.idBytes })
    else { return false }
    let target = index + offset
    return section.profiles.indices.contains(target)
      && section.profiles[target].favorite == item.favorite
  }

  func move(_ item: WorkbenchProfileItem, in section: ProfileSection, offset: Int) async {
    guard let client,
      canMove(item, in: section, offset: offset),
      let index = section.profiles.firstIndex(where: { $0.idBytes == item.idBytes })
    else { return }
    var ordered = section.profiles
    ordered.swapAt(index, index + offset)
    profileActionError = nil
    do {
      try await client.reorderProfiles(
        group: section.id == "ungrouped" ? nil : section.title,
        profiles: ordered
      )
      profileActionOutcome = "Connection order updated"
      await refreshProfiles()
    } catch { profileActionError = "Reorder failed: \(error)" }
  }

  func createProfile() {
    editorDraft = ProfileEditorDraft(
      WorkbenchProfileDraft(
        idBytes: nil, revision: 0, engine: "postgresql", name: "",
        group: "", environment: "", host: "127.0.0.1", port: "5432",
        database: "postgres", username: "postgres", passwordSource: "prompt",
        passwordValue: "", passwordReference: nil, hasStoredPassword: false,
        plaintextAcknowledged: false, tlsMode: "verify_full",
        safetyMode: "confirm_writes"
      ))
  }

  func beginConnectionUrlImport() {
    connectionUrlImport = ConnectionUrlImport()
  }

  func parseConnectionUrl(_ input: String) async -> String? {
    guard let client else { return "Bridge unavailable" }
    profileActionError = nil
    do {
      var draft = ProfileEditorDraft(try await client.parseConnectionUrl(input))
      draft.name = draft.database.isEmpty ? draft.host : "\(draft.database) on \(draft.host)"
      connectionUrlImport = nil
      editorDraft = draft
      return nil
    } catch {
      let message = "URL rejected: \(error)"
      connectionUrlImport?.error = message
      return message
    }
  }

  #if TABLEROCK_DEVELOPMENT_SUPPORT
    func receiveExternalUrlFixtureIfNeeded() async {
    guard !externalUrlFixtureConsumed,
      let raw = fixtures.externalURL,
      let url = URL(string: raw)
    else { return }
    externalUrlFixtureConsumed = true
    await receiveExternalURL(url)
    }
  #endif

  func receiveExternalURL(_ externalUrl: URL) async {
    let input: String
    do {
      input = try externalConnectionUrlPayload(externalUrl)
    } catch {
      profileActionError = "External URL rejected before database parsing: \(error)"
      return
    }
    guard let client else {
      profileActionError = "External URL rejected: bridge unavailable"
      return
    }
    do {
      let draft = ProfileEditorDraft(try await client.parseConnectionUrl(input))
      let matched = profiles.first {
        $0.engine == draft.engine && $0.host == draft.host && $0.port == draft.port
          && ($0.context ?? "") == draft.database
      }
      let user = draft.username.isEmpty ? "(none)" : draft.username
      let secret = draft.passwordValue.isEmpty ? "absent" : "present"
      externalUrlReview = ExternalUrlReview(
        draft: draft,
        summary:
          "\(draft.engine) · \(draft.host):\(draft.port)/\(draft.database) · user \(user) · password \(secret) · TLS \(draft.tlsMode)",
        matchedProfile: matched
      )
      profileActionError = nil
    } catch {
      profileActionError = "External URL rejected: \(error)"
    }
  }

  func reviewExternalURLAsNewConnection() {
    guard var draft = externalUrlReview?.draft else { return }
    draft.name = draft.database.isEmpty ? draft.host : "\(draft.database) on \(draft.host)"
    externalUrlReview = nil
    editorDraft = draft
  }

  func connectExternalSavedProfile() async {
    guard let profile = externalUrlReview?.matchedProfile else { return }
    externalUrlReview = nil
    _ = await connect(profile)
  }

  func connectExternalTemporarily() async {
    guard let draft = externalUrlReview?.draft, let port = UInt16(draft.port) else { return }
    externalUrlReview = nil
    await connectTemporary(
      WorkbenchOpenParams(
        engine: draft.engine, host: draft.host, port: port, database: draft.database,
        user: draft.username, password: draft.passwordValue, tlsMode: draft.tlsMode
      ))
  }

  func showQuickSwitcher() async {
    quickSwitcherSearch = ""
    await refreshSavedQueries()
    quickSwitcherPresented = true
  }

  var quickSwitcherItems: [QuickSwitcherItem] {
    var items: [QuickSwitcherItem] = []
    items += profiles.map {
      QuickSwitcherItem(
        id: "profile:\($0.idBytes.hexEncodedString())", title: $0.name,
        subtitle: "Connection · \($0.engine) · \($0.host ?? ""):\($0.port ?? "")",
        favorite: $0.favorite, target: .profile($0.idBytes))
    }
    items += queryTabs.map {
      QuickSwitcherItem(
        id: "query:\($0.id.uuidString)", title: $0.title, subtitle: "Query tab",
        favorite: false, target: .queryTab($0.id))
    }
    items += objectTabs.map {
      QuickSwitcherItem(
        id: "object:\($0.id.uuidString)", title: $0.title,
        subtitle: $0.pinned ? "Pinned object tab" : "Preview object tab",
        favorite: $0.pinned, target: .objectTab($0.id))
    }
    items += (catalogSnapshot ?? []).filter { !$0.expandable }.map {
      QuickSwitcherItem(
        id: "catalog:\($0.idBytes.hexEncodedString())", title: $0.name,
        subtitle: "Catalog · \($0.kind.replacingOccurrences(of: "_", with: " "))",
        favorite: false, target: .catalog(catalogNodeKey($0.idBytes)))
    }
    items += savedQueries.map {
      QuickSwitcherItem(
        id: "saved:\($0.queryId)", title: $0.name, subtitle: "Saved query · \($0.engine)",
        favorite: false, target: .savedQuery($0.queryId))
    }
    let query = quickSwitcherSearch.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    return
      items
      .filter {
        query.isEmpty || $0.title.lowercased().contains(query)
          || $0.subtitle.lowercased().contains(query)
      }
      .sorted {
        if $0.favorite != $1.favorite { return $0.favorite && !$1.favorite }
        let lhsExact = $0.title.lowercased() == query
        let rhsExact = $1.title.lowercased() == query
        if lhsExact != rhsExact { return lhsExact }
        let lhsPrefix = $0.title.lowercased().hasPrefix(query)
        let rhsPrefix = $1.title.lowercased().hasPrefix(query)
        if lhsPrefix != rhsPrefix { return lhsPrefix }
        return $0.title.localizedCaseInsensitiveCompare($1.title) == .orderedAscending
      }
  }

  func activateQuickSwitcherItem(_ item: QuickSwitcherItem) async {
    quickSwitcherPresented = false
    switch item.target {
    case .profile(let id):
      if let profile = profiles.first(where: { $0.idBytes == id }) { _ = await connect(profile) }
    case .queryTab(let id):
      if let tab = queryTabs.first(where: { $0.id == id }) { selectQueryTab(tab) }
    case .objectTab(let id):
      if let tab = objectTabs.first(where: { $0.id == id }) { selectObjectTab(tab) }
    case .catalog(let key):
      await openCatalogObject(nodeKey: key)
    case .savedQuery(let id):
      if let query = savedQueries.first(where: { $0.queryId == id }) {
        restoreSavedQuery(query)
        selectedWorkbenchKind = "query"
      }
    }
  }

  func editProfile(_ item: WorkbenchProfileItem) async {
    guard let client else { return }
    profileActionError = nil
    do { editorDraft = ProfileEditorDraft(try await client.profileDraft(id: item.idBytes)) } catch {
      profileActionError = "Load connection failed: \(error)"
    }
  }

  func duplicateProfile(_ item: WorkbenchProfileItem) async {
    await editProfile(item)
    guard var copy = editorDraft else { return }
    copy.idBytes = nil
    copy.revision = 0
    copy.name += " Copy"
    if copy.hasStoredPassword { copy.passwordValue = "" }
    if copy.passwordSource == "keychain" {
      copy.passwordReference = nil
      copy.hasStoredPassword = false
    }
    editorDraft = copy
  }

  func saveProfile(_ draft: ProfileEditorDraft) async -> Bool {
    guard let client else { return false }
    profileActionError = nil
    var draft = draft
    let oldReference = draft.passwordReference
    var addedReference: Data?
    do {
      if draft.passwordSource == "keychain", !draft.passwordValue.isEmpty {
        var secret = Data(draft.passwordValue.utf8)
        defer { secret.resetBytes(in: 0..<secret.count) }
        let reference = try dependencies.keychain.store(
          secret: secret,
          account: dependencies.identifiers.next().uuidString.lowercased()
        )
        addedReference = reference
        draft.passwordReference = reference
        draft.passwordValue = ""
        draft.hasStoredPassword = true
      }
      _ = try await client.saveProfile(draft.workbench)
      var cleanupWarning = false
      if let oldReference, let addedReference, oldReference != addedReference {
        do { try dependencies.keychain.remove(reference: oldReference) } catch {
          cleanupWarning = true
        }
      }
      editorDraft = nil
      profileActionOutcome =
        cleanupWarning
        ? "Connection saved; previous Keychain item cleanup failed"
        : (draft.idBytes == nil ? "Connection created" : "Connection saved")
      await refreshProfiles()
      return true
    } catch {
      if let addedReference {
        try? dependencies.keychain.remove(reference: addedReference)
      }
      profileActionError = "Save connection failed: \(error)"
      return false
    }
  }

  func testProfile(_ item: WorkbenchProfileItem, passwordOverride: String? = nil) async {
    guard let client else { return }
    var resolvedOverride = passwordOverride.map { Data($0.utf8) }
    defer { zeroizeTransientData(&resolvedOverride) }
    if passwordOverride == nil {
      do {
        let draft = try await client.profileDraft(id: item.idBytes)
        if draft.passwordSource == "prompt" {
          passwordPrompt = ProfilePasswordPrompt(profile: item, action: .test)
          return
        }
        if draft.passwordSource == "keychain" {
          resolvedOverride = try keychainPassword(for: draft)
        }
      } catch {
        profileActionError = "Load connection failed: \(error)"
        return
      }
    }
    profileActionError = nil
    profileActionOutcome = "Testing \(item.name)…"
    do {
      let report = try await client.testProfile(
        id: item.idBytes, secretOverride: resolvedOverride
      )
      profileActionOutcome =
        "\(report.identity) · TLS \(report.tlsOutcome) · \(report.elapsedMillis) ms"
    } catch { profileActionError = "Connection test failed: \(error)" }
  }

  func removePendingProfile() async {
    guard let client, let item = pendingRemoval else { return }
    pendingRemoval = nil
    profileActionError = nil
    do {
      let reference = try await client.profileDraft(id: item.idBytes).passwordReference
      try await client.deleteProfile(id: item.idBytes, revision: item.revision)
      var cleanupWarning = false
      if let reference {
        do { try dependencies.keychain.remove(reference: reference) } catch {
          cleanupWarning = true
        }
      }
      profileActionOutcome =
        cleanupWarning
        ? "Connection removed; Keychain item cleanup failed"
        : "Connection removed: \(item.name)"
      await refreshProfiles()
    } catch { profileActionError = "Remove connection failed: \(error)" }
  }

  /// Connect directly from form params (temporary session, no saved profile).
  func connectByParams() async {
    guard let port = UInt16(formPort), !formHost.isEmpty
    else {
      connectError = "Invalid host or port"
      return
    }
    await connectTemporary(
      WorkbenchOpenParams(
        engine: formEngine, host: formHost, port: port, database: formDatabase,
        user: formUser, password: formPassword, tlsMode: "off"
      ))
  }

  private func connectTemporary(_ params: WorkbenchOpenParams) async {
    guard !hasRunningWorkbench else {
      connectError = "Cancel running queries before replacing the connection"
      return
    }
    guard let client else {
      connectError = "Bridge unavailable"
      return
    }
    let previousSession = sessionData
    await persistSessionIntent()
    connectError = nil
    do {
      let session = try await client.open(params: params)
      connectedEngine = params.engine
      activeProfileId = nil
      sessionData = session
      sessionHex = session.map { String(format: "%02x", $0) }.joined()
      sessionHealth = nil
      reconnectState = nil
      reconnectGeneration &+= 1
      if let previousSession { try? await client.disconnect(session: previousSession) }
      catalogSummary = nil
      catalogSnapshot = nil
      catalogRefreshState = .idle
      clearVolatileTabState()
      await refreshProfiles()
      await checkActiveHealth()
    } catch {
      connectError = "Connect failed: \(error)"
    }
  }

  /// Open a saved profile, prompting transiently when its source requires it.
  /// Create sample SQLite under data root, save profile, connect.
  func trySampleDatabase() async {
    guard let client else {
      profileActionError = "Bridge unavailable"
      return
    }
    do {
      let draft = try await client.prepareSampleDatabase(dataRoot: dataRootPath)
      let id = try await client.saveProfile(draft)
      await refreshProfiles()
      if let item = profiles.first(where: { $0.idBytes == id }) {
        let connected = await connect(item)
        guard connected else {
          // connect() already sets connectError; do not claim success.
          if profileActionError == nil && connectError == nil {
            profileActionError = "Sample database saved but connect failed"
          }
          return
        }
        if let tabIndex = queryTabs.indices.first {
          queryTabs[tabIndex].statementText =
            """
            SELECT t.name AS track, a.title AS album, ar.name AS artist
            FROM tracks t
            JOIN albums a ON a.id = t.album_id
            JOIN artists ar ON ar.id = a.artist_id
            ORDER BY t.id;
            """
        }
        profileActionOutcome = "Sample database ready"
      } else {
        profileActionError = "Sample profile missing after save"
      }
    } catch {
      profileActionError = "Sample database failed: \(error)"
    }
  }

  @discardableResult
  func connect(_ item: WorkbenchProfileItem, passwordOverride: String? = nil) async -> Bool {
    guard let client else { return false }
    guard !hasRunningWorkbench else {
      connectError = "Cancel running queries before replacing the connection"
      return false
    }
    var resolvedOverride = passwordOverride.map { Data($0.utf8) }
    defer { zeroizeTransientData(&resolvedOverride) }
    if passwordOverride == nil {
      do {
        let draft = try await client.profileDraft(id: item.idBytes)
        // Local SQLite (sample / file path) is passwordless — never prompt.
        let isSqlite =
          item.engine.caseInsensitiveCompare("sqlite") == .orderedSame
          || draft.engine.caseInsensitiveCompare("sqlite") == .orderedSame
        if draft.passwordSource == "prompt", !isSqlite {
          passwordPrompt = ProfilePasswordPrompt(profile: item, action: .connect)
          return false
        }
        if draft.passwordSource == "keychain" {
          resolvedOverride = try keychainPassword(for: draft)
        }
      } catch {
        connectError = "Load connection failed: \(error)"
        return false
      }
    }
    let previousSession = sessionData
    await persistSessionIntent()
    connectingName = item.name
    connectError = nil
    do {
      let session = try await client.openProfile(
        id: item.idBytes, secretOverride: resolvedOverride
      )
      connectedEngine = item.engine
      activeProfileId = item.idBytes
      sessionData = session
      sessionHex = session.map { String(format: "%02x", $0) }.joined()
      sessionHealth = nil
      reconnectState = nil
      reconnectGeneration &+= 1
      if let previousSession { try? await client.disconnect(session: previousSession) }
      catalogSummary = nil
      catalogError = nil
      catalogSnapshot = nil
      catalogRefreshState = .idle
      clearVolatileTabState()
      await restoreSessionIntent(profileId: item.idBytes)
      await refreshProfiles()
      await checkActiveHealth()
      passwordPrompt = nil
      connectingName = nil
      return true
    } catch {
      connectError = "Connect failed: \(error)"
      connectingName = nil
      return false
    }
  }

  func disconnectActive() async {
    guard let client, let session = sessionData else { return }
    await persistSessionIntent()
    if redisSubscriptionIsActive { await closeRedisSubscription() }
    do {
      try await client.disconnect(session: session)
      sessionData = nil
      sessionHex = nil
      connectedEngine = ""
      sessionHealth = nil
      reconnectState = nil
      reconnectGeneration &+= 1
      catalogSummary = nil
      catalogSnapshot = nil
      catalogRefreshState = .idle
      resultTable = nil
      profileActionOutcome = "Disconnected"
      await refreshProfiles()
    } catch { profileActionError = "Disconnect failed: \(error)" }
  }

  func checkActiveHealth() async {
    guard let client, let session = sessionData, !healthChecking else { return }
    healthChecking = true
    defer { healthChecking = false }
    do {
      sessionHealth = try await client.checkHealth(session: session)
      if sessionHealth?.serverReachable == false {
        await reconnectAutomatically(
          sourceSession: session,
          authenticationStopped: sessionHealth?.authenticationStopped == true
        )
      }
    } catch {
      sessionHealth = WorkbenchSessionHealth(
        state: "unhealthy", serverReachable: false,
        elapsedMillis: nil, authenticationStopped: false
      )
      profileActionError = "Health check failed: \(error)"
    }
  }

  func reconnectActive() async {
    guard let client, let sourceSession = sessionData else { return }
    if let activeProfileId,
      let profile = profiles.first(where: { $0.idBytes == activeProfileId })
    {
      do {
        let draft = try await client.profileDraft(id: profile.idBytes)
        if draft.passwordSource == "prompt" {
          passwordPrompt = ProfilePasswordPrompt(profile: profile, action: .reconnect)
          return
        }
        if draft.passwordSource == "keychain" {
          let password = try keychainPassword(for: draft)
          await reconnectActive(
            sourceSession: sourceSession, secretOverride: password
          )
          return
        }
      } catch {
        profileActionError = "Load connection failed: \(error)"
        return
      }
    }
    await reconnectActive(sourceSession: sourceSession, secretOverride: nil)
  }

  private func keychainPassword(for draft: WorkbenchProfileDraft) throws -> Data {
    guard let reference = draft.passwordReference else {
      throw AppCapabilityError.rejected("keychain-reference-missing")
    }
    let bytes = try dependencies.keychain.read(reference: reference)
    guard !bytes.isEmpty else {
      throw AppCapabilityError.rejected("keychain-value-invalid")
    }
    return bytes
  }

  private func reconnectActive(sourceSession: Data, secretOverride: Data?) async {
    guard let client else { return }
    var secretOverride = secretOverride
    defer { zeroizeTransientData(&secretOverride) }
    reconnectGeneration &+= 1
    let generation = reconnectGeneration
    reconnectState = "Reconnecting"
    do {
      let attempt = try await client.reconnect(
        session: sourceSession, secretOverride: secretOverride
      )
      guard attempt.state == "connected", let replacement = attempt.sessionId else {
        reconnectState =
          attempt.state == "authentication_stopped"
          ? "Authentication stopped" : "Reconnect failed"
        return
      }
      guard generation == reconnectGeneration else {
        try? await client.disconnect(session: replacement)
        return
      }
      sessionData = replacement
      sessionHex = replacement.map { String(format: "%02x", $0) }.joined()
      reconnectState = nil
      sessionHealth = try await client.checkHealth(session: replacement)
      await refreshProfiles()
    } catch {
      guard generation == reconnectGeneration else { return }
      reconnectState = "Reconnect failed"
      profileActionError = "Reconnect failed: \(error)"
    }
  }

  func submitPasswordPrompt(_ prompt: ProfilePasswordPrompt, password: String) async -> Bool {
    switch prompt.action {
    case .connect:
      return await connect(prompt.profile, passwordOverride: password)
    case .test:
      await testProfile(prompt.profile, passwordOverride: password)
      if profileActionError == nil {
        passwordPrompt = nil
        return true
      }
      return false
    case .reconnect:
      guard let sourceSession = sessionData else { return false }
      await reconnectActive(
        sourceSession: sourceSession, secretOverride: Data(password.utf8)
      )
      if reconnectState == nil {
        passwordPrompt = nil
        return true
      }
      return false
    }
  }

  private func reconnectAutomatically(
    sourceSession: Data,
    authenticationStopped: Bool
  ) async {
    guard let client else { return }
    reconnectGeneration &+= 1
    let generation = reconnectGeneration
    var attempt: UInt32 = 0
    while generation == reconnectGeneration, sessionData == sourceSession {
      let plan: WorkbenchReconnectPlan
      do {
        plan = try await client.planReconnect(
          session: sourceSession, attempt: attempt,
          authenticationStopped: authenticationStopped
        )
      } catch {
        reconnectState = "Reconnect unavailable"
        return
      }
      switch plan.action {
      case "manual":
        reconnectState = nil
        return
      case "authentication_stopped":
        reconnectState = "Authentication stopped"
        return
      case "exhausted":
        reconnectState = "Reconnect budget exhausted"
        return
      case "retry":
        let delay = plan.delayMillis ?? 0
        reconnectState = "Reconnecting · attempt \(attempt + 1)"
        if delay > 0 {
          try? await Task.sleep(for: .milliseconds(Int64(delay)))
        }
        guard generation == reconnectGeneration, sessionData == sourceSession else {
          return
        }
        do {
          let reconnectAttempt = try await client.reconnect(
            session: sourceSession, secretOverride: nil
          )
          if reconnectAttempt.state == "authentication_stopped" {
            reconnectState = "Authentication stopped"
            return
          }
          guard reconnectAttempt.state == "connected",
            let replacement = reconnectAttempt.sessionId
          else {
            attempt &+= 1
            continue
          }
          guard generation == reconnectGeneration else {
            try? await client.disconnect(session: replacement)
            return
          }
          sessionData = replacement
          sessionHex = replacement.map { String(format: "%02x", $0) }.joined()
          reconnectState = nil
          sessionHealth = try await client.checkHealth(session: replacement)
          await refreshProfiles()
          return
        } catch {
          attempt &+= 1
        }
      default:
        reconnectState = "Reconnect unavailable"
        return
      }
    }
  }

  func connectionState(_ profile: WorkbenchProfileItem) -> String {
    if connectingName == profile.name { return "Connecting" }
    guard profile.connected else { return "Disconnected" }
    guard isActiveProfile(profile) else { return "Connected in another window" }
    if let reconnectState { return reconnectState }
    guard let sessionHealth else { return "Connected" }
    switch sessionHealth.state {
    case "healthy":
      return sessionHealth.elapsedMillis.map { "Healthy · \($0) ms" } ?? "Healthy"
    case "authentication_stopped": return "Authentication stopped"
    case "timeout": return "Health timeout"
    case "unreachable": return "Unreachable"
    default: return "Unhealthy"
    }
  }

  func isActiveProfile(_ profile: WorkbenchProfileItem) -> Bool {
    sessionData != nil && activeProfileId == profile.idBytes
  }
}
