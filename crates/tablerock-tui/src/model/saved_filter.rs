//! Shared saved-filter contract re-exported for the TUI presentation model.

pub use tablerock_core::{
    SavedFilterCondition, SavedFilterLibrary, SavedFilterPreset, fuzzy_score, is_safe_preset_name,
    rank_preset_names, resolve_preset_name, should_auto_reconnect,
};
