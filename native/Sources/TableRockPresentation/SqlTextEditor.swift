import AppKit
import SwiftUI
import TableRockFeature

struct SqlTextEditor: NSViewRepresentable {
  @Binding var text: String
  @Binding var selection: NSRange
  var isRunning: Bool = false

  func makeCoordinator() -> Coordinator { Coordinator(text: $text, selection: $selection) }

  func makeNSView(context: Context) -> NSScrollView {
    let editor = NSTextView()
    editor.delegate = context.coordinator
    editor.isEditable = true
    editor.isSelectable = true
    editor.isRichText = false
    editor.importsGraphics = false
    editor.allowsUndo = true
    editor.isAutomaticQuoteSubstitutionEnabled = false
    editor.isAutomaticDashSubstitutionEnabled = false
    editor.isAutomaticTextReplacementEnabled = false
    editor.isAutomaticSpellingCorrectionEnabled = false
    editor.isContinuousSpellCheckingEnabled = false
    editor.usesFindBar = true
    editor.isIncrementalSearchingEnabled = true
    editor.font = NSFont.monospacedSystemFont(
      ofSize: NSFont.systemFontSize, weight: .regular)
    // Gutter clearance for line numbers (drawn in ruler).
    editor.textContainerInset = NSSize(width: 4, height: 8)
    editor.drawsBackground = true
    editor.backgroundColor = .textBackgroundColor
    editor.maxSize = NSSize(
      width: CGFloat.greatestFiniteMagnitude,
      height: CGFloat.greatestFiniteMagnitude)
    editor.isHorizontallyResizable = true
    editor.textContainer?.widthTracksTextView = true
    editor.textContainer?.containerSize = NSSize(
      width: CGFloat.greatestFiniteMagnitude,
      height: CGFloat.greatestFiniteMagnitude)
    editor.string = text
    editor.setAccessibilityEnabled(true)
    editor.setAccessibilityLabel("SQL editor")
    editor.setAccessibilityIdentifier("query.editor")

    let scroll = NSScrollView()
    scroll.documentView = editor
    scroll.drawsBackground = true
    scroll.backgroundColor = .textBackgroundColor
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    scroll.borderType = .lineBorder
    scroll.focusRingType = .exterior
    scroll.hasVerticalRuler = true
    scroll.rulersVisible = true
    let ruler = SqlLineNumberRulerView(scrollView: scroll, orientation: .verticalRuler)
    ruler.clientView = editor
    scroll.verticalRulerView = ruler
    context.coordinator.ruler = ruler
    return scroll
  }

  func updateNSView(_ scroll: NSScrollView, context: Context) {
    guard let editor = scroll.documentView as? NSTextView else { return }
    context.coordinator.text = $text
    context.coordinator.selection = $selection
    // Never replace storage while an input method owns marked text.
    guard !editor.hasMarkedText() else { return }
    if editor.string != text {
      let selectedRanges = editor.selectedRanges
      editor.string = text
      let maximum = (text as NSString).length
      editor.selectedRanges = selectedRanges.map { value in
        let range = value.rangeValue
        return NSValue(
          range: NSRange(
            location: min(range.location, maximum),
            length: min(range.length, max(0, maximum - min(range.location, maximum)))
          ))
      }
      context.coordinator.ruler?.needsDisplay = true
    }
    let maximum = (text as NSString).length
    let requested = NSRange(
      location: min(selection.location, maximum),
      length: min(selection.length, max(0, maximum - min(selection.location, maximum))))
    if editor.selectedRange() != requested {
      editor.setSelectedRange(requested)
      editor.scrollRangeToVisible(requested)
    }
    // Soft running cue: keep editable for cancel/edit, dim slightly via text color.
    editor.textColor = isRunning ? NSColor.secondaryLabelColor : NSColor.labelColor
    context.coordinator.ruler?.needsDisplay = true
  }

  @MainActor
  final class Coordinator: NSObject, NSTextViewDelegate {
    var text: Binding<String>
    var selection: Binding<NSRange>
    weak var ruler: SqlLineNumberRulerView?

    init(text: Binding<String>, selection: Binding<NSRange>) {
      self.text = text
      self.selection = selection
    }

    func textDidChange(_ notification: Notification) {
      guard let editor = notification.object as? NSTextView else { return }
      text.wrappedValue = editor.string
      ruler?.needsDisplay = true
    }

    func textViewDidChangeSelection(_ notification: Notification) {
      guard let editor = notification.object as? NSTextView else { return }
      selection.wrappedValue = editor.selectedRange()
      ruler?.needsDisplay = true
    }
  }
}

/// Vertical line-number gutter for the SQL editor (opaque content chrome).
final class SqlLineNumberRulerView: NSRulerView {
  private let gutterWidth: CGFloat = 36

  override init(scrollView: NSScrollView?, orientation: NSRulerView.Orientation) {
    super.init(scrollView: scrollView, orientation: orientation)
    ruleThickness = gutterWidth
    clientView = scrollView?.documentView
  }

  @available(*, unavailable)
  required init(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  override func drawHashMarksAndLabels(in rect: NSRect) {
    guard let textView = clientView as? NSTextView,
      let layoutManager = textView.layoutManager,
      let textContainer = textView.textContainer
    else { return }

    NSColor.controlBackgroundColor.setFill()
    bounds.fill()

    let selected = textView.selectedRange()
    let caret = SqlEditorMetrics.caret(text: textView.string, selection: selected)
    let ns = textView.string as NSString
    let length = ns.length
    let attrs: [NSAttributedString.Key: Any] = [
      .font: NSFont.monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize, weight: .regular),
      .foregroundColor: NSColor.secondaryLabelColor,
    ]
    let activeAttrs: [NSAttributedString.Key: Any] = [
      .font: NSFont.monospacedDigitSystemFont(
        ofSize: NSFont.smallSystemFontSize, weight: .semibold),
      .foregroundColor: NSColor.labelColor,
    ]

    if length == 0 {
      let label = "1" as NSString
      let size = label.size(withAttributes: activeAttrs)
      label.draw(
        at: NSPoint(x: ruleThickness - size.width - 6, y: textView.textContainerInset.height),
        withAttributes: activeAttrs)
      return
    }

    var line = 1
    var index = 0
    let visible = textView.visibleRect
    while index < length {
      let lineRange = ns.lineRange(for: NSRange(location: index, length: 0))
      let glyphRange = layoutManager.glyphRange(
        forCharacterRange: lineRange, actualCharacterRange: nil)
      var lineRect = layoutManager.boundingRect(forGlyphRange: glyphRange, in: textContainer)
      lineRect.origin.x += textView.textContainerOrigin.x
      lineRect.origin.y += textView.textContainerOrigin.y
      if lineRect.maxY >= visible.minY, lineRect.minY <= visible.maxY {
        let y = lineRect.minY - visible.minY
        let label = "\(line)" as NSString
        let used = line == caret.line ? activeAttrs : attrs
        let size = label.size(withAttributes: used)
        let point = NSPoint(
          x: ruleThickness - size.width - 6,
          y: y + max(0, (lineRect.height - size.height) / 2))
        label.draw(at: point, withAttributes: used)
      }
      let next = lineRange.location + lineRange.length
      if next <= index { break }
      index = next
      line += 1
      if line > 100_000 { break }
    }
  }
}
