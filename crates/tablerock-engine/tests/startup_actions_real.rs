//! Real-server proof: PostgreSQL startup actions auto-run ReadOnly, skip Write.

use tablerock_core::{
    BoundedText, ByteLimit, StartupAction, StartupActionOutcome, StartupActionSet,
    StartupSafetyClass,
};
use tablerock_engine::{PostgresConnectConfig, PostgresSession, PostgresTlsMode, run_postgres_startup_actions};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

fn text(s: &str) -> BoundedText {
    BoundedText::copy_from_str(s, ByteLimit::new(253)).unwrap()
}

#[tokio::test]
async fn postgres_startup_actions_auto_run_and_skip_writes() {
    let container = GenericImage::new("postgres", "18.4-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .start()
        .await
        .unwrap();
    let port = container.get_host_port_ipv4(5432.tcp()).await.unwrap();
    let session = PostgresSession::connect(&PostgresConnectConfig::new(
        text("127.0.0.1"),
        port,
        text("postgres"),
        text("postgres"),
        PostgresTlsMode::Disabled,
    ))
    .await
    .unwrap();

    let set = StartupActionSet::new(vec![
        StartupAction::from_str("SELECT 1", StartupSafetyClass::ReadOnly, 5_000, true).unwrap(),
        StartupAction::from_str(
            "CREATE TABLE IF NOT EXISTS startup_probe (id int)",
            StartupSafetyClass::Write,
            5_000,
            true,
        )
        .unwrap(),
        StartupAction::from_str("SELECT 1/0", StartupSafetyClass::ReadOnly, 5_000, true).unwrap(),
    ])
    .unwrap();

    let report = run_postgres_startup_actions(&session, &set, false).await;
    let outcomes: Vec<_> = report.outcomes().iter().map(|(_, o)| *o).collect();
    assert_eq!(
        outcomes,
        vec![
            StartupActionOutcome::Succeeded,
            StartupActionOutcome::SkippedNeedsReview,
            StartupActionOutcome::Failed, // division by zero
        ]
    );
    assert!(report.has_failure());

    // Reconnect filter: Write-only-on-initial would be skipped on reconnect;
    // here all three have run_on_reconnect true, still skip Write.
    let reconnect = run_postgres_startup_actions(&session, &set, true).await;
    assert_eq!(
        reconnect.outcomes()[1].1,
        StartupActionOutcome::SkippedNeedsReview
    );

    session.shutdown().await.ok();
}
