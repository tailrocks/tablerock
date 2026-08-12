import Foundation

@MainActor
extension WorkbenchPresentationStore {
  func showFindReplace() {
    guard queryWorkbenchSelected else { return }
    findPattern = ""
    findReplacement = ""
    findMode = "literal"
    findScope = "document"
    findStatus = nil
    findError = nil
    activeQueryTab.findScopeRange = nil
    activeQueryTab.lastFindMatch = nil
    findReplacePresented = true
  }

  func setFindScope(_ scope: String) {
    findScope = scope
    activeQueryTab.findScopeRange = scope == "selection" ? activeQueryTab.editorSelection : nil
    activeQueryTab.lastFindMatch = nil
    findStatus = nil
    findError = nil
  }

  func resetFindTraversal() {
    activeQueryTab.lastFindMatch = nil
    findStatus = nil
    findError = nil
  }

  func findEditorMatch(backwards: Bool) {
    do {
      let match = try NativeFindReplaceEngine.find(
        in: queryText, pattern: findPattern, mode: findMode,
        scope: try effectiveFindScope(), selection: queryEditorSelection,
        previousMatch: activeQueryTab.lastFindMatch, backwards: backwards)
      guard let match else {
        findStatus = "No match"
        findError = nil
        activeQueryTab.lastFindMatch = nil
        return
      }
      queryEditorSelection = match
      activeQueryTab.lastFindMatch = match
      findStatus = "Match at character \(match.location + 1)"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  func replaceEditorMatch() {
    do {
      let outcome = try NativeFindReplaceEngine.replaceCurrent(
        in: queryText, pattern: findPattern, replacement: findReplacement,
        mode: findMode, scope: try effectiveFindScope(), selection: queryEditorSelection)
      guard let outcome else {
        findEditorMatch(backwards: false)
        return
      }
      queryText = outcome.text
      queryEditorSelection = outcome.selection
      updateFindScope(afterReplacing: outcome.replacedRange, delta: outcome.delta)
      activeQueryTab.lastFindMatch = nil
      findStatus = "Replaced 1 match"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  func replaceAllEditorMatches() {
    do {
      let outcome = try NativeFindReplaceEngine.replaceAll(
        in: queryText, pattern: findPattern, replacement: findReplacement,
        mode: findMode, scope: try effectiveFindScope())
      queryText = outcome.text
      queryEditorSelection = outcome.selection
      if findScope == "selection" { activeQueryTab.findScopeRange = outcome.selection }
      activeQueryTab.lastFindMatch = nil
      findStatus = "Replaced \(outcome.count) match\(outcome.count == 1 ? "" : "es")"
      findError = nil
    } catch {
      findError = String(describing: error)
      findStatus = nil
    }
  }

  private func effectiveFindScope() throws -> NSRange {
    let whole = NSRange(location: 0, length: (queryText as NSString).length)
    guard findScope == "selection", let selected = activeQueryTab.findScopeRange else {
      return whole
    }
    let location = min(selected.location, whole.length)
    let scope = NSRange(location: location, length: min(selected.length, whole.length - location))
    guard scope.length > 0 else { throw NativeFindReplaceError.invalidScope }
    return scope
  }

  private func updateFindScope(afterReplacing range: NSRange, delta: Int) {
    guard findScope == "selection", var scope = activeQueryTab.findScopeRange,
      range.location >= scope.location, NSMaxRange(range) <= NSMaxRange(scope)
    else { return }
    scope.length = max(0, scope.length + delta)
    activeQueryTab.findScopeRange = scope
  }
}
