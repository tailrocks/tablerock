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
    BridgeEventBatch, BridgeEventRecord, BridgeHistoryItem, BridgeNativeWindowIntent,
    BridgePostgresToolProbe, BridgePostgresToolRequest, BridgePostgresToolStatus,
    BridgeProfileDraft, BridgeProfileGroup, BridgeProfileItem, BridgeProfileOrderItem,
    BridgeReconnectAttempt, BridgeReconnectPlan, BridgeSavedFilterPreset, BridgeSavedQueryItem,
    BridgeSessionHealth, BridgeSessionIntent, BridgeSqlFile, BridgeWorkspaceTab, CancelOutcome,
    OpenParams, ShutdownOutcome, SubmitSpec, TableRockBridge,
};
pub use error::BridgeError;
