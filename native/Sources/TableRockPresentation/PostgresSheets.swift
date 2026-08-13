import Foundation
import SwiftUI
import TableRockFeature

private struct PendingPostgresSignal {
  let kind: String
  let pid: Int32
}

struct PostgresRolesSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  private var matchingRoles: [String] {
    guard let snapshot = model.postgresRoleSnapshot else { return [] }
    let query = model.postgresRoleSearch.trimmingCharacters(in: .whitespacesAndNewlines)
    return query.isEmpty
      ? snapshot.roles
      : snapshot.roles.filter { $0.localizedCaseInsensitiveContains(query) }
  }
  private var isPrivilegeChange: Bool {
    model.postgresRoleChangeKind.hasSuffix("privilege")
  }

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 12) {
      HStack {
        Label("PostgreSQL Roles and Privileges", systemImage: "person.2")
          .font(.headline)
        Spacer()
        Button("Refresh") { Task { await model.refreshPostgresRoles() } }
          .disabled(model.postgresRolesLoading)
        Button("Close") {
          Task { await model.discardPostgresRoleChange() }
          model.postgresRolesPresented = false
        }
      }
      TextField("Search roles", text: $model.postgresRoleSearch)
        .textFieldStyle(.roundedBorder)
        .accessibilityIdentifier("postgres.roles.search")
      if let snapshot = model.postgresRoleSnapshot {
        Text("Current user: \(snapshot.currentUser)").font(.subheadline)
        HStack(alignment: .top, spacing: 16) {
          GroupBox("Roles") {
            List(matchingRoles, id: \.self) { role in Text(role) }
          }
          GroupBox("Effective membership") {
            List(snapshot.effectiveRoles, id: \.self) { role in Text(role) }
          }
          GroupBox("Direct memberships") {
            List(snapshot.memberships) { membership in
              VStack(alignment: .leading) {
                Text("\(membership.member) in \(membership.role)")
                Text(
                  "inherit \(membership.inheritOption ? "yes" : "no") · admin \(membership.adminOption ? "yes" : "no") · set \(membership.setOption ? "yes" : "no")"
                )
                .font(.caption).foregroundStyle(.secondary)
              }
            }
          }
        }
        .frame(minHeight: 150)
        GroupBox(snapshot.privilegeScope.map { "Privileges · \($0)" } ?? "Privileges") {
          if snapshot.privilegesUnavailable {
            Text("Privileges unavailable for this relation.")
          } else if snapshot.privileges.isEmpty {
            Text(
              snapshot.privilegeScope == nil
                ? "Select a relation to inspect grants." : "No grants found.")
          } else {
            List(snapshot.privileges) { privilege in
              HStack {
                Text(privilege.grantee)
                Text(privilege.privilege).fontWeight(.medium)
                Spacer()
                Text(privilege.grantable ? "Grantable" : "Not grantable")
                  .foregroundStyle(.secondary)
              }
            }
            .frame(minHeight: 100)
          }
        }
        if !snapshot.cycleEdges.isEmpty {
          Label("Membership cycle detected", systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange)
        }
        if snapshot.truncated {
          Label("Snapshot truncated at safety limits", systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange)
        }
        GroupBox("Reviewed change") {
          VStack(alignment: .leading, spacing: 8) {
            Picker("Action", selection: $model.postgresRoleChangeKind) {
              Text("Grant membership").tag("grant_membership")
              Text("Revoke membership").tag("revoke_membership")
              Text("Grant table privilege").tag("grant_privilege")
              Text("Revoke table privilege").tag("revoke_privilege")
            }
            .pickerStyle(.segmented)
            if !isPrivilegeChange {
              TextField("Role", text: $model.postgresRoleChangeRole)
                .accessibilityIdentifier("postgres.roles.change.role")
            }
            TextField(
              isPrivilegeChange ? "Grantee" : "Member", text: $model.postgresRoleChangeSubject
            )
            .accessibilityIdentifier("postgres.roles.change.subject")
            if isPrivilegeChange {
              Picker("Privilege", selection: $model.postgresRoleChangePrivilege) {
                ForEach(
                  ["SELECT", "INSERT", "UPDATE", "DELETE", "TRUNCATE", "REFERENCES", "TRIGGER"],
                  id: \.self
                ) {
                  Text($0).tag($0)
                }
              }
              Text("Privilege changes use selected relation only.").font(.caption)
            }
            Button("Review Change…") { Task { await model.stagePostgresRoleChange() } }
              .disabled(
                model.postgresRoleChangeSubject.trimmingCharacters(in: .whitespacesAndNewlines)
                  .isEmpty
                  || (!isPrivilegeChange
                    && model.postgresRoleChangeRole.trimmingCharacters(in: .whitespacesAndNewlines)
                      .isEmpty)
                  || (isPrivilegeChange && model.selectedObjectTab == nil)
              )
              .accessibilityIdentifier("postgres.roles.change.review")
            Text("Revoking current-user authority is blocked before review.")
              .font(.caption).foregroundStyle(.secondary)
          }
        }
        if let outcome = model.postgresRoleChangeOutcome {
          Text(outcome).foregroundStyle(.green)
            .accessibilityIdentifier("postgres.roles.change.outcome")
        }
      }
      if model.postgresRolesLoading { ProgressView("Loading roles…") }
      if let error = model.postgresRolesError {
        Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.red)
      }
    }
    .padding(18)
    .frame(minWidth: 720, minHeight: 520)
    .confirmationDialog(
      "Apply role change?",
      isPresented: Binding(
        get: { model.postgresRoleChangeReview != nil },
        set: { if !$0 { Task { await model.discardPostgresRoleChange() } } }
      ),
      presenting: model.postgresRoleChangeReview
    ) { _ in
      Button("Apply Role Change", role: .destructive) {
        Task { await model.applyPostgresRoleChange() }
      }
      Button("Cancel", role: .cancel) { Task { await model.discardPostgresRoleChange() } }
    } message: { review in
      Text("\(review.summary). Authority expires in 60 seconds and is consumed on apply.")
    }
  }
}

