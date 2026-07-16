use tablerock_core::{
    ApplicationCode, DiagnosticBuildError, DiagnosticPosition, Engine, FailureClass,
    OperationSafety, OperatorAction, OutcomeCertainty, PositionUnit, PostgreSqlCode, RedisCode,
    RetryAdvice, SafeCode, SafeDiagnostic, Severity,
};

#[test]
fn safe_codes_are_closed_or_numeric_and_cannot_carry_arbitrary_text() {
    let codes = [
        SafeCode::PostgreSql(PostgreSqlCode::UniqueViolation),
        SafeCode::ClickHouse(241),
        SafeCode::Redis(RedisCode::NoAuth),
        SafeCode::Application(ApplicationCode::ResourceLimit),
    ];
    for code in codes {
        let debug = format!("{code:?}");
        assert!(!debug.contains("password"));
        assert!(!debug.contains("localhost"));
    }
}

#[test]
fn writes_and_unknown_safety_can_never_claim_automatic_retry() {
    for (certainty, safety) in [
        (OutcomeCertainty::Unknown, OperationSafety::Unknown),
        (OutcomeCertainty::WriteApplied, OperationSafety::MayWrite),
        (OutcomeCertainty::NotDispatched, OperationSafety::MayWrite),
    ] {
        assert_eq!(
            SafeDiagnostic::new(
                FailureClass::Server,
                Engine::PostgreSql,
                Severity::Error,
                certainty,
                safety,
            )
            .with_code(SafeCode::PostgreSql(PostgreSqlCode::SerializationFailure,))
            .unwrap()
            .with_action(OperatorAction::ReviewOutcome)
            .with_retry(RetryAdvice::SafeAutomaticReadOnly),
            Err(DiagnosticBuildError::UnsafeRetryAdvice { certainty, safety })
        );
    }
}

#[test]
fn redacted_diagnostic_preserves_closed_code_action_position_and_certainty() {
    let diagnostic = SafeDiagnostic::new(
        FailureClass::Conflict,
        Engine::PostgreSql,
        Severity::Error,
        OutcomeCertainty::WriteNotApplied,
        OperationSafety::MayWrite,
    )
    .with_code(SafeCode::PostgreSql(PostgreSqlCode::UniqueViolation))
    .unwrap()
    .with_position(DiagnosticPosition::new(PositionUnit::ServerCharacter, 17))
    .with_action(OperatorAction::ReviewInput)
    .with_retry(RetryAdvice::AfterUserAction)
    .unwrap();

    assert_eq!(
        diagnostic.code(),
        Some(SafeCode::PostgreSql(PostgreSqlCode::UniqueViolation))
    );
    assert_eq!(diagnostic.position().unwrap().value(), 17);
    assert_eq!(diagnostic.action(), OperatorAction::ReviewInput);
    assert_eq!(diagnostic.certainty(), OutcomeCertainty::WriteNotApplied);
    assert_eq!(diagnostic.safety(), OperationSafety::MayWrite);
}

#[test]
fn engine_specific_codes_cannot_cross_engine_boundaries() {
    let diagnostic = SafeDiagnostic::new(
        FailureClass::Server,
        Engine::Redis,
        Severity::Error,
        OutcomeCertainty::ReadOnly,
        OperationSafety::ProvenReadOnly,
    );
    assert_eq!(
        diagnostic.with_code(SafeCode::PostgreSql(PostgreSqlCode::Other)),
        Err(DiagnosticBuildError::CodeEngineMismatch {
            diagnostic_engine: Engine::Redis,
        })
    );
}

#[test]
fn automatic_retry_requires_both_read_only_proof_and_safe_outcome_certainty() {
    let conservative = SafeDiagnostic::new(
        FailureClass::Connectivity,
        Engine::Redis,
        Severity::Error,
        OutcomeCertainty::Unknown,
        OperationSafety::Unknown,
    );
    assert_eq!(conservative.code(), None);
    assert_eq!(conservative.position(), None);
    assert_eq!(conservative.action(), OperatorAction::None);
    assert_eq!(conservative.retry(), RetryAdvice::Never);

    for certainty in [OutcomeCertainty::NotDispatched, OutcomeCertainty::ReadOnly] {
        assert!(
            SafeDiagnostic::new(
                FailureClass::Connectivity,
                Engine::Redis,
                Severity::Error,
                certainty,
                OperationSafety::ProvenReadOnly,
            )
            .with_retry(RetryAdvice::SafeAutomaticReadOnly)
            .is_ok()
        );
    }

    assert_eq!(
        SafeDiagnostic::new(
            FailureClass::Connectivity,
            Engine::Redis,
            Severity::Error,
            OutcomeCertainty::Unknown,
            OperationSafety::ProvenReadOnly,
        )
        .with_retry(RetryAdvice::SafeAutomaticReadOnly),
        Err(DiagnosticBuildError::UnsafeRetryAdvice {
            certainty: OutcomeCertainty::Unknown,
            safety: OperationSafety::ProvenReadOnly,
        })
    );
}
