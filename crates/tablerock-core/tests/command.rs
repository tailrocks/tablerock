use tablerock_core::{
    BudgetField, CommandBudget, CommandBudgetError, CommandBudgetLimits, CommandBuildError,
    CommandEnvelope, CommandIntent, CommandSafety, CommandScope, ContextId, IdParts,
    MAX_STATEMENT_BYTES, OperationId, OperationScope, PageRequest, ProfileId, RedactionClass,
    RequestId, ResultId, Revision, SessionId, StatementText, StatementTextError,
};

fn limits() -> CommandBudgetLimits {
    CommandBudgetLimits::new(30_000, 256, 8 * 1024 * 1024, 500).unwrap()
}

fn id<T>(constructor: impl FnOnce(IdParts) -> Result<T, tablerock_core::IdDecodeError>) -> T {
    constructor(IdParts::new(0, 1).unwrap()).unwrap()
}

fn context_scope() -> CommandScope {
    CommandScope::Context(OperationScope::new(
        id(ProfileId::from_parts),
        id(SessionId::from_parts),
        id(ContextId::from_parts),
    ))
}

fn budget() -> tablerock_core::ValidatedCommandBudget {
    CommandBudget::new(5_000, 32, 1024 * 1024, 200)
        .unwrap()
        .validate(limits())
        .unwrap()
}

#[test]
fn command_budget_requires_finite_nonzero_values_within_owner_limits() {
    let budget = CommandBudget::new(5_000, 32, 1024 * 1024, 200)
        .unwrap()
        .validate(limits())
        .unwrap();
    assert_eq!(budget.max_duration_ms(), 5_000);
    assert_eq!(budget.max_event_count(), 32);
    assert_eq!(budget.max_response_bytes(), 1024 * 1024);
    assert_eq!(budget.max_page_rows(), 200);

    assert_eq!(
        CommandBudget::new(0, 1, 1, 1),
        Err(CommandBudgetError::ZeroLimit {
            field: BudgetField::Duration
        })
    );
    assert_eq!(
        CommandBudget::new(30_001, 1, 1, 1)
            .unwrap()
            .validate(limits()),
        Err(CommandBudgetError::LimitExceeded {
            field: BudgetField::Duration,
            actual: 30_001,
            limit: 30_000,
        })
    );

    for (candidate, field) in [
        (CommandBudget::new(0, 1, 1, 1), BudgetField::Duration),
        (CommandBudget::new(1, 0, 1, 1), BudgetField::EventCount),
        (CommandBudget::new(1, 1, 0, 1), BudgetField::ResponseBytes),
        (CommandBudget::new(1, 1, 1, 0), BudgetField::PageRows),
    ] {
        assert_eq!(candidate, Err(CommandBudgetError::ZeroLimit { field }));
    }

    for (candidate, field, actual, limit) in [
        (
            CommandBudget::new(30_001, 1, 1, 1).unwrap(),
            BudgetField::Duration,
            30_001,
            30_000,
        ),
        (
            CommandBudget::new(1, 257, 1, 1).unwrap(),
            BudgetField::EventCount,
            257,
            256,
        ),
        (
            CommandBudget::new(1, 1, 8 * 1024 * 1024 + 1, 1).unwrap(),
            BudgetField::ResponseBytes,
            8 * 1024 * 1024 + 1,
            8 * 1024 * 1024,
        ),
        (
            CommandBudget::new(1, 1, 1, 501).unwrap(),
            BudgetField::PageRows,
            501,
            500,
        ),
    ] {
        assert_eq!(
            candidate.validate(limits()),
            Err(CommandBudgetError::LimitExceeded {
                field,
                actual,
                limit,
            })
        );
    }
}

#[test]
fn every_intent_has_one_explicit_scope_shape() {
    let profile_id = id(ProfileId::from_parts);
    let session_id = id(SessionId::from_parts);
    let scopes = [
        CommandScope::Application,
        CommandScope::Profile(profile_id),
        CommandScope::Session {
            profile_id,
            session_id,
        },
        context_scope(),
    ];
    let intents_and_required_scope = [
        (CommandIntent::Shutdown, 0),
        (CommandIntent::TestProfile, 1),
        (CommandIntent::Connect, 1),
        (CommandIntent::Disconnect, 2),
        (CommandIntent::RefreshCatalog, 3),
        (
            CommandIntent::FetchPage(
                PageRequest::new(id(ResultId::from_parts), Revision::INITIAL, 0, 1).unwrap(),
            ),
            3,
        ),
        (
            CommandIntent::Execute {
                statement: StatementText::new("select 1").unwrap(),
            },
            3,
        ),
        (
            CommandIntent::Cancel {
                operation_id: id(OperationId::from_parts),
            },
            3,
        ),
    ];
    for (intent, required_scope) in intents_and_required_scope {
        for (scope_index, scope) in scopes.into_iter().enumerate() {
            let result = CommandEnvelope::new(
                id(RequestId::from_parts),
                scope,
                Revision::INITIAL,
                budget(),
                None,
                intent.clone(),
            );
            if scope_index == required_scope {
                assert!(result.is_ok(), "valid scope {scope_index} rejected");
            } else {
                assert_eq!(result, Err(CommandBuildError::ScopeMismatch));
            }
        }
    }
}

