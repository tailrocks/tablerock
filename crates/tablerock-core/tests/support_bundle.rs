use tablerock_core::{
    Engine, FailureClass, MAX_SUPPORT_DIAGNOSTICS, OperationSafety, OutcomeCertainty,
    SafeDiagnostic, Severity, SupportArchitecture, SupportBundle, SupportBundleError,
    SupportOperatingSystem, SupportPlatform,
};

fn safe_failure() -> SafeDiagnostic {
    SafeDiagnostic::new(
        FailureClass::Authentication,
        Engine::PostgreSql,
        Severity::Error,
        OutcomeCertainty::NotDispatched,
        OperationSafety::Unknown,
    )
}

#[test]
fn bundle_projects_only_closed_safe_diagnostic_fields() {
    let mut bundle = SupportBundle::new(SupportPlatform::new(
        SupportOperatingSystem::MacOs,
        SupportArchitecture::Arm64,
    ));
    bundle.push(&safe_failure()).unwrap();

    let rendered = bundle.render("0.1.0");
    assert!(rendered.contains("schema=1\n"));
    assert!(rendered.contains("platform.os=macos\n"));
    assert!(rendered.contains("diagnostic.0=PostgreSql|Authentication|None|Error"));
    for forbidden in ["password", "SELECT", "/Users/", "localhost", "cell-value"] {
        assert!(!rendered.contains(forbidden));
    }
}

#[test]
fn bundle_is_bounded_and_reports_omissions() {
    let mut bundle = SupportBundle::new(SupportPlatform::current());
    for _ in 0..MAX_SUPPORT_DIAGNOSTICS {
        bundle.push(&safe_failure()).unwrap();
    }
    assert_eq!(
        bundle.push(&safe_failure()),
        Err(SupportBundleError::DiagnosticLimit)
    );
    assert_eq!(bundle.diagnostics().len(), MAX_SUPPORT_DIAGNOSTICS);
    assert_eq!(bundle.omitted_diagnostics(), 1);
}

#[test]
fn arbitrary_version_text_cannot_enter_bundle() {
    let bundle = SupportBundle::new(SupportPlatform::current());
    let rendered = bundle.render("0.1.0\npassword=secret");
    assert!(rendered.contains("client.version=invalid\n"));
    assert!(!rendered.contains("secret"));
}
