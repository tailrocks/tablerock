# Plan 020 — AppKit TextKit editor

Date: 2026-07-19  
SDK: Xcode 26.6 / macOS 26.5 SDK

## Checkpoint

The native query editor now uses `NSTextView` inside `NSScrollView` through
`NSViewRepresentable`. Its coordinator provides two-way model binding while
retaining native undo, responder-chain find behavior, selection, keyboard, and
input-method handling. Rich text, smart quotes, smart dashes, and automatic
replacement are disabled so database statements remain literal.

Swift-to-AppKit updates never replace text storage while an input method owns
marked text. External updates preserve and clamp the selected ranges. The
editor exposes an explicit `SQL editor` accessibility label.

## Evidence

| Gate | Observation |
|------|-------------|
| `./scripts/build-native-app.sh` | PASS with Swift 6 complete concurrency and warnings-as-errors |
| installed-SDK API check | `NSTextViewDelegate`, marked-text, selection, undo, and text replacement controls compile on macOS 26.5 SDK |
| app launch inspection | native shell launches normally after editor integration |
| `./scripts/verify-native-behavior.sh` | PASS on PostgreSQL 18.4, ClickHouse 25.8, Redis 8.0 |
| source audit | no SwiftUI `TextEditor`; SQL remains opaque presentation text and executes through Rust bridge |

## Bounds

The adapter structurally preserves IME composition, but the full multilingual
manual IME matrix remains a Phase 14 evidence item. Syntax highlighting,
completion popup, history/saved/file screens, and AppKit catalog outline remain
open native parity work.
