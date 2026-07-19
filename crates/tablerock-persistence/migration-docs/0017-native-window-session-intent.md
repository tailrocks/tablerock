# 0017 native window session intent

Adds one intent-only row per native SwiftUI `WindowGroup` UUID. Multiple windows
may use the same saved profile without overwriting each other's selected tab or
editor text. The profile foreign key removes window intents when that profile
is deleted.

The JSON uses the existing session-intent validation boundary. Result pages,
cells, operation state, and pending writes remain forbidden. Existing
profile-keyed `session_intent` rows remain for TUI compatibility; no data is
silently copied because one legacy row cannot be assigned safely to an
arbitrary restored native window.
