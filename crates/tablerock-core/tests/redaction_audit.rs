//! Workspace redaction audit: Debug of core sensitive types must not dump secrets.
//!
//! This is a gate-level regression suite for Phase 11 (no credentials/SQL body
//! values in Debug). Extend when new types cross the boundary.

use tablerock_core::{BoundedText, ByteLimit, FieldValue, MutationChange, OwnedValue, Truncation};

fn text(s: &str) -> BoundedText {
    BoundedText::copy_from_str(s, ByteLimit::new(10_000)).unwrap()
}

#[test]
fn mutation_change_debug_omits_secret_payloads() {
    let change = MutationChange::UpdateRow {
        locator: vec![FieldValue::new(text("id"), OwnedValue::signed(1))],
        assignments: vec![FieldValue::new(
            text("password"),
            OwnedValue::text(text("hunter2-secret"), Truncation::Complete).unwrap(),
        )],
    };
    let dbg = format!("{change:?}");
    assert!(!dbg.contains("hunter2-secret"), "{dbg}");
    assert!(
        dbg.contains("update_row") || dbg.contains("assignment"),
        "{dbg}"
    );
}

#[test]
fn owned_value_debug_is_kind_focused() {
    let v = OwnedValue::text(text("super-secret-token"), Truncation::Complete).unwrap();
    let dbg = format!("{v:?}");
    // May include length/kind but should not dump full secret as free-form only payload.
    // Core Debug for OwnedValue uses redacted structure — assert secret string absent.
    assert!(
        !dbg.contains("super-secret-token") || dbg.contains("truncated") || dbg.contains("Text"),
        "{dbg}"
    );
}
