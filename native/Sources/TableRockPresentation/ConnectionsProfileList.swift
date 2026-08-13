import SwiftUI
import TableRockFeature

private enum ConnectionsNavigatorSelection: Hashable {
  case connections
  case setup
  case profile(Data)
}

/// Stable disconnected navigation. Connection management owns the leading
/// plane; editable values remain in the opaque detail plane or a native sheet.
struct ConnectionsNavigatorPane: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(spacing: 0) {
      List(
        selection: Binding(
          get: {
            switch model.connectionWorkspaceSurface {
            case .connections: ConnectionsNavigatorSelection.connections
            case .setup: ConnectionsNavigatorSelection.setup
            }
          },
          set: { selection in
            switch selection {
            case nil: break
            case .connections: model.showConnectionBrowser()
            case .setup: model.showConnectionSetup()
            case .profile(let id):
              guard let profile = model.profiles.first(where: { $0.idBytes == id }) else { return }
              Task { await model.connect(profile) }
            }
          }
        )
      ) {
        Section("Workspace") {
          Label("Connections", systemImage: "externaldrive.connected.to.line.below")
            .tag(ConnectionsNavigatorSelection.connections)
          Label("Connection Setup", systemImage: "slider.horizontal.3")
            .tag(ConnectionsNavigatorSelection.setup)
        }

        if !model.profiles.isEmpty {
          Section("Recent") {
            ForEach(model.profiles.prefix(8), id: \.idBytes) { profile in
              Label(profile.name, systemImage: connectionEngineSymbol(profile.engine))
                .lineLimit(1)
                .tag(ConnectionsNavigatorSelection.profile(profile.idBytes))
                .accessibilityIdentifier(
                  "sidebar.profile.\(profile.idBytes.hexEncodedString())"
                )
                .accessibilityLabel("Open \(profile.name)")
            }
          }
        }
      }
      .listStyle(.sidebar)
      .accessibilityIdentifier("sidebar.profiles")
      .searchable(text: $model.profileSearch, prompt: "Search connections")
      .task(id: model.profileSearch) {
        try? await Task.sleep(for: .milliseconds(150))
        guard !Task.isCancelled else { return }
        await model.refreshProfilesForConnectionSearch()
      }

      Divider()
      HStack {
        SettingsLink {
          Label("Settings", systemImage: "gearshape")
        }
        .buttonStyle(.plain)
        Spacer()
        Button {
          model.createProfile()
        } label: {
          Image(systemName: "plus")
        }
        .buttonStyle(.plain)
        .help("New connection")
      }
      .font(.caption)
      .padding(.horizontal, 12)
      .frame(height: 40)
    }
    .background(.bar)
  }
}

struct ConnectionWorkspaceView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    switch model.connectionWorkspaceSurface {
    case .connections:
      ConnectionsBrowserView()
    case .setup:
      if let draft = model.editorDraft, model.profileEditorPresentation == .workspace {
        ProfileEditorSheet(
          initialDraft: draft,
          presentation: .workspace,
          onCancel: { model.cancelConnectionSetup() },
          onTest: { tested in await model.testProfileDraft(tested) },
          onSave: { saved in await model.saveAndConnectProfile(saved) }
        )
      } else {
        ProgressView("Preparing connection…")
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .task { model.showConnectionSetup() }
      }
    }
  }
}

