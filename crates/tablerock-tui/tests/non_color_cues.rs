//! Non-color state cue audit: every GridOperationState has a text label.
//!
//! Phase 11 accessibility gate — status must never rely on color alone.

use tablerock_tui::model::grid::GridOperationState;

#[test]
fn every_operation_state_has_nonempty_text_label() {
    let states = [
        GridOperationState::Idle,
        GridOperationState::Queued,
        GridOperationState::Running,
        GridOperationState::Streaming,
        GridOperationState::Completed,
        GridOperationState::CancelRequested,
        GridOperationState::ClientStopped,
        GridOperationState::ServerConfirmedCancelled,
        GridOperationState::CancelUnknown,
        GridOperationState::Cancelled,
        GridOperationState::Failed,
        GridOperationState::Disconnected,
    ];
    for s in states {
        assert!(!s.label().is_empty(), "{s:?}");
        // Labels must not be single-character color codes.
        assert!(s.label().len() > 2, "{s:?}");
    }
}

#[test]
fn cancel_states_are_pairwise_distinct() {
    use std::collections::BTreeSet;
    let labels: BTreeSet<_> = [
        GridOperationState::CancelRequested,
        GridOperationState::ClientStopped,
        GridOperationState::ServerConfirmedCancelled,
        GridOperationState::CancelUnknown,
        GridOperationState::Cancelled,
    ]
    .iter()
    .map(|s| s.label())
    .collect();
    assert_eq!(labels.len(), 5);
}
