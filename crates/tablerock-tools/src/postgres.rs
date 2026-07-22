//! PostgreSQL client-tool discovery and supervised execution.
#![allow(
    clippy::too_many_arguments,
    reason = "process boundary keeps connection, credential, path, and cancellation inputs explicit"
)]

use std::{
    path::{Path, PathBuf},
    process::{Command as SyncCommand, Stdio},
    time::Duration,
};

use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Found { path: PathBuf, version: String },
    Missing { name: String },
    VersionProbeFailed { path: PathBuf, detail: String },
}

impl ToolStatus {
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Found { .. })
    }
}

#[must_use]
pub fn discover_tool(name: &str, explicit_path: Option<&str>) -> ToolStatus {
    if let Some(value) = explicit_path {
        let path = PathBuf::from(value.trim());
        return if path.is_file() {
            probe_version(path, name)
        } else {
            ToolStatus::Missing { name: name.into() }
        };
    }
    match find_on_path(name) {
        Some(path) => probe_version(path, name),
        None => ToolStatus::Missing { name: name.into() },
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|directory| directory.join(name))
            .find(|candidate| candidate.is_file())
    })
}

fn probe_version(path: PathBuf, expected_name: &str) -> ToolStatus {
    match SyncCommand::new(&path).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            let version = if stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr).trim().to_owned()
            } else {
                stdout
            };
            if version.starts_with(expected_name) || version.contains(&format!("{expected_name} ("))
            {
                ToolStatus::Found { path, version }
            } else {
                ToolStatus::VersionProbeFailed {
                    path,
                    detail: "tool identity mismatch".into(),
                }
            }
        }
        Ok(output) => ToolStatus::VersionProbeFailed {
            path,
            detail: format!("exit {:?}", output.status.code()),
        },
        Err(error) => ToolStatus::VersionProbeFailed {
            path,
            detail: error.to_string(),
        },
    }
}

#[must_use]
pub fn argv_contains_secret(argv: &[&str], secret: &str) -> bool {
    !secret.is_empty() && argv.iter().any(|argument| argument.contains(secret))
}

#[must_use]
pub fn pg_dump_argv(
    tool: &Path,
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    file: &Path,
) -> Vec<String> {
    vec![
        tool.display().to_string(),
        "-h".into(),
        host.into(),
        "-p".into(),
        port.to_string(),
        "-U".into(),
        username.into(),
        "-d".into(),
        database.into(),
        "-Fc".into(),
        "-f".into(),
        file.display().to_string(),
        "--no-password".into(),
    ]
}

#[must_use]
pub fn pg_restore_argv(
    tool: &Path,
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    file: &Path,
) -> Vec<String> {
    vec![
        tool.display().to_string(),
        "-h".into(),
        host.into(),
        "-p".into(),
        port.to_string(),
        "-U".into(),
        username.into(),
        "-d".into(),
        database.into(),
        "--no-password".into(),
        file.display().to_string(),
    ]
}

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
    run_pg_dump_configured(
        tool, host, port, database, username, password, file, "all", false, cancel,
    )
    .await
}

pub async fn run_pg_dump_configured(
    tool: &Path,
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: Option<&str>,
    file: &Path,
    content: &str,
    no_owner: bool,
    cancel: tokio::sync::watch::Receiver<bool>,
) -> PgToolRunOutcome {
    let mut argv = pg_dump_argv(tool, host, port, database, username, file);
    add_content_and_owner_options(&mut argv, content, no_owner);
    run_supervised(&argv, password, Some(file), cancel).await
}

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
    run_pg_restore_configured(
        tool, host, port, database, username, password, file, "all", false, false, cancel,
    )
    .await
}

pub async fn run_pg_restore_configured(
    tool: &Path,
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: Option<&str>,
    file: &Path,
    content: &str,
    clean: bool,
    no_owner: bool,
    cancel: tokio::sync::watch::Receiver<bool>,
) -> PgToolRunOutcome {
    let mut argv = pg_restore_argv(tool, host, port, database, username, file);
    add_content_and_owner_options(&mut argv, content, no_owner);
    if clean {
        argv.push("--clean".into());
        argv.push("--if-exists".into());
    }
    run_supervised(&argv, password, None, cancel).await
}