private struct ConnectionsBrowserView: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  private var favoriteProfiles: [WorkbenchProfileItem] {
    model.profiles.filter(\.favorite)
  }

  private var otherProfiles: [WorkbenchProfileItem] {
    model.profiles.filter { !$0.favorite }
  }

  var body: some View {
    @Bindable var model = model
    VStack(spacing: 0) {
      HStack(spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text("Connections")
            .font(.title2.weight(.semibold))
          Text("Open a workspace or configure a data source")
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        Spacer()
        Button("New Connection", systemImage: "plus") {
          model.createProfile()
        }
        .buttonStyle(.borderedProminent)
        .accessibilityIdentifier("profile.add")
      }
      .padding(.horizontal, 24)
      .frame(height: 76)
      Divider()

      if model.profilesLoading {
        ProgressView("Loading connections…")
          .frame(maxWidth: .infinity, maxHeight: .infinity)
          .accessibilityIdentifier("connections.loading")
      } else if let profilesError = model.profilesError {
        ContentUnavailableView(
          "Connections failed",
          systemImage: "exclamationmark.triangle",
          description: Text(profilesError)
        )
        .accessibilityIdentifier("connections.error")
      } else if model.profiles.isEmpty {
        ContentUnavailableView {
          Label(
            model.profileSearch.isEmpty ? "No connections" : "No matches",
            systemImage: model.profileSearch.isEmpty ? "tray" : "magnifyingglass"
          )
          .accessibilityIdentifier("connections.empty")
        } description: {
          Text(
            model.profileSearch.isEmpty
              ? "Try the sample database or create a connection."
              : "No saved connection matches this search."
          )
        } actions: {
          if model.profileSearch.isEmpty {
            Button("Try Sample Database") {
              Task { await model.trySampleDatabase() }
            }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("profile.try-sample")
            Button("New Connection…") { model.createProfile() }
            Button("Connect Temporarily…") {
              model.directConnectionPresented = true
            }
            .accessibilityIdentifier("connection.direct.open")
          }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      } else {
        ScrollView {
          VStack(alignment: .leading, spacing: 18) {
            HStack(spacing: 8) {
              Image(systemName: "magnifyingglass")
                .foregroundStyle(.secondary)
              TextField("Search connections", text: $model.profileSearch)
                .textFieldStyle(.plain)
              Text("⌘K")
                .font(.caption)
                .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 12)
            .frame(height: 36)
            .background(Color(nsColor: .controlBackgroundColor), in: .rect(cornerRadius: 9))
            .overlay {
              RoundedRectangle(cornerRadius: 9)
                .stroke(Color(nsColor: .separatorColor), lineWidth: 0.5)
            }

            if !favoriteProfiles.isEmpty {
              ConnectionCardSection(title: "Favorites", profiles: favoriteProfiles)
            }
            if !otherProfiles.isEmpty {
              ConnectionCardSection(
                title: favoriteProfiles.isEmpty ? "Connections" : "All Connections",
                profiles: otherProfiles
              )
            }

            Button {
              model.createProfile()
            } label: {
              VStack(spacing: 9) {
                Image(systemName: "plus.circle")
                  .font(.title2)
                Text("New Connection")
                  .font(.headline)
                Text("PostgreSQL, ClickHouse, Redis, or local SQLite")
                  .font(.caption)
                  .foregroundStyle(.secondary)
              }
              .foregroundStyle(.secondary)
              .frame(maxWidth: 310, minHeight: 142)
              .background(
                Color(nsColor: .controlBackgroundColor).opacity(0.45), in: .rect(cornerRadius: 12)
              )
              .overlay {
                RoundedRectangle(cornerRadius: 12)
                  .stroke(
                    Color(nsColor: .separatorColor),
                    style: StrokeStyle(lineWidth: 1, dash: [5])
                  )
              }
            }
            .buttonStyle(.plain)

            HStack(spacing: 10) {
              Label(
                "Secrets stay in their configured source",
                systemImage: "lock.shield"
              )
              Spacer()
              Button("Connect Temporarily…") {
                model.directConnectionPresented = true
              }
              .accessibilityIdentifier("connection.direct.open")
              Button("Import…") { model.beginConnectionUrlImport() }
                .accessibilityIdentifier("profile.url-import")
              Menu("Manage Groups…") {
                Button {
                  model.beginCreateGroup()
                } label: {
                  Label("Group", systemImage: "folder.badge.plus")
                }
                ForEach(model.profileSections.filter { $0.id != "ungrouped" }) { section in
                  Menu(section.title) {
                    Button {
                      Task { await model.setGroupAlphabetical(section, false) }
                    } label: {
                      Label(
                        "Manual Order",
                        systemImage: section.alphabetical ? "circle" : "checkmark"
                      )
                    }
                    Button {
                      Task { await model.setGroupAlphabetical(section, true) }
                    } label: {
                      Label(
                        "Alphabetical",
                        systemImage: section.alphabetical ? "checkmark" : "circle"
                      )
                    }
                    Divider()
                    Button("Rename Group…") { model.beginRenameGroup(section.title) }
                    Button("Remove Group…", role: .destructive) {
                      model.pendingGroupRemoval = section.title
                    }
                  }
                }
              }
              Button {
                Task { await model.trySampleDatabase() }
              } label: {
                Text("Try Sample")
              }
              .accessibilityIdentifier("profile.try-sample")
            }
            .font(.caption)
            .foregroundStyle(.secondary)

            if let outcome = model.profileActionOutcome {
              Text(outcome)
                .font(.callout)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("profile.action.outcome")
            }
          }
          .padding(24)
        }
      }
    }
    .background(Color(nsColor: .windowBackgroundColor))
  }
}

