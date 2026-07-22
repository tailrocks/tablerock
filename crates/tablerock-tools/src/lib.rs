//! Shared, shell-free external-tool discovery and process supervision.

mod postgres;

pub use postgres::{
    PgToolRunOutcome, ToolStatus, argv_contains_secret, cancel_channel, discover_tool,
    pg_dump_argv, pg_restore_argv, run_pg_dump, run_pg_restore, validate_dump_path,
};
