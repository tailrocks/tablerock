//! Optional Vim-like mode layer over neutral editor state (keymap only).
//!
//! TermRock TextArea is not forked — this module tracks mode and maps
//! intents for TableRock keymaps.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VimMode {
    #[default]
    Insert,
    Normal,
}

impl VimMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Normal => "normal",
        }
    }

    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::Insert => Self::Normal,
            Self::Normal => Self::Insert,
        }
    }
}

/// Intent derived from a key in the current mode (no I/O).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VimIntent {
    /// Leave handling to the underlying TextArea insert path.
    PassThrough,
    EnterNormal,
    EnterInsert,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    DeleteLine,
    Undo,
    Ignored,
}

/// Map a character key under the current mode.
#[must_use]
pub fn map_char(mode: VimMode, ch: char, ctrl: bool) -> (VimMode, VimIntent) {
    if ctrl {
        return (mode, VimIntent::PassThrough);
    }
    match mode {
        VimMode::Insert => {
            if ch == '\x1b' {
                // Escape handled as KeyCode::Esc by caller usually.
                (VimMode::Normal, VimIntent::EnterNormal)
            } else {
                (VimMode::Insert, VimIntent::PassThrough)
            }
        }
        VimMode::Normal => match ch {
            'i' => (VimMode::Insert, VimIntent::EnterInsert),
            'h' => (mode, VimIntent::MoveLeft),
            'l' => (mode, VimIntent::MoveRight),
            'k' => (mode, VimIntent::MoveUp),
            'j' => (mode, VimIntent::MoveDown),
            'd' => (mode, VimIntent::DeleteLine),
            'u' => (mode, VimIntent::Undo),
            _ => (mode, VimIntent::Ignored),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_passthrough_and_normal_motions() {
        let (m, i) = map_char(VimMode::Insert, 'a', false);
        assert_eq!(m, VimMode::Insert);
        assert_eq!(i, VimIntent::PassThrough);
        let (m, i) = map_char(VimMode::Normal, 'i', false);
        assert_eq!(m, VimMode::Insert);
        assert_eq!(i, VimIntent::EnterInsert);
        let (_, i) = map_char(VimMode::Normal, 'h', false);
        assert_eq!(i, VimIntent::MoveLeft);
    }

    #[test]
    fn mode_toggle_labels() {
        assert_eq!(VimMode::Insert.toggle(), VimMode::Normal);
        assert_eq!(VimMode::Normal.label(), "normal");
    }
}
