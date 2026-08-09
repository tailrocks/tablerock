//! Shared durable operator-store path for CLI/TUI and native macOS.
//!
//! Production clients open the same `profiles.db` under a single data root so
//! saved profiles, query history, session intent, layouts, and related
//! operator data round-trip when switching clients. Live engine sessions stay
//! process-local (no daemon); switch mid-work is durable intent + reconnect.
//!
//! Concurrent dual-writer access is not supported: open one client, quit or
//! close persistence, then open the other (single-writer switch discipline).

use std::{
    env, fmt,
    path::{Path, PathBuf},
};

/// Durable store filename both clients open (matches native `AppPaths`).
pub const OPERATOR_PROFILES_DB_FILE: &str = "profiles.db";

/// Application data directory name under the platform operator root.
pub const OPERATOR_DATA_DIR_NAME: &str = "TableRock";

/// Absolute directory override for test isolation (`{root}/profiles.db`).
///
/// Same variable the native app honors under `TABLEROCK_TEST_MODE`. CLI/TUI
/// production open-default uses an absolute value when set.
pub const TEST_ROOT_ENV: &str = "TABLEROCK_TEST_ROOT";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorPathError {
    /// `TABLEROCK_TEST_ROOT` was set but is not an absolute path.
    AbsoluteTestRootRequired,
    /// No usable home directory for the production layout.
    HomeUnavailable,
}

impl fmt::Display for OperatorPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AbsoluteTestRootRequired => {
                write!(f, "TABLEROCK_TEST_ROOT must be an absolute path")
            }
            Self::HomeUnavailable => write!(f, "home directory unavailable for operator store"),
        }
    }
}

impl std::error::Error for OperatorPathError {}

/// Platform operator data root (parent of `profiles.db`), given an operator home.
///
/// - macOS: `{home}/Library/Application Support/TableRock`
/// - other: `{XDG_DATA_HOME|/home/.local/share}/TableRock`
#[must_use]
pub fn operator_data_root(home: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Application Support")
            .join(OPERATOR_DATA_DIR_NAME)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let base = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".local").join("share"));
        base.join(OPERATOR_DATA_DIR_NAME)
    }
}

/// Resolve the durable profiles database path from explicit roots (testable).
///
/// When `test_root` is `Some`, it must be absolute and yields
/// `{test_root}/profiles.db`. Otherwise returns the production path under
/// `home`.
pub fn resolve_operator_profiles_database(
    home: &Path,
    test_root: Option<&Path>,
) -> Result<PathBuf, OperatorPathError> {
    if let Some(root) = test_root {
        if !root.is_absolute() {
            return Err(OperatorPathError::AbsoluteTestRootRequired);
        }
        return Ok(root.join(OPERATOR_PROFILES_DB_FILE));
    }
    Ok(operator_data_root(home).join(OPERATOR_PROFILES_DB_FILE))
}

/// Production open-default path used by CLI/TUI (and the layout native matches).
///
/// Honors absolute `TABLEROCK_TEST_ROOT`. Never embeds a process id in the
/// durable store filename.
pub fn default_operator_profiles_database() -> Result<PathBuf, OperatorPathError> {
    let home = operator_home_dir().ok_or(OperatorPathError::HomeUnavailable)?;
    let test_root = match env::var_os(TEST_ROOT_ENV) {
        Some(raw) => {
            let root = PathBuf::from(raw);
            if !root.is_absolute() {
                return Err(OperatorPathError::AbsoluteTestRootRequired);
            }
            Some(root)
        }
        None => None,
    };
    resolve_operator_profiles_database(&home, test_root.as_deref())
}

fn operator_home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn production_macos_layout_matches_native_app_paths() {
        let home = Path::new("/Users/operator");
        let path = resolve_operator_profiles_database(home, None).unwrap();
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                path,
                PathBuf::from("/Users/operator/Library/Application Support/TableRock/profiles.db")
            );
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(
                path,
                PathBuf::from("/Users/operator/.local/share/TableRock/profiles.db")
            );
        }
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(OPERATOR_PROFILES_DB_FILE)
        );
        let rendered = path.to_string_lossy();
        assert!(!rendered.contains("state-"));
        assert!(!rendered.contains(&format!("{}", std::process::id())));
    }

    #[test]
    fn absolute_test_root_isolates_store() {
        let home = Path::new("/Users/operator");
        let root = Path::new("/private/tmp/TableRockUITest-123");
        let path = resolve_operator_profiles_database(home, Some(root)).unwrap();
        assert_eq!(
            path,
            PathBuf::from("/private/tmp/TableRockUITest-123/profiles.db")
        );
        assert!(!path
            .to_string_lossy()
            .contains("Application Support"));
    }

    #[test]
    fn relative_test_root_is_rejected() {
        let home = Path::new("/Users/operator");
        let err = resolve_operator_profiles_database(home, Some(Path::new("relative/path")))
            .unwrap_err();
        assert_eq!(err, OperatorPathError::AbsoluteTestRootRequired);
    }
}
