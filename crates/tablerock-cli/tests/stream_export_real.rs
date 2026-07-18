//! Real-path streaming re-query export: SELECT pages → CSV → atomic file.

use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tablerock_cli::{StreamExportFormat, run_stream_export};
use tablerock_core::{
    BoundedText, ByteLimit, Engine, IdParts, PageIdentity, PageLimits, ResultId, Revision,
    StatementText, Truncation, ValueKind,
};
use tablerock_engine::{
    DriverPageRequest, DriverPageStream, DriverSession, PostgresConnectConfig, PostgresSession,
    PostgresTlsMode,
};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

fn bt(s: &str) -> BoundedText {
    BoundedText::copy_from_str(s, ByteLimit::new(128)).unwrap()
}

fn page_to_strings(page: &tablerock_core::ResultPage) -> (Vec<String>, Vec<Vec<String>>) {
    let envelope = page.envelope();
    let columns: Vec<String> = page.columns().iter().map(|c| c.name().to_owned()).collect();
    let mut rows = Vec::new();
    for row in 0..envelope.row_count() {
        let mut cells = Vec::new();
        for col in 0..envelope.column_count() {
            let cell = page.cell(row, col).unwrap();
            let text = if cell.is_null() {
                "NULL".into()
            } else if cell.kind() == ValueKind::Signed {
                let mut buf = [0u8; 8];
                let b = cell.bytes();
                let n = b.len().min(8);
                buf[8 - n..].copy_from_slice(&b[..n]);
                i64::from_be_bytes(buf).to_string()
            } else {
                let mut s = String::from_utf8_lossy(cell.bytes()).into_owned();
                if matches!(cell.truncation(), Truncation::Truncated { .. }) {
                    s.push('…');
                }
                s
            };
            cells.push(text);
        }
        rows.push(cells);
    }
    (columns, rows)
}

#[tokio::test]
async fn streaming_requery_export_csv_and_cancel_cleanup() {
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
        bt("127.0.0.1"),
        port,
        bt("postgres"),
        bt("postgres"),
        PostgresTlsMode::Disabled,
    ))
    .await
    .unwrap();
    session
        .execute_sql(
            "CREATE TABLE export_probe (id int PRIMARY KEY, label text);
             INSERT INTO export_probe VALUES (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd'), (5, 'e');",
        )
        .await
        .unwrap();

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "tablerock-export-real-{}-{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("probe.csv");
    let path = dest.to_string_lossy().into_owned();

    let sql = StatementText::new("SELECT id, label FROM export_probe ORDER BY id").unwrap();
    let mut stream = session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement: sql,
            parameters: Vec::new(),
            limits: PageLimits::new(2, 16, 1024 * 1024, 64 * 1024),
            max_cell_bytes: 1024,
        })
        .await
        .unwrap();
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(0, 99).unwrap()).unwrap(),
        Revision::INITIAL,
        Engine::PostgreSql,
    );
    let mut start_row = 0_u64;
    let cancel = Arc::new(AtomicBool::new(false));
    let outcome = {
        let cancel = Arc::clone(&cancel);
        // Pull pages on the async runtime via a blocking bridge: collect pages first.
        let mut pages = Vec::new();
        loop {
            match stream.next_page(identity, start_row).await {
                Ok(Some(page)) => {
                    let count = u64::from(page.envelope().row_count());
                    pages.push(page_to_strings(&page));
                    start_row = start_row.saturating_add(count);
                    if page.envelope().delivery() == tablerock_core::PageDelivery::Final {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => panic!("stream error: {e}"),
            }
        }
        let mut iter = pages.into_iter().map(Some).chain(std::iter::once(None));
        run_stream_export(&path, StreamExportFormat::Csv, cancel, || {
            Ok(iter.next().flatten())
        })
        .unwrap()
    };
    assert!(outcome.rows >= 5, "rows={}", outcome.rows);
    let body = fs::read_to_string(&dest).unwrap();
    assert!(body.starts_with("id,label\n"));
    assert!(body.contains("5,e\n"));

    // Cancel path: write first page then cancel before finish.
    let dest2 = dir.join("cancel.csv");
    let path2 = dest2.to_string_lossy().into_owned();
    let cancel2 = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel2);
    let mut n = 0_u32;
    let err = run_stream_export(&path2, StreamExportFormat::Csv, cancel2, || {
        n += 1;
        if n == 1 {
            Ok(Some((
                vec!["id".into()],
                vec![vec!["1".into()], vec!["2".into()]],
            )))
        } else {
            flag.store(true, Ordering::SeqCst);
            Ok(Some((vec!["id".into()], vec![vec!["3".into()]])))
        }
    })
    .unwrap_err();
    assert!(matches!(
        err,
        tablerock_cli::StreamExportError::Cancelled { .. }
    ));
    assert!(!dest2.exists());

    let _ = fs::remove_dir_all(&dir);
}