fn add_content_and_owner_options(argv: &mut Vec<String>, content: &str, no_owner: bool) {
    match content {
        "schema_only" => argv.push("--schema-only".into()),
        "data_only" => argv.push("--data-only".into()),
        _ => {}
    }
    if no_owner {
        argv.push("--no-owner".into());
    }
}

async fn run_supervised(
    argv: &[String],
    password: Option<&str>,
    remove_on_cancel: Option<&Path>,
    mut cancel: tokio::sync::watch::Receiver<bool>,
) -> PgToolRunOutcome {
    let Some(program) = argv.first() else {
        return PgToolRunOutcome::SpawnFailed {
            detail: "empty argv".into(),
        };
    };
    let mut command = Command::new(program);
    command
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(secret) = password {
        command.env("PGPASSWORD", secret);
    } else {
        command.env_remove("PGPASSWORD");
    }
    let mut child = match command.spawn() {
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
                    if let Some(path) = remove_on_cancel { let _ = std::fs::remove_file(path); }
                    return PgToolRunOutcome::Cancelled;
                }
            }
            status = child.wait() => {
                return match status {
                    Ok(status) if status.success() => PgToolRunOutcome::Succeeded {
                        exit_code: status.code().unwrap_or(0),
                    },
                    Ok(status) => PgToolRunOutcome::Failed {
                        exit_code: status.code(), detail: format!("exit {:?}", status.code()),
                    },
                    Err(error) => PgToolRunOutcome::Failed {
                        exit_code: None, detail: error.to_string(),
                    },
                };
            }
        }
    }
}

#[must_use]
pub fn cancel_channel() -> (
    tokio::sync::watch::Sender<bool>,
    tokio::sync::watch::Receiver<bool>,
) {
    tokio::sync::watch::channel(false)
}

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

    #[test]
    fn argv_never_carries_password_and_dump_is_custom_format() {
        let dump = pg_dump_argv(
            Path::new("/usr/bin/pg_dump"),
            "127.0.0.1",
            5432,
            "db",
            "user",
            Path::new("/tmp/out.dump"),
        );
        let refs = dump.iter().map(String::as_str).collect::<Vec<_>>();
        assert!(!argv_contains_secret(&refs, "secret"));
        assert!(refs.contains(&"-Fc"));
        assert!(refs.contains(&"--no-password"));
    }

    #[test]
    fn explicit_path_must_report_expected_tool_identity() {
        let path = Path::new("/usr/bin/true");
        if path.is_file() {
            assert!(matches!(
                discover_tool("pg_dump", Some("/usr/bin/true")),
                ToolStatus::VersionProbeFailed { .. }
            ));
        }
    }

    #[tokio::test]
    async fn configured_options_are_closed_and_composed() {
        let mut dump = pg_dump_argv(
            Path::new("/usr/bin/pg_dump"),
            "host",
            5432,
            "db",
            "user",
            Path::new("/tmp/out.dump"),
        );
        add_content_and_owner_options(&mut dump, "schema_only", true);
        assert!(dump.iter().any(|arg| arg == "--schema-only"));
        assert!(dump.iter().any(|arg| arg == "--no-owner"));

        let mut restore = pg_restore_argv(
            Path::new("/usr/bin/pg_restore"),
            "host",
            5432,
            "db",
            "user",
            Path::new("/tmp/in.dump"),
        );
        add_content_and_owner_options(&mut restore, "data_only", false);
        restore.extend(["--clean".into(), "--if-exists".into()]);
        assert!(restore.iter().any(|arg| arg == "--data-only"));
        assert!(restore.iter().any(|arg| arg == "--clean"));
        assert!(restore.iter().any(|arg| arg == "--if-exists"));
    }

    #[tokio::test]
    async fn cancellation_removes_partial_dump() {
        let directory =
            std::env::temp_dir().join(format!("tablerock-tools-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let output = directory.join("partial.dump");
        std::fs::write(&output, b"partial").unwrap();
        let (sender, receiver) = cancel_channel();
        let argv = vec!["sleep".into(), "30".into()];
        let task_output = output.clone();
        let task =
            tokio::spawn(
                async move { run_supervised(&argv, None, Some(&task_output), receiver).await },
            );
        tokio::time::sleep(Duration::from_millis(50)).await;
        sender.send(true).unwrap();
        assert_eq!(task.await.unwrap(), PgToolRunOutcome::Cancelled);
        assert!(!output.exists());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
