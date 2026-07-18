//! Connection editor form state (TableRock-local; TermRock Form renders it).

use crate::effect::EngineKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorField {
    Engine,
    Name,
    Group,
    Environment,
    Host,
    Port,
    Database,
    Username,
    Password,
    PasswordSource,
    TlsMode,
    SshHost,
    SshPort,
    SshUsername,
    SshPassword,
    SshPrivateKey,
    SshKnownHostsPath,
    SshUseAgent,
    /// Newline-separated ReadOnly startup SQL/commands.
    StartupSql,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordSourceChoice {
    PromptOnConnect,
    DangerousPlaintext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsModeChoice {
    Off,
    VerifyCa,
    VerifyFull,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionFormModel {
    pub engine: EngineKind,
    pub name: String,
    pub group: String,
    pub environment: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub password_source: PasswordSourceChoice,
    pub tls_mode: TlsModeChoice,
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_username: String,
    pub ssh_password: String,
    /// OpenSSH private key PEM (plaintext or encrypted; password = passphrase).
    pub ssh_private_key: String,
    pub ssh_known_hosts_path: String,
    pub ssh_use_agent: bool,
    /// Newline-separated ReadOnly startup statements (one per line; `#` comments ignored).
    pub startup_sql: String,
    pub focused: EditorField,
    pub plaintext_acknowledged: bool,
    pub validation_error: Option<String>,
    pub test_status: Option<String>,
}

impl Default for ConnectionFormModel {
    fn default() -> Self {
        Self {
            engine: EngineKind::PostgreSql,
            name: String::new(),
            group: String::new(),
            environment: String::new(),
            host: "127.0.0.1".into(),
            port: "5432".into(),
            database: "postgres".into(),
            username: "postgres".into(),
            password: String::new(),
            password_source: PasswordSourceChoice::PromptOnConnect,
            tls_mode: TlsModeChoice::Off,
            ssh_host: String::new(),
            ssh_port: "22".into(),
            ssh_username: String::new(),
            ssh_password: String::new(),
            ssh_private_key: String::new(),
            ssh_known_hosts_path: String::new(),
            ssh_use_agent: false,
            startup_sql: String::new(),
            focused: EditorField::Name,
            plaintext_acknowledged: false,
            validation_error: None,
            test_status: None,
        }
    }
}

impl ConnectionFormModel {
    #[must_use]
    pub fn field_value(&self, field: EditorField) -> String {
        match field {
            EditorField::Engine => engine_label(self.engine).into(),
            EditorField::Name => self.name.clone(),
            EditorField::Group => self.group.clone(),
            EditorField::Environment => self.environment.clone(),
            EditorField::Host => self.host.clone(),
            EditorField::Port => self.port.clone(),
            EditorField::Database => self.database.clone(),
            EditorField::Username => self.username.clone(),
            EditorField::Password => {
                if self.password.is_empty() {
                    String::new()
                } else {
                    "••••".into()
                }
            }
            EditorField::PasswordSource => match self.password_source {
                PasswordSourceChoice::PromptOnConnect => "prompt".into(),
                PasswordSourceChoice::DangerousPlaintext => "plaintext".into(),
            },
            EditorField::TlsMode => match self.tls_mode {
                TlsModeChoice::Off => "off".into(),
                TlsModeChoice::VerifyCa => "verify-ca".into(),
                TlsModeChoice::VerifyFull => "verify-full".into(),
            },
            EditorField::SshHost => self.ssh_host.clone(),
            EditorField::SshPort => self.ssh_port.clone(),
            EditorField::SshUsername => self.ssh_username.clone(),
            EditorField::SshPassword => {
                if self.ssh_password.is_empty() {
                    String::new()
                } else {
                    "••••".into()
                }
            }
            EditorField::SshPrivateKey => {
                if self.ssh_private_key.is_empty() {
                    String::new()
                } else {
                    "•••• key present".into()
                }
            }
            EditorField::SshKnownHostsPath => self.ssh_known_hosts_path.clone(),
            EditorField::SshUseAgent => {
                if self.ssh_use_agent {
                    "agent".into()
                } else {
                    "password/key".into()
                }
            }
            EditorField::StartupSql => {
                if self.startup_sql.is_empty() {
                    String::new()
                } else {
                    let lines = self
                        .startup_sql
                        .lines()
                        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                        .count();
                    format!("{lines} line(s)")
                }
            }
        }
    }

    /// Parse editor startup SQL lines into a `StartupActionSet`.
    ///
    /// Line prefixes (case-insensitive, space after prefix required):
    /// - none / `#` comment → ReadOnly (default)
    /// - `!write ` or `!w ` → Write (review-gated at connect)
    /// - `!danger ` / `!dangerous ` / `!d ` → Dangerous (review-gated)
    pub fn startup_action_set(&self) -> Result<tablerock_core::StartupActionSet, String> {
        use tablerock_core::{StartupAction, StartupActionSet};
        let mut actions = Vec::new();
        for line in self.startup_sql.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (safety, statement) = parse_startup_line(trimmed);
            actions.push(
                StartupAction::from_str(statement, safety, 5_000, true)
                    .map_err(|e| e.to_string())?,
            );
        }
        StartupActionSet::new(actions).map_err(|e| e.to_string())
    }

    pub fn cycle_engine(&mut self) {
        self.engine = match self.engine {
            EngineKind::PostgreSql => {
                self.port = "8123".into();
                self.database = "default".into();
                EngineKind::ClickHouse
            }
            EngineKind::ClickHouse => {
                self.port = "6379".into();
                self.database = "0".into();
                EngineKind::Redis
            }
            EngineKind::Redis => {
                self.port = "5432".into();
                self.database = "postgres".into();
                EngineKind::PostgreSql
            }
        };
    }

    pub fn focus_next(&mut self) {
        self.focused = match self.focused {
            EditorField::Engine => EditorField::Name,
            EditorField::Name => EditorField::Group,
            EditorField::Group => EditorField::Environment,
            EditorField::Environment => EditorField::Host,
            EditorField::Host => EditorField::Port,
            EditorField::Port => EditorField::Database,
            EditorField::Database => EditorField::Username,
            EditorField::Username => EditorField::Password,
            EditorField::Password => EditorField::PasswordSource,
            EditorField::PasswordSource => EditorField::TlsMode,
            EditorField::TlsMode => EditorField::SshHost,
            EditorField::SshHost => EditorField::SshPort,
            EditorField::SshPort => EditorField::SshUsername,
            EditorField::SshUsername => EditorField::SshPassword,
            EditorField::SshPassword => EditorField::SshPrivateKey,
            EditorField::SshPrivateKey => EditorField::SshKnownHostsPath,
            EditorField::SshKnownHostsPath => EditorField::SshUseAgent,
            EditorField::SshUseAgent => EditorField::StartupSql,
            EditorField::StartupSql => EditorField::Engine,
        };
    }

    /// Validate first-version required fields.
    pub fn validate(&mut self) -> bool {
        if self.name.trim().is_empty() {
            self.validation_error = Some("name required".into());
            return false;
        }
        if self.host.trim().is_empty() {
            self.validation_error = Some("host required".into());
            return false;
        }
        if self.port.parse::<u16>().ok().filter(|p| *p > 0).is_none() {
            self.validation_error = Some("port must be 1..=65535".into());
            return false;
        }
        if matches!(
            self.password_source,
            PasswordSourceChoice::DangerousPlaintext
        ) && !self.plaintext_acknowledged
        {
            self.validation_error = Some("acknowledge plaintext password storage".into());
            return false;
        }
        if !self.ssh_host.trim().is_empty() {
            if self
                .ssh_port
                .parse::<u16>()
                .ok()
                .filter(|p| *p > 0)
                .is_none()
            {
                self.validation_error = Some("SSH port must be 1..=65535".into());
                return false;
            }
            if self.ssh_known_hosts_path.trim().is_empty() {
                self.validation_error = Some("SSH known_hosts path required".into());
                return false;
            }
            if !self.ssh_use_agent
                && self.ssh_password.is_empty()
                && self.ssh_private_key.trim().is_empty()
            {
                self.validation_error =
                    Some("SSH password, private key, or agent mode required".into());
                return false;
            }
        }
        if let Err(error) = self.startup_action_set() {
            self.validation_error = Some(error);
            return false;
        }
        self.validation_error = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_validate_accepts_private_key_without_password() {
        let mut editor = ConnectionFormModel {
            name: "demo".into(),
            host: "db.internal".into(),
            port: "5432".into(),
            ssh_host: "bastion".into(),
            ssh_port: "22".into(),
            ssh_private_key: "-----BEGIN OPENSSH PRIVATE KEY-----\nx\n-----END OPENSSH PRIVATE KEY-----"
                .into(),
            ssh_known_hosts_path: "/tmp/known_hosts".into(),
            ..ConnectionFormModel::default()
        };
        assert!(editor.validate());
    }

    #[test]
    fn ssh_validate_requires_auth_material() {
        let mut editor = ConnectionFormModel {
            name: "demo".into(),
            host: "db.internal".into(),
            port: "5432".into(),
            ssh_host: "bastion".into(),
            ssh_port: "22".into(),
            ssh_known_hosts_path: "/tmp/known_hosts".into(),
            ..ConnectionFormModel::default()
        };
        assert!(!editor.validate());
        assert!(editor
            .validation_error
            .as_deref()
            .unwrap_or("")
            .contains("password, private key, or agent"));
    }

    #[test]
    fn ssh_validate_accepts_agent_mode_without_secrets() {
        let mut editor = ConnectionFormModel {
            name: "demo".into(),
            host: "db.internal".into(),
            port: "5432".into(),
            ssh_host: "bastion".into(),
            ssh_port: "22".into(),
            ssh_use_agent: true,
            ssh_known_hosts_path: "/tmp/known_hosts".into(),
            ..ConnectionFormModel::default()
        };
        assert!(editor.validate());
    }

    #[test]
    fn startup_sql_parses_read_only_lines() {
        let editor = ConnectionFormModel {
            startup_sql: "# comment\nSELECT 1\n\nSELECT 2\n".into(),
            ..ConnectionFormModel::default()
        };
        let set = editor.startup_action_set().unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(set.actions()[0].statement(), "SELECT 1");
        assert_eq!(set.actions()[1].statement(), "SELECT 2");
        assert!(set.actions()[0].safety().may_auto_run());
    }

    #[test]
    fn startup_sql_parses_write_and_danger_prefixes() {
        use tablerock_core::StartupSafetyClass;
        let editor = ConnectionFormModel {
            startup_sql: "SELECT 1\n!write SET search_path TO app\n!danger DROP TABLE tmp\n".into(),
            ..ConnectionFormModel::default()
        };
        let set = editor.startup_action_set().unwrap();
        assert_eq!(set.len(), 3);
        assert_eq!(set.actions()[0].safety(), StartupSafetyClass::ReadOnly);
        assert_eq!(set.actions()[1].safety(), StartupSafetyClass::Write);
        assert_eq!(set.actions()[1].statement(), "SET search_path TO app");
        assert_eq!(set.actions()[2].safety(), StartupSafetyClass::Dangerous);
        assert_eq!(set.actions()[2].statement(), "DROP TABLE tmp");
        assert_eq!(set.review_required(false).len(), 2);
        assert_eq!(set.auto_runnable(false).len(), 1);
    }
}

/// Split optional safety prefix from a startup SQL line.
fn parse_startup_line(trimmed: &str) -> (tablerock_core::StartupSafetyClass, &str) {
    use tablerock_core::StartupSafetyClass;
    let lower = trimmed.to_ascii_lowercase();
    for (prefix, safety) in [
        ("!dangerous ", StartupSafetyClass::Dangerous),
        ("!danger ", StartupSafetyClass::Dangerous),
        ("!d ", StartupSafetyClass::Dangerous),
        ("!write ", StartupSafetyClass::Write),
        ("!w ", StartupSafetyClass::Write),
    ] {
        if lower.starts_with(prefix) {
            return (safety, trimmed[prefix.len()..].trim());
        }
    }
    (StartupSafetyClass::ReadOnly, trimmed)
}

const fn engine_label(engine: EngineKind) -> &'static str {
    match engine {
        EngineKind::PostgreSql => "PostgreSQL",
        EngineKind::ClickHouse => "ClickHouse",
        EngineKind::Redis => "Redis",
    }
}