private struct ConnectionCardSection: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let title: String
  let profiles: [WorkbenchProfileItem]

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(title.uppercased())
        .font(.caption2.weight(.semibold))
        .foregroundStyle(.secondary)

      LazyVGrid(
        columns: [GridItem(.adaptive(minimum: 270, maximum: 360), spacing: 12)],
        spacing: 12
      ) {
        ForEach(profiles, id: \.idBytes) { profile in
          ZStack(alignment: .topTrailing) {
            Button {
              Task { await model.connect(profile) }
            } label: {
              ConnectionCard(profile: profile)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Open \(profile.name)")
            .accessibilityIdentifier("profile.\(profile.idBytes.hexEncodedString())")

            Menu("More", systemImage: "ellipsis") {
              ConnectionActions(profile: profile)
            }
            .menuStyle(.borderlessButton)
            .labelStyle(.iconOnly)
            .accessibilityLabel("Actions for \(profile.name)")
            .padding(13)
          }
          .contextMenu { ConnectionActions(profile: profile) }
        }
      }
    }
  }
}

private struct ConnectionCard: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let profile: WorkbenchProfileItem

  private var state: (word: String, detail: String?) {
    ProfileLiveStatePresentation.parts(from: model.connectionState(profile))
  }

  private var endpoint: String {
    [profile.host, profile.port].compactMap { $0 }.filter { !$0.isEmpty }
      .joined(separator: ":")
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 13) {
      HStack(alignment: .top, spacing: 11) {
        Image(systemName: connectionEngineSymbol(profile.engine))
          .font(.title2)
          .foregroundStyle(model.isActiveProfile(profile) ? Color.accentColor : .secondary)
          .frame(width: 38, height: 38)
          .background(Color.accentColor.opacity(0.09), in: .rect(cornerRadius: 9))
        VStack(alignment: .leading, spacing: 3) {
          Text(profile.name)
            .font(.headline)
            .lineLimit(1)
          Text(ProfileEngineBadge.accessibilityName(profile.engine))
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        Spacer()
        Color.clear.frame(width: 20, height: 20)
          .accessibilityHidden(true)
      }

      Text(endpoint.isEmpty ? (profile.context ?? "Local connection") : endpoint)
        .font(.caption.monospaced())
        .foregroundStyle(.secondary)
        .lineLimit(1)
        .truncationMode(.middle)

      HStack {
        ConnectionEnvironmentBadge(profile: profile)
        Spacer()
        Label(
          state.detail.map { "\(state.word.capitalized) · \($0)" }
            ?? state.word.capitalized,
          systemImage: model.isActiveProfile(profile) ? "circle.fill" : "clock"
        )
        .foregroundStyle(model.isActiveProfile(profile) ? Color.green : .secondary)
        .font(.caption)
      }
    }
    .padding(16)
    .frame(maxWidth: .infinity, minHeight: 146, alignment: .topLeading)
    .background(Color(nsColor: .controlBackgroundColor), in: .rect(cornerRadius: 12))
    .overlay {
      RoundedRectangle(cornerRadius: 12)
        .stroke(Color(nsColor: .separatorColor), lineWidth: 0.5)
    }
    .contentShape(.rect)
    .accessibilityElement(children: .combine)
  }
}