struct PostgresRelationshipsSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack {
        Label("Relation Lens", systemImage: "arrow.triangle.branch")
          .font(.headline)
        Spacer()
        Button("Refresh") { Task { await model.refreshPostgresRelationships() } }
          .disabled(model.postgresRelationshipsLoading)
        Button("Close") { model.postgresRelationshipsPresented = false }
      }
      if let snapshot = model.postgresRelationshipSnapshot {
        Text(
          "\(snapshot.namespace).\(snapshot.relation) · \(snapshot.edges.count) foreign-key columns"
        )
        .font(.caption)
        .foregroundStyle(.secondary)
        if snapshot.truncated {
          Label("Showing first 512 edges", systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange)
        }
        if snapshot.edges.isEmpty && !model.postgresRelationshipsLoading {
          ContentUnavailableView(
            "No relationships", systemImage: "arrow.triangle.branch",
            description: Text("No inbound or outbound foreign keys were found."))
        } else {
          List(snapshot.edges) { edge in
            HStack(spacing: 10) {
              VStack(alignment: .leading, spacing: 3) {
                Text("\(edge.fromSchema).\(edge.fromTable).\(edge.fromColumn)")
                Text("→ \(edge.toSchema).\(edge.toTable).\(edge.toColumn)")
                  .foregroundStyle(.secondary)
                if edge.fromSchema == edge.toSchema && edge.fromTable == edge.toTable {
                  Text("Self-reference").font(.caption).foregroundStyle(.orange)
                }
              }
              Spacer()
              Button("Relation Lens") { Task { await model.openRelatedRelation(edge) } }
                .buttonStyle(.glass)
                .accessibilityLabel("Open Relation Lens for \(edge.id)")
                .accessibilityIdentifier("relation.lens.open")
            }
          }
        }
      }
      if model.postgresRelationshipsLoading { ProgressView("Loading relationships…") }
      if let error = model.postgresRelationshipsError {
        Label(error, systemImage: "exclamationmark.triangle")
          .foregroundStyle(.red)
      }
    }
    .padding(18)
    .frame(minWidth: 680, minHeight: 420)
  }
}

