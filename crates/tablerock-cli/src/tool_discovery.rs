//! Compatibility re-exports for the shared external-tool adapter.

pub use tablerock_tools::{
    ToolStatus, argv_contains_secret, discover_tool, pg_dump_argv, pg_restore_argv,
};
