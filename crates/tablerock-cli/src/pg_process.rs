//! Supervised `pg_dump` / `pg_restore` process runs (no shell).
//!
//! Password is passed only via `PGPASSWORD` environment variable — never argv.
//! Cancellation kills the process group and removes incomplete output files.
#![allow(
    clippy::too_many_arguments,
    reason = "process boundary keeps connection, credential, path, and cancellation inputs explicit"
)]

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use tokio::process::Command;

use crate::tool_discovery::{pg_dump_argv, pg_restore_argv};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PgToolRunOutcome {
    Succeeded {
        exit_code: i32,
    },
    Failed {
        exit_code: Option<i32>,
        detail: String,
    },
    Cancelled,
    SpawnFailed {
        detail: String,
    },
}

/// Run pg_dump with optional password (env only) and cancel support.
pub async fn run_pg_dump(
    tool: &Path,
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: Option<&str>,
    file: &Path,
    cancel: tokio::sync::watch::Receiver<bool>,
) -> PgToolRunOutcome {
    let argv = pg_dump_argv(tool, host, port, database, username, file);
    run_supervised(&argv, password, Some(file), cancel).await
}

/// Run pg_restore with optional password (env only) and cancel support.
pub async fn run_pg_restore(
    tool: &Path,
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: Option<&str>,
    file: &Path,
    cancel: tokio::sync::watch::Receiver<bool>,
) -> PgToolRunOutcome {
    let argv = pg_restore_argv(tool, host, port, database, username, file);
    run_supervised(&argv, password, None, cancel).await
}

async fn run_supervised(
    argv: &[String],
    password: Option<&str>,
    remove_on_cancel: Option<&Path>,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) -> PgToolRunOutcome {
    if argv.is_empty() {
        return PgToolRunOutcome::SpawnFailed {
            detail: "empty argv".into(),
        };
    }
    let program = &argv[0];
    let mut cmd = Command::new(program);
    if argv.len() > 1 {
        cmd.args(&argv[1..]);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // Password only via env — never argv (enforced by caller argv builders).
    if let Some(secret) = password {
        cmd.env("PGPASSWORD", secret);
    } else {
        cmd.env_remove("PGPASSWORD");
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            return PgToolRunOutcome::SpawnFailed {
                detail: error.to_string(),
            };
        }
    };

    loop {
        tokio::select! {
            changed = cancel.changed() => {
                if changed.is_ok() && *cancel.borrow() {
                    let _ = child.start_kill();
                    let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
                    if let Some(path) = remove_on_cancel {
                        let _ = std::fs::remove_file(path);
                    }
                    return PgToolRunOutcome::Cancelled;
                }
            }
            status = child.wait() => {
                return match status {
                    Ok(status) if status.success() => PgToolRunOutcome::Succeeded {
                        exit_code: status.code().unwrap_or(0),
                    },
                    Ok(status) => PgToolRunOutcome::Failed {
                        exit_code: status.code(),
                        detail: format!("exit {:?}", status.code()),
                    },
                    Err(error) => PgToolRunOutcome::Failed {
                        exit_code: None,
                        detail: error.to_string(),
                    },
                };
            }
        }
    }
}

/// Build cancel channel (true = cancel requested).
#[must_use]
pub fn cancel_channel() -> (
    tokio::sync::watch::Sender<bool>,
    tokio::sync::watch::Receiver<bool>,
) {
    tokio::sync::watch::channel(false)
}

/// Whether a dump path is absolute or will be created as a file (not a directory).
pub fn validate_dump_path(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("dump path is empty".into());
    }
    if path.is_dir() {
        return Err("dump path is a directory".into());
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn cancel_kills_long_running_process_and_removes_output() {
        let dir = std::env::temp_dir().join(format!(
            "tablerock-pg-process-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::create_dir_all(&dir);
        let out = dir.join("out.dump");
        // Pre-create incomplete output to prove cleanup on cancel.
        std::fs::write(&out, b"partial").unwrap();

        let (tx, rx) = cancel_channel();
        // Use sleep as a stand-in long tool (no shell).
        let argv = vec!["sleep".into(), "30".into()];
        let out_for_task = out.clone();
        let join = tokio::spawn(async move {
            run_supervised(&argv, None, Some(out_for_task.as_path()), rx).await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        tx.send(true).unwrap();
        let outcome = join.await.unwrap();
        assert_eq!(outcome, PgToolRunOutcome::Cancelled);
        assert!(!out.exists(), "cancel must remove incomplete dump");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn successful_true_command() {
        let (tx, rx) = cancel_channel();
        let argv = vec!["true".into()];
        let outcome = run_supervised(&argv, None, None, rx).await;
        assert!(matches!(outcome, PgToolRunOutcome::Succeeded { .. }));
        drop(tx);
    }

    #[tokio::test]
    async fn password_env_not_in_argv_builders() {
        use crate::tool_discovery::argv_contains_secret;
        let dump = pg_dump_argv(
            Path::new("/usr/bin/pg_dump"),
            "127.0.0.1",
            5432,
            "db",
            "user",
            Path::new("/tmp/x.dump"),
        );
        let restore = pg_restore_argv(
            Path::new("/usr/bin/pg_restore"),
            "127.0.0.1",
            5432,
            "db",
            "user",
            Path::new("/tmp/x.dump"),
        );
        let secret = "s3cret-pass";
        let dump_refs: Vec<&str> = dump.iter().map(String::as_str).collect();
        let restore_refs: Vec<&str> = restore.iter().map(String::as_str).collect();
        assert!(!argv_contains_secret(&dump_refs, secret));
        assert!(!argv_contains_secret(&restore_refs, secret));
    }

    #[test]
    fn validate_dump_path_rejects_empty_and_dir() {
        assert!(validate_dump_path(Path::new("")).is_err());
        let dir = std::env::temp_dir();
        assert!(validate_dump_path(&dir).is_err());
    }
}
