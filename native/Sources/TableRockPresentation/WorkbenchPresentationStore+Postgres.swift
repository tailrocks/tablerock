import Foundation
import TableRockFeature

@MainActor
extension WorkbenchPresentationStore {
  func showPostgresActivity() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresActivityPresented = true
    await refreshPostgresActivity()
  }

  func refreshPostgresActivity() async {
    guard let client, let session = sessionData, !postgresActivityLoading else { return }
    postgresActivityLoading = true
    postgresActivityError = nil
    defer { postgresActivityLoading = false }
    do {
      postgresActivityRows = try await client.postgresActivity(sessionId: session)
    } catch {
      postgresActivityRows = []
      postgresActivityError = "PostgreSQL activity failed: \(error)"
    }
  }

  func showPostgresRelationships() async {
    guard connectedEngine == "postgresql", activeObjectTab != nil else { return }
    postgresRelationshipsPresented = true
    await refreshPostgresRelationships()
  }

  func refreshPostgresRelationships() async {
    guard let client, let session = sessionData, let object = activeObjectTab,
      !postgresRelationshipsLoading
    else { return }
    postgresRelationshipsLoading = true
    postgresRelationshipsError = nil
    defer { postgresRelationshipsLoading = false }
    do {
      postgresRelationshipSnapshot = try await client.postgresRelationships(
        sessionId: session, catalogNodeId: object.catalogNodeId)
    } catch {
      postgresRelationshipSnapshot = nil
      postgresRelationshipsError = "Relationships unavailable: \(error)"
    }
  }

  func showPostgresRoles() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresRolesPresented = true
    await refreshPostgresRoles()
  }

  func refreshPostgresRoles() async {
    guard let client, let session = sessionData, !postgresRolesLoading else { return }
    postgresRolesLoading = true
    postgresRolesError = nil
    defer { postgresRolesLoading = false }
    do {
      postgresRoleSnapshot = try await client.postgresRoles(
        sessionId: session, catalogNodeId: activeObjectTab?.catalogNodeId)
    } catch {
      postgresRoleSnapshot = nil
      postgresRolesError = "Roles unavailable: \(error)"
    }
  }

  func stagePostgresRoleChange() async {
    guard let client, let session = sessionData else { return }
    postgresRolesError = nil
    postgresRoleChangeOutcome = nil
    do {
      postgresRoleChangeReview = try await client.stagePostgresRoleChange(
        sessionId: session, catalogNodeId: activeObjectTab?.catalogNodeId,
        kind: postgresRoleChangeKind,
        role: postgresRoleChangeRole.trimmingCharacters(in: .whitespacesAndNewlines),
        memberOrGrantee: postgresRoleChangeSubject.trimmingCharacters(in: .whitespacesAndNewlines),
        privilege: postgresRoleChangePrivilege,
        nowMs: dependencies.clock.nowMilliseconds())
    } catch {
      postgresRoleChangeReview = nil
      postgresRolesError = "Role change rejected: \(error)"
    }
  }

  func applyPostgresRoleChange() async {
    guard let client, let session = sessionData, let review = postgresRoleChangeReview else {
      return
    }
    postgresRoleChangeReview = nil
    do {
      postgresRoleChangeOutcome = try await client.applyPostgresRoleChange(
        tokenId: review.tokenId, sessionId: session,
        nowMs: dependencies.clock.nowMilliseconds(), confirmed: true)
      await refreshPostgresRoles()
    } catch {
      postgresRolesError = "Role change outcome unknown or failed; review consumed: \(error)"
    }
  }

  func discardPostgresRoleChange() async {
    if let review = postgresRoleChangeReview, let client {
      _ = try? await client.revokePostgresRoleChange(tokenId: review.tokenId)
    }
    postgresRoleChangeReview = nil
  }

  func openRelatedRelation(_ edge: WorkbenchRelationshipEdge) async {
    guard let snapshot = postgresRelationshipSnapshot, let nodes = catalogSnapshot else { return }
    let selectedIsSource =
      edge.fromSchema == snapshot.namespace && edge.fromTable == snapshot.relation
    let namespace = selectedIsSource ? edge.toSchema : edge.fromSchema
    let relation = selectedIsSource ? edge.toTable : edge.fromTable
    let node = nodes.first { candidate in
      guard candidate.name == relation, let parentId = candidate.parentIdBytes else { return false }
      return nodes.first(where: { $0.idBytes == parentId })?.name == namespace
    }
    guard let node else {
      postgresRelationshipsError = "Load \(namespace).\(relation) in the catalog before opening it."
      return
    }
    postgresRelationshipsPresented = false
    await openCatalogObject(nodeKey: catalogNodeKey(node.idBytes))
  }

  func signalPostgresBackend(kind: String, pid: Int32) async {
    guard let client, let session = sessionData else { return }
    postgresActivityError = nil
    postgresActivityOutcome = nil
    do {
      let outcome = try await client.signalPostgresBackend(
        sessionId: session, kind: kind, pid: pid)
      postgresActivityOutcome =
        outcome.acknowledged
        ? "\(kind.capitalized) acknowledged for PID \(pid)"
        : "PID \(pid) was not signalable"
      await refreshPostgresActivity()
    } catch {
      postgresActivityError = "\(kind.capitalized) failed: \(error)"
    }
  }

  func showPostgresTools() async {
    guard connectedEngine == "postgresql", sessionData != nil else { return }
    postgresToolsPresented = true
    postgresToolError = nil
    await probePostgresTool()
  }

  func probePostgresTool() async {
    guard let client else { return }
    postgresToolError = nil
    let explicit = postgresToolExplicitPath.trimmingCharacters(in: .whitespacesAndNewlines)
    do {
      postgresToolProbe = try await client.probePostgresTool(
        kind: postgresToolKind,
        explicitPath: explicit.isEmpty ? nil : explicit)
    } catch {
      postgresToolProbe = nil
      postgresToolError = "Tool probe failed: \(error)"
    }
  }

  func choosePostgresToolFile() {
    let request = AppFilePanelRequest(
      title: postgresToolKind == "dump" ? "Choose Backup Destination" : "Choose Restore Archive",
      prompt: postgresToolKind == "dump" ? "Choose" : "Restore",
      suggestedFilename: postgresToolKind == "dump" ? "tablerock.dump" : nil,
      allowedExtensions: ["dump", "backup"])
    postgresToolFileUrl =
      postgresToolKind == "dump"
      ? dependencies.filePanels.chooseSaveFile(request)
      : dependencies.filePanels.chooseOpenFile(request)
    postgresToolStatus = nil
    postgresToolError = nil
  }

  func requestStartPostgresTool() {
    guard postgresToolProbe?.available == true, postgresToolFileUrl != nil else {
      postgresToolError = "Choose an available tool and archive file first"
      return
    }
    postgresToolReviewRequested = true
  }

  func startPostgresTool() async {
    postgresToolReviewRequested = false
    guard let client, let session = sessionData, let tool = postgresToolProbe?.path,
      let file = postgresToolFileUrl
    else { return }
    postgresToolError = nil
    postgresToolStatus = nil
    postgresToolSecurityScopeActive = file.startAccessingSecurityScopedResource()
    do {
      let operation = try await client.startPostgresTool(
        sessionId: session, kind: postgresToolKind, toolPath: tool, filePath: file.path,
        content: postgresToolContent, clean: postgresToolKind == "restore" && postgresToolClean,
        noOwner: postgresToolNoOwner)
      postgresToolStatus = WorkbenchPostgresToolStatus(
        operationId: operation, kind: postgresToolKind, phase: "running",
        summary: "Process started")
      await pollPostgresTool(operation)
    } catch {
      releasePostgresToolSecurityScope()
      postgresToolError = "PostgreSQL tool failed to start: \(error)"
    }
  }

  private func pollPostgresTool(_ operation: Data) async {
    guard let client else { return }
    while true {
      do {
        let status = try await client.postgresToolStatus(operationId: operation)
        postgresToolStatus = status
        if status.phase != "running" && status.phase != "cancel_requested" {
          releasePostgresToolSecurityScope()
          return
        }
      } catch {
        releasePostgresToolSecurityScope()
        postgresToolError = "PostgreSQL tool status failed: \(error)"
        return
      }
      try? await Task.sleep(for: .milliseconds(200))
    }
  }

  func cancelPostgresTool() async {
    guard let client, let operation = postgresToolStatus?.operationId else { return }
    do {
      if try await client.cancelPostgresTool(operationId: operation) {
        postgresToolStatus = WorkbenchPostgresToolStatus(
          operationId: operation, kind: postgresToolKind, phase: "cancel_requested",
          summary: "Cancellation requested")
      }
    } catch { postgresToolError = "PostgreSQL tool cancellation failed: \(error)" }
  }

  func closePostgresTools() {
    guard
      postgresToolStatus?.phase != "running"
        && postgresToolStatus?.phase != "cancel_requested"
    else { return }
    releasePostgresToolSecurityScope()
    postgresToolsPresented = false
  }

  private func releasePostgresToolSecurityScope() {
    if postgresToolSecurityScopeActive, let file = postgresToolFileUrl {
      file.stopAccessingSecurityScopedResource()
    }
    postgresToolSecurityScopeActive = false
  }
}
