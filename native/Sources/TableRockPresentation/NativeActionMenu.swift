import AppKit
import SwiftUI

enum NativeActionMenuEntry {
  case command(
    title: String,
    systemImage: String? = nil,
    accessibilityLabel: String? = nil,
    identifier: String? = nil,
    isEnabled: Bool = true,
    state: NSControl.StateValue = .off,
    action: @MainActor () -> Void
  )
  case separator
  case section(String)
}

/// AppKit-owned action menu. SwiftUI `Menu` currently exposes actionless
/// `AXMenuButton` elements on macOS 26; `NSPopUpButton` supplies the native
/// target/action and accessibility contract directly.
struct NativeActionMenu: NSViewRepresentable {
  let title: String
  let systemImage: String?
  let accessibilityLabel: String
  var accessibilityHint: String?
  var identifier: String?
  var isEnabled = true
  let entries: [NativeActionMenuEntry]

  func makeCoordinator() -> Coordinator { Coordinator() }

  func makeNSView(context: Context) -> NSPopUpButton {
    let button = NSPopUpButton(frame: .zero, pullsDown: true)
    button.controlSize = .small
    button.bezelStyle = .accessoryBarAction
    button.imagePosition = .imageLeading
    button.setContentHuggingPriority(.required, for: .horizontal)
    configure(button, coordinator: context.coordinator)
    return button
  }

  func updateNSView(_ button: NSPopUpButton, context: Context) {
    configure(button, coordinator: context.coordinator)
  }

  private func configure(_ button: NSPopUpButton, coordinator: Coordinator) {
    let menu = NSMenu()
    menu.autoenablesItems = false

    let header = NSMenuItem(title: title, action: nil, keyEquivalent: "")
    header.image = systemImage.flatMap {
      NSImage(systemSymbolName: $0, accessibilityDescription: nil)
    }
    menu.addItem(header)

    coordinator.actions.removeAll(keepingCapacity: true)
    var commandIndex = 0
    for entry in entries {
      switch entry {
      case .separator:
        menu.addItem(.separator())
      case .section(let title):
        menu.addItem(.sectionHeader(title: title))
      case .command(
        let title,
        let systemImage,
        let accessibilityLabel,
        let identifier,
        let isEnabled,
        let state,
        let action
      ):
        let item = NSMenuItem(
          title: title,
          action: #selector(Coordinator.performCommand(_:)),
          keyEquivalent: ""
        )
        item.target = coordinator
        item.tag = commandIndex
        item.isEnabled = isEnabled
        item.state = state
        item.image = systemImage.flatMap {
          NSImage(systemSymbolName: $0, accessibilityDescription: nil)
        }
        item.setAccessibilityLabel(accessibilityLabel ?? title)
        if let identifier { item.setAccessibilityIdentifier(identifier) }
        coordinator.actions[commandIndex] = action
        commandIndex += 1
        menu.addItem(item)
      }
    }

    button.menu = menu
    button.isEnabled = isEnabled
    button.setAccessibilityLabel(accessibilityLabel)
    button.setAccessibilityHelp(accessibilityHint)
    button.setAccessibilityIdentifier(identifier)
  }

  @MainActor
  final class Coordinator: NSObject {
    var actions: [Int: @MainActor () -> Void] = [:]

    @objc func performCommand(_ sender: NSMenuItem) {
      actions[sender.tag]?()
    }
  }
}
