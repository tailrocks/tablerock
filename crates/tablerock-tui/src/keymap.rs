//! Root keyboard actions and their single dispatch/hint registry.

use termrock::{
    input::KeyCode,
    keymap::{KeyBinding, KeyChord, Keymap, Visibility},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKeyAction {
    FocusNext,
    FocusPrevious,
    ActionPrevious,
    ActionNext,
    Activate,
    Quit,
}

static BINDINGS: &[KeyBinding<ShellKeyAction>] = &[
    KeyBinding::borrowed(
        &[KeyChord::plain(KeyCode::Enter)],
        ShellKeyAction::Activate,
        Some("Activate"),
        Visibility::Shown,
        None,
    ),
    KeyBinding::borrowed(
        &[KeyChord::plain(KeyCode::Tab)],
        ShellKeyAction::FocusNext,
        Some("Next focus"),
        Visibility::Shown,
        None,
    ),
    KeyBinding::borrowed(
        &[
            KeyChord::shift(KeyCode::Tab),
            KeyChord::shift(KeyCode::BackTab),
            KeyChord::plain(KeyCode::BackTab),
        ],
        ShellKeyAction::FocusPrevious,
        Some("Previous focus"),
        Visibility::Shown,
        None,
    ),
    KeyBinding::borrowed(
        &[KeyChord::plain(KeyCode::Left)],
        ShellKeyAction::ActionPrevious,
        Some("Choose action"),
        Visibility::Shown,
        Some("←/→"),
    ),
    KeyBinding::borrowed(
        &[KeyChord::plain(KeyCode::Right)],
        ShellKeyAction::ActionNext,
        None,
        Visibility::HiddenAlias,
        None,
    ),
    KeyBinding::borrowed(
        &[KeyChord::ctrl(KeyCode::Char('c'))],
        ShellKeyAction::Quit,
        Some("Quit"),
        Visibility::Shown,
        None,
    ),
];

#[must_use]
pub const fn default_keymap() -> Keymap<ShellKeyAction> {
    Keymap::from_static(BINDINGS)
}