struct PostgresActivitySheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @State private var pendingSignal: PendingPostgresSignal?

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("PostgreSQL Activity", systemImage: "waveform.path.ecg")
          .font(.title2.bold())
        Spacer()
        Button("Refresh") { Task { await model.refreshPostgresActivity() } }
          .disabled(model.postgresActivityLoading)
          .accessibilityIdentifier("postgres.activity.refresh")
        Button("Close") { model.postgresActivityPresented = false }
          .accessibilityIdentifier("postgres.activity.close")
      }
      Text("Current client backends. Cancel stops one query; terminate closes its session.")
        .font(.callout)
        .foregroundStyle(.secondary)
      if model.postgresActivityLoading {
        ProgressView("Loading bounded pg_stat_activity snapshot…")
      }
      if model.postgresActivityRows.isEmpty && !model.postgresActivityLoading
        && model.postgresActivityError == nil
      {
        ContentUnavailableView(
          "No client backends", systemImage: "server.rack",
          description: Text("Refresh to inspect current PostgreSQL activity."))
      } else {
        List(model.postgresActivityRows) { row in
          VStack(alignment: .leading, spacing: 6) {
            HStack {
              Text("PID \(row.pid)").font(.headline)
              Text(row.state).foregroundStyle(.secondary)
              Spacer()
              Button("Cancel Query") {
                pendingSignal = PendingPostgresSignal(kind: "cancel", pid: row.pid)
              }
              .accessibilityIdentifier("postgres.activity.cancel.\(row.pid)")
              Button("Terminate Session", role: .destructive) {
                pendingSignal = PendingPostgresSignal(kind: "terminate", pid: row.pid)
              }
              .accessibilityIdentifier("postgres.activity.terminate.\(row.pid)")
            }
            Text(
              "\(row.user) · \(row.application.isEmpty ? "unknown application" : row.application)"
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            Text(row.queryPreview.isEmpty ? "No query text" : row.queryPreview)
              .font(.system(.body, design: .monospaced))
              .textSelection(.enabled)
          }
          .padding(.vertical, 4)
          .accessibilityElement(children: .contain)
          .accessibilityIdentifier("postgres.activity.row.\(row.pid)")
        }
      }
      if let outcome = model.postgresActivityOutcome {
        Label(outcome, systemImage: "checkmark.circle.fill")
          .foregroundStyle(.green)
          .accessibilityIdentifier("postgres.activity.outcome")
      }
      if let error = model.postgresActivityError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("postgres.activity.error")
      }
    }
    .padding(20)
    .frame(minWidth: 760, minHeight: 520)
    .accessibilityElement(children: .contain)
    .confirmationDialog(
      pendingSignal?.kind == "terminate" ? "Terminate PostgreSQL session?" : "Cancel query?",
      isPresented: Binding(
        get: { pendingSignal != nil },
        set: { if !$0 { pendingSignal = nil } }
      ),
      presenting: pendingSignal
    ) { pending in
      Button(
        pending.kind == "terminate" ? "Terminate PID \(pending.pid)" : "Cancel PID \(pending.pid)",
        role: pending.kind == "terminate" ? .destructive : nil
      ) {
        pendingSignal = nil
        Task { await model.signalPostgresBackend(kind: pending.kind, pid: pending.pid) }
      }
      .accessibilityIdentifier("postgres.activity.confirm")
      Button("Keep Running", role: .cancel) { pendingSignal = nil }
    } message: { pending in
      Text(
        pending.kind == "terminate"
          ? "PostgreSQL will close backend PID \(pending.pid)."
          : "PostgreSQL will request cancellation for PID \(pending.pid).")
    }
  }
}

