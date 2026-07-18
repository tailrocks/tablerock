//! TableRock's root terminal application.

pub mod effect;
pub mod keymap;
pub mod message;
pub mod model;
pub mod reconnect;
pub mod subscriptions;
pub mod update;
pub mod view;

pub use effect::{
    CatalogLevelSpec, ConnectionDraft, Effect, EngineKind, PasswordSourceSpec,
    ProfileListFilterSpec, ProfileRef, RequestToken, TlsModeSpec,
};
pub use keymap::{ShellKeyAction, default_keymap};
pub use message::{EngineMsg, MAX_PASTE_BYTES, Message, PasteText, ProfilesMsg};
pub use model::catalog::{CatalogModel, CatalogNodeProjection, CatalogNodeStatus};
pub use model::grid::{
    CellDistinction, DataGridModel, GridOperationState, GridRowTotal, ProjectedCell,
    distinction_from_kind_label,
};
pub use model::completion::{CompletionCandidateView, CompletionSession, StaleCompletion};
pub use model::query_editor::{QueryEditorModel, StatementSpanView};
pub use model::{
    ActionId, FocusRegion, LayoutMode, Model, PasswordPrompt, Screen, ScrollDirection,
    SessionFacts, ShellTarget,
    profiles::{FailureProjection, LiveConnectionState, ProfileListState, ProfileRowProjection},
};
pub use reconnect::{next_backoff_ms, stop_on_failure_label};
pub use update::{Update, update};
pub use view::{ShellGeometry, ShellView};
