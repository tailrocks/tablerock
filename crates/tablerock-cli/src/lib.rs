//! Process and terminal adapters for TableRock.

mod effects;
mod import_apply;
mod import_csv;
mod ingress;
mod input;
mod pg_process;
mod projection;
mod run;
mod stream_export;
mod telemetry;
mod tool_discovery;

pub use import_apply::{ImportApplyError, apply_csv_inserts};
pub use import_csv::{
    CsvImportError, CsvTable, csv_to_insert_changes, is_formula_like, parse_csv,
    validate_insert_batch_size,
};
pub use pg_process::{
    PgToolRunOutcome, cancel_channel, run_pg_dump, run_pg_restore, validate_dump_path,
};
pub use stream_export::{
    StreamExportError, StreamExportFormat, StreamExportOutcome, StreamExporter, run_stream_export,
};
pub use tablerock_files::{AtomicFileWriter, FileEffectError, validate_export_path, write_atomic};
pub use telemetry::{default_otlp_is_off, enable_otlp_export, init_local_tracing, otlp_enabled};
pub use tool_discovery::{
    ToolStatus, argv_contains_secret, discover_tool, pg_dump_argv, pg_restore_argv,
};

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

pub use crossterm::event::EventStream;
pub use ingress::{
    Delivery, IngressReceiver, IngressSender, SendOutcome, TryReceiveError, bounded_ingress,
};
pub use input::{InputAdapter, map_backend_event, map_event};
pub use run::{
    RootMessageReceiver, RootMessageSender, RootProgress, RunError, root_message_channel, run,
    run_caught, run_with_root_messages,
};
pub use termrock::crossterm::{Session, SessionOptions};