struct PostgresToolsSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  private var operationActive: Bool {
    model.postgresToolStatus?.phase == "running"
      || model.postgresToolStatus?.phase == "cancel_requested"
  }

  var body: some View {
    @Bindable var model = model
    let target =
      model.activeProfile.map {
        "\($0.name) · \($0.host ?? "unknown host"):\($0.port ?? "?")/\($0.context ?? "postgres")"
      } ?? "Temporary · \(model.formHost):\(model.formPort)/\(model.formDatabase)"
    VStack(alignment: .leading, spacing: 16) {
      HStack {
        Label("PostgreSQL Backup and Restore", systemImage: "externaldrive.badge.timemachine")
          .font(.title2.bold())
        Spacer()
        Button("Close") { model.closePostgresTools() }
          .disabled(operationActive)
          .accessibilityIdentifier("postgres.tools.close")
      }
      Label(target, systemImage: "server.rack")
        .font(.callout)
        .foregroundStyle(.secondary)
        .textSelection(.enabled)
        .accessibilityIdentifier("postgres.tools.target")
      Picker("Operation", selection: $model.postgresToolKind) {
        Text("Backup").tag("dump")
        Text("Restore").tag("restore")
      }
      .pickerStyle(.segmented)
      .disabled(operationActive)
      .accessibilityIdentifier("postgres.tools.kind")
      .onChange(of: model.postgresToolKind) {
        model.postgresToolFileUrl = nil
        model.postgresToolStatus = nil
        Task { await model.probePostgresTool() }
      }
      GroupBox("Client tool") {
        VStack(alignment: .leading, spacing: 8) {
          HStack {
            TextField("Optional absolute tool path", text: $model.postgresToolExplicitPath)
              .textFieldStyle(.roundedBorder)
              .disabled(operationActive)
              .accessibilityIdentifier("postgres.tools.path")
            Button("Check Version") { Task { await model.probePostgresTool() } }
              .disabled(operationActive)
              .accessibilityIdentifier("postgres.tools.probe")
          }
          if let probe = model.postgresToolProbe {
            Label(
              probe.summary,
              systemImage: probe.available ? "checkmark.circle.fill" : "xmark.circle.fill"
            )
            .foregroundStyle(probe.available ? .green : .red)
            .accessibilityIdentifier("postgres.tools.probe-result")
            if let path = probe.path {
              Text(path).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
            }
          }
        }.padding(6)
      }
      GroupBox(model.postgresToolKind == "dump" ? "Backup destination" : "Restore archive") {
        HStack {
          Text(model.postgresToolFileUrl?.path ?? "No archive selected")
            .lineLimit(1).truncationMode(.middle).textSelection(.enabled)
          Spacer()
          Button("Choose…") { model.choosePostgresToolFile() }
            .disabled(operationActive)
            .accessibilityIdentifier("postgres.tools.choose-file")
        }.padding(6)
      }
      GroupBox("Configuration") {
        VStack(alignment: .leading, spacing: 8) {
          Picker("Content", selection: $model.postgresToolContent) {
            Text("Schema and data").tag("all")
            Text("Schema only").tag("schema_only")
            Text("Data only").tag("data_only")
          }
          .disabled(operationActive)
          .accessibilityIdentifier("postgres.tools.content")
          Toggle("Do not restore original ownership", isOn: $model.postgresToolNoOwner)
            .disabled(operationActive)
            .accessibilityIdentifier("postgres.tools.no-owner")
          if model.postgresToolKind == "restore" {
            Toggle("Drop matching objects before restore", isOn: $model.postgresToolClean)
              .disabled(operationActive)
              .accessibilityIdentifier("postgres.tools.clean")
            if model.postgresToolClean {
              Text("Uses --clean with --if-exists. Matching objects may be destroyed.")
                .foregroundStyle(.orange)
            }
          }
        }.padding(6)
      }
      GroupBox("Review") {
        Text(
          model.postgresToolKind == "dump"
            ? "Create a \(model.postgresToolContent.replacingOccurrences(of: "_", with: " ")) PostgreSQL custom-format backup at the selected destination. An incomplete archive is removed if cancelled."
            : "Load \(model.postgresToolContent.replacingOccurrences(of: "_", with: " ")) from the selected archive into the connected database. Restore may execute code chosen by source superusers and overwrite objects or data; use only a trusted archive."
        )
        .foregroundStyle(model.postgresToolKind == "restore" ? .orange : .secondary)
        .padding(6)
      }
      if let status = model.postgresToolStatus {
        HStack {
          if operationActive { ProgressView() }
          Text(
            "\(status.phase.replacingOccurrences(of: "_", with: " ").capitalized): \(status.summary)"
          )
          .accessibilityIdentifier("postgres.tools.status")
          Spacer()
          if operationActive {
            Button("Cancel", role: .destructive) { Task { await model.cancelPostgresTool() } }
              .disabled(status.phase == "cancel_requested")
              .accessibilityIdentifier("postgres.tools.cancel")
          }
        }
      }
      if let error = model.postgresToolError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
          .accessibilityIdentifier("postgres.tools.error")
      }
      HStack {
        Spacer()
        Button(model.postgresToolKind == "dump" ? "Start Backup…" : "Start Restore…") {
          model.requestStartPostgresTool()
        }
        .buttonStyle(.glassProminent)
        .disabled(
          operationActive || model.postgresToolProbe?.available != true
            || model.postgresToolFileUrl == nil
        )
        .accessibilityIdentifier("postgres.tools.start")
      }
    }
    .padding(20)
    .frame(minWidth: 700, minHeight: 500)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(operationActive)
    .confirmationDialog(
      model.postgresToolKind == "dump" ? "Start PostgreSQL backup?" : "Start PostgreSQL restore?",
      isPresented: $model.postgresToolReviewRequested
    ) {
      Button(
        model.postgresToolKind == "dump" ? "Create Backup" : "Restore Database",
        role: model.postgresToolKind == "restore" ? .destructive : nil
      ) { Task { await model.startPostgresTool() } }
      .accessibilityIdentifier("postgres.tools.confirm")
      Button("Cancel", role: .cancel) { model.postgresToolReviewRequested = false }
    } message: {
      Text(
        model.postgresToolKind == "dump"
          ? "Run the checked pg_dump version against the connected PostgreSQL database?"
          : "Run the checked pg_restore version against the connected PostgreSQL database? This can replace database objects and data."
      )
    }
  }
}