#[test]
fn command_envelope_derives_scope_safety_and_redaction_from_typed_intent() {
    let command = CommandEnvelope::new(
        id(RequestId::from_parts),
        context_scope(),
        Revision::from_wire_u64(7),
        budget(),
        None,
        CommandIntent::RefreshCatalog,
    )
    .unwrap();

    assert_eq!(command.schema_version(), CommandEnvelope::SCHEMA_VERSION);
    assert_eq!(command.expected_revision(), Revision::from_wire_u64(7));
    assert_eq!(command.safety(), CommandSafety::ReadOnly);
    assert_eq!(command.redaction(), RedactionClass::MetadataOnly);
}

#[test]
fn command_envelope_rejects_wrong_scope_excessive_page_and_unknown_version() {
    let profile_scope = CommandScope::Profile(id(ProfileId::from_parts));
    assert_eq!(
        CommandEnvelope::new(
            id(RequestId::from_parts),
            profile_scope,
            Revision::INITIAL,
            budget(),
            None,
            CommandIntent::Disconnect,
        ),
        Err(CommandBuildError::ScopeMismatch)
    );

    assert_eq!(
        CommandEnvelope::new(
            id(RequestId::from_parts),
            context_scope(),
            Revision::INITIAL,
            budget(),
            None,
            CommandIntent::FetchPage(
                PageRequest::new(id(ResultId::from_parts), Revision::INITIAL, 0, 201).unwrap(),
            ),
        ),
        Err(CommandBuildError::PageRowsExceedBudget {
            requested: 201,
            limit: 200,
        })
    );

    assert_eq!(
        PageRequest::new(id(ResultId::from_parts), Revision::INITIAL, 0, 0),
        Err(CommandBuildError::ZeroPageRows)
    );
    assert_eq!(
        PageRequest::new(id(ResultId::from_parts), Revision::INITIAL, u64::MAX, 1),
        Err(CommandBuildError::PageRangeOverflow)
    );

    assert_eq!(
        CommandEnvelope::from_wire(
            CommandEnvelope::SCHEMA_VERSION + 1,
            id(RequestId::from_parts),
            CommandScope::Application,
            Revision::INITIAL,
            budget(),
            None,
            CommandIntent::Shutdown,
        ),
        Err(CommandBuildError::UnsupportedSchemaVersion {
            actual: 2,
            supported: 1,
        })
    );
}

#[test]
fn cancel_is_lifecycle_metadata_and_preserves_parent_scope() {
    let parent = id(OperationId::from_parts);
    let command = CommandEnvelope::new(
        id(RequestId::from_parts),
        context_scope(),
        Revision::from_wire_u64(4),
        budget(),
        Some(parent),
        CommandIntent::Cancel {
            operation_id: parent,
        },
    )
    .unwrap();
    assert_eq!(command.safety(), CommandSafety::Lifecycle);
    assert_eq!(command.redaction(), RedactionClass::MetadataOnly);
    assert_eq!(command.parent_operation_id(), Some(parent));
}

#[test]
fn statement_text_bounds_and_redacts_debug() {
    let statement = StatementText::new("select 1").unwrap();
    assert_eq!(statement.as_str(), "select 1");
    assert_eq!(statement.len(), 8);
    assert!(!statement.is_empty());
    let debug = format!("{statement:?}");
    assert!(debug.contains("bytes: 8"));
    assert!(!debug.contains("select"));

    assert_eq!(
        StatementText::new("x".repeat(MAX_STATEMENT_BYTES + 1)),
        Err(StatementTextError::TooLarge {
            actual: MAX_STATEMENT_BYTES + 1,
            limit: MAX_STATEMENT_BYTES,
        })
    );
    assert!(StatementText::new("x".repeat(MAX_STATEMENT_BYTES)).is_ok());
}

#[test]
fn execute_intent_is_may_write_and_context_scoped() {
    let statement = StatementText::new("update t set c = 1").unwrap();
    let command = CommandEnvelope::new(
        id(RequestId::from_parts),
        context_scope(),
        Revision::INITIAL,
        budget(),
        None,
        CommandIntent::Execute {
            statement: statement.clone(),
        },
    )
    .unwrap();
    assert_eq!(command.safety(), CommandSafety::MayWrite);
    assert_eq!(command.redaction(), RedactionClass::MetadataOnly);
    match command.intent() {
        CommandIntent::Execute { statement: body } => {
            assert_eq!(body.as_str(), "update t set c = 1");
        }
        other => panic!("unexpected intent {other:?}"),
    }

    let debug = format!("{:?}", command.intent());
    assert!(!debug.contains("update"));
    assert!(debug.contains("bytes:"));

    assert_eq!(
        CommandEnvelope::new(
            id(RequestId::from_parts),
            CommandScope::Application,
            Revision::INITIAL,
            budget(),
            None,
            CommandIntent::Execute { statement },
        ),
        Err(CommandBuildError::ScopeMismatch)
    );
}
