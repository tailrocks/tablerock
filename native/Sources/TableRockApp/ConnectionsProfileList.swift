import SwiftUI

struct ConnectionsProfileList: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    List {
      ForEach(model.profileSections) { section in
        Section {
          if !model.collapsedProfileGroups.contains(section.id) {
            ForEach(section.profiles, id: \.idBytes) { profile in
              let active = model.isActiveProfile(profile)
              HStack(spacing: 4) {
                Button {
                  Task { await model.connect(profile) }
                } label: {
                  ProfileRow(
                    profile: profile,
                    connectionState: model.connectionState(profile),
                    isActive: active
                  )
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier(
                  "profile.\(profile.idBytes.hexEncodedString())"
                )
                Menu {
                  Button("Connect") { Task { await model.connect(profile) } }
                  if active {
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
                  Button("Move Up") {
                    Task { await model.move(profile, in: section, offset: -1) }
                  }
                  .disabled(!model.canMove(profile, in: section, offset: -1))
                  Button("Move Down") {
                    Task { await model.move(profile, in: section, offset: 1) }
                  }
                  .disabled(!model.canMove(profile, in: section, offset: 1))
                  Divider()
                  Button("Remove…", role: .destructive) {
                    model.pendingRemoval = profile
                  }
                } label: {
                  Image(systemName: "ellipsis.circle")
                }
                .menuStyle(.borderlessButton)
                .accessibilityLabel("Actions for \(profile.name)")
              }
              .listRowBackground(
                active
                  ? Color.accentColor.opacity(0.12)
                  : Color.clear
              )
              .contextMenu {
                Button("Connect") { Task { await model.connect(profile) } }
                if profile.connected {
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
                Button("Move Up") {
                  Task { await model.move(profile, in: section, offset: -1) }
                }
                .disabled(!model.canMove(profile, in: section, offset: -1))
                Button("Move Down") {
                  Task { await model.move(profile, in: section, offset: 1) }
                }
                .disabled(!model.canMove(profile, in: section, offset: 1))
                Divider()
                Button("Remove…", role: .destructive) {
                  model.pendingRemoval = profile
                }
              }
            }
          }
        } header: {
          HStack {
            Button {
              if model.collapsedProfileGroups.contains(section.id) {
                model.collapsedProfileGroups.remove(section.id)
              } else {
                model.collapsedProfileGroups.insert(section.id)
              }
            } label: {
              Label(
                "\(section.title) (\(section.profiles.count))",
                systemImage: model.collapsedProfileGroups.contains(section.id)
                  ? "chevron.right" : "chevron.down"
              )
              .font(.caption.weight(.semibold))
              .textCase(.uppercase)
            }
            .buttonStyle(.plain)
            Spacer()
            if section.id != "ungrouped" {
              Menu {
                Button {
                  Task { await model.setGroupAlphabetical(section, false) }
                } label: {
                  Label(
                    "Manual Order",
                    systemImage: section.alphabetical ? "circle" : "checkmark")
                }
                Button {
                  Task { await model.setGroupAlphabetical(section, true) }
                } label: {
                  Label(
                    "Alphabetical",
                    systemImage: section.alphabetical ? "checkmark" : "circle")
                }
                Divider()
                Button("Rename Group…") {
                  model.beginRenameGroup(section.title)
                }
                Button("Remove Group…", role: .destructive) {
                  model.pendingGroupRemoval = section.title
                }
              } label: {
                Image(systemName: "ellipsis")
              }
              .menuStyle(.borderlessButton)
              .accessibilityLabel("Actions for group \(section.title)")
            }
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
      await model.refreshProfiles()
    }
    .safeAreaInset(edge: .bottom) {
      GlassEffectContainer {
        HStack(spacing: 8) {
          Button {
            model.createProfile()
          } label: {
            Label("New", systemImage: "plus")
          }
          .buttonStyle(.glass)
          .accessibilityIdentifier("profile.add")
          Button {
            Task { await model.trySampleDatabase() }
          } label: {
            Label("Sample", systemImage: "cylinder.split.1x2")
          }
          .buttonStyle(.glassProminent)
          .accessibilityIdentifier("profile.try-sample")
          Button {
            model.beginCreateGroup()
          } label: {
            Label("Group", systemImage: "folder.badge.plus")
          }
          .buttonStyle(.glass)
          Button {
            model.beginConnectionUrlImport()
          } label: {
            Label("URL", systemImage: "link.badge.plus")
          }
          .buttonStyle(.glass)
          .accessibilityIdentifier("profile.url-import")
          Spacer(minLength: 0)
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
      }
    }
    .overlay {
      if model.profilesLoading {
        ProgressView("Loading connections…")
          .accessibilityIdentifier("connections.loading")
      } else if let profilesError = model.profilesError {
        ContentUnavailableView(
          "Connections failed",
          systemImage: "exclamationmark.triangle",
          description: Text(profilesError)
        )
        .accessibilityIdentifier("connections.error")
      } else if model.profiles.isEmpty && model.sessionHex == nil
        && (!model.profileSearch.isEmpty || model.profileGroups.isEmpty)
      {
        ContentUnavailableView {
          Label(
            model.profileSearch.isEmpty ? "No connections" : "No matches",
            systemImage: model.profileSearch.isEmpty ? "tray" : "magnifyingglass")
        } description: {
          Text(
            model.profileSearch.isEmpty
              ? "Try the sample database or create a connection."
              : "No saved connection matches this search.")
        } actions: {
          if model.profileSearch.isEmpty {
            Button {
              Task { await model.trySampleDatabase() }
            } label: {
              Text("Try Sample Database")
            }
            .buttonStyle(.glassProminent)
            .controlSize(.large)
            .accessibilityIdentifier("profile.try-sample")
            Button("New Connection…") {
              model.createProfile()
            }
            .buttonStyle(.glass)
          }
        }
        .accessibilityIdentifier("connections.empty")
      }
    }
  }
}
