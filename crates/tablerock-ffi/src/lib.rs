//! Synchronous coarse UniFFI facade over the TableRock engine.
//!
//! Swift never holds Tokio handles or driver objects. Every entry point is
//! synchronous, panics are contained, and pages cross only as versioned
//! columnar `Vec<u8>` payloads (`ResultPage::encode_v1`).

#![allow(unsafe_code)]

uniffi::setup_scaffolding!();

mod bridge;
mod error;
mod ids;
mod page_limits;
mod runtime;

pub use bridge::{
    ApplyOutcome, BridgeBrowseFilter, BridgeBrowseSort, BridgeConnectionTestReport,
    BridgeCsvImportPreview, BridgeCsvImportProgress, BridgeCsvImportReview, BridgeCsvRow,
    BridgeDdlChangeRequest, BridgeDdlChangeReview, BridgeEventBatch, BridgeEventRecord,
    BridgeHistoryItem, BridgeNamedParameterPlan, BridgeNativeWindowIntent, BridgePostgresToolProbe,
    BridgePostgresToolRequest, BridgePostgresToolStatus, BridgeProfileDraft, BridgeProfileGroup,
    BridgeProfileItem, BridgeProfileOrderItem, BridgeQueryParameter, BridgeReconnectAttempt,
    BridgeReconnectPlan, BridgeRedisSubscriptionStatus, BridgeRoleChangeRequest,
    BridgeRoleChangeReview, BridgeRoleMembership, BridgeRolePrivilege, BridgeRoleSnapshot,
    BridgeSavedFilterPreset, BridgeSavedQueryItem, BridgeSessionHealth, BridgeSessionIntent,
    BridgeSqlFile, BridgeTableOperationRequest, BridgeTableOperationReview, BridgeWorkspaceTab,
    CancelOutcome, OpenParams, ShutdownOutcome, SubmitSpec, TableRockBridge,
};
pub use error::BridgeError;