private struct ConnectionEnvironmentBadge: View {
  let profile: WorkbenchProfileItem

  var body: some View {
    let word = (profile.environment ?? "Unspecified").uppercased()
    Label(
      word,
      systemImage: profile.productionWarning ? "exclamationmark.triangle.fill" : "circle.fill"
    )
    .font(.caption2.weight(.semibold))
    .foregroundStyle(profile.productionWarning ? Color.orange : .secondary)
    .padding(.horizontal, 7)
    .padding(.vertical, 3)
    .background(.quaternary.opacity(0.5), in: .capsule)
    .accessibilityLabel("Environment \(word)")
  }
}

private struct ConnectionActions: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  let profile: WorkbenchProfileItem

  private var section: ProfileSection? {
    model.profileSections.first { section in
      section.profiles.contains(where: { $0.idBytes == profile.idBytes })
    }
  }

  var body: some View {
    Button("Connect") { Task { await model.connect(profile) } }
    if model.isActiveProfile(profile) {
      Button("Check Health") { Task { await model.checkActiveHealth() } }
      Button("Reconnect") { Task { await model.reconnectActive() } }
      Button("Disconnect") { Task { await model.disconnectActive() } }
    }
    Button("Edit…") { Task { await model.editProfile(profile) } }
    Button("Duplicate…") { Task { await model.duplicateProfile(profile) } }
    Button("Test") { Task { await model.testProfile(profile) } }
    Button(profile.favorite ? "Remove Favorite" : "Add Favorite") {
      Task { await model.toggleFavorite(profile) }
    }
    if let section {
      Button("Move Up") { Task { await model.move(profile, in: section, offset: -1) } }
        .disabled(!model.canMove(profile, in: section, offset: -1))
      Button("Move Down") { Task { await model.move(profile, in: section, offset: 1) } }
        .disabled(!model.canMove(profile, in: section, offset: 1))
    }
    Divider()
    Button("Remove…", role: .destructive) { model.pendingRemoval = profile }
  }
}

struct TemporaryConnectionSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    @Bindable var model = model
    NavigationStack {
      Form {
        Section("Connection") {
          Picker("Engine", selection: $model.formEngine) {
            Text("PostgreSQL").tag("postgresql")
            Text("ClickHouse").tag("clickhouse")
            Text("Redis").tag("redis")
            Text("SQLite").tag("sqlite")
          }
          TextField(model.formEngine == "sqlite" ? "Path" : "Host", text: $model.formHost)
          TextField("Port", text: $model.formPort)
            .disabled(model.formEngine == "sqlite")
          TextField(
            model.formEngine == "sqlite" ? "File" : "Database",
            text: $model.formDatabase
          )
          TextField("User", text: $model.formUser)
          SecureField("Password", text: $model.formPassword)
        }
        Section("Scope") {
          Label(
            "Temporary connection values are not saved as a profile.",
            systemImage: "lock.shield"
          )
          .foregroundStyle(.secondary)
        }
        if let error = model.connectError {
          Section("Connection Error") {
            Text(error).foregroundStyle(.red).textSelection(.enabled)
          }
        }
      }
      .formStyle(.grouped)
      .navigationTitle("Connect Temporarily")
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button("Cancel") {
            model.directConnectionPresented = false
            dismiss()
          }
        }
        ToolbarItem(placement: .confirmationAction) {
          Button("Connect") { Task { await model.connectByParams() } }
            .buttonStyle(.borderedProminent)
            .accessibilityIdentifier("connection.direct.connect")
            .keyboardShortcut(.defaultAction)
        }
      }
    }
    .frame(minWidth: 560, minHeight: 430)
  }
}

func connectionEngineSymbol(_ engine: String) -> String {
  switch engine.lowercased() {
  case "postgresql", "postgres": "cylinder.split.1x2"
  case "clickhouse": "chart.bar.xaxis"
  case "redis": "square.stack.3d.up.fill"
  case "sqlite": "internaldrive"
  default: "cylinder"
  }
}
