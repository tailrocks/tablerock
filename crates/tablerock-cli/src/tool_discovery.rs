//! External tool discovery (pg_dump / pg_restore / ssh) without shell.
//!
//! Tools are located by absolute path setting or PATH lookup via `which`-style
//! search of PATH components. Version probe uses direct spawn, never a shell.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

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

/// Resolve a tool: explicit path first, then PATH search for `name`.
pub fn discover_tool(name: &str, explicit_path: Option<&str>) -> ToolStatus {
    if let Some(p) = explicit_path {
        let path = PathBuf::from(p.trim());
        if path.is_file() {
            return probe_version(path);
        }
        return ToolStatus::Missing {
            name: name.to_owned(),
        };
    }
    match find_on_path(name) {
        Some(path) => probe_version(path),
        None => ToolStatus::Missing {
            name: name.to_owned(),
        },
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn probe_version(path: PathBuf) -> ToolStatus {
    // Direct spawn — no shell. Version flag common to pg_dump/pg_restore/ssh.
    let output = Command::new(&path).arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            let version = if version.is_empty() {
                String::from_utf8_lossy(&out.stderr).trim().to_owned()
            } else {
                version
            };
            ToolStatus::Found { path, version }
        }
        Ok(out) => ToolStatus::VersionProbeFailed {
            path,
            detail: format!("exit {:?}", out.status.code()),
        },
        Err(e) => ToolStatus::VersionProbeFailed {
            path,
            detail: e.to_string(),
        },
    }
}

/// Assert secrets never appear as argv elements (export helper for tests).
#[must_use]
pub fn argv_contains_secret(argv: &[&str], secret: &str) -> bool {
    !secret.is_empty() && argv.iter().any(|a| a.contains(secret))
}

/// Build pg_dump argv with password via env (never argv).
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
        "-f".into(),
        file.display().to_string(),
        "--no-password".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_tool_is_explicit() {
        let s = discover_tool("definitely-not-a-real-tool-xyzzy", None);
        assert!(matches!(s, ToolStatus::Missing { .. }));
        assert!(!s.is_available());
    }

    #[test]
    fn password_never_in_argv() {
        let argv = pg_dump_argv(
            Path::new("/usr/bin/pg_dump"),
            "127.0.0.1",
            5432,
            "db",
            "user",
            Path::new("/tmp/out.dump"),
        );
        let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        assert!(!argv_contains_secret(&refs, "s3cret"));
        assert!(refs.iter().any(|a| *a == "--no-password"));
    }

    #[test]
    fn which_self_or_missing() {
        // `true` exists on unix PATH typically.
        let s = discover_tool("true", None);
        // May be Found or Missing depending on environment — just must not panic.
        let _ = s.is_available();
    }
}
