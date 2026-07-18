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
    PasswordSource,
    TlsMode,
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
    pub focused: EditorField,
    pub plaintext_acknowledged: bool,
    pub validation_error: Option<String>,
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
            focused: EditorField::Name,
            plaintext_acknowledged: false,
            validation_error: None,
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
            EditorField::PasswordSource => match self.password_source {
                PasswordSourceChoice::PromptOnConnect => "prompt".into(),
                PasswordSourceChoice::DangerousPlaintext => "plaintext".into(),
            },
            EditorField::TlsMode => match self.tls_mode {
                TlsModeChoice::Off => "off".into(),
                TlsModeChoice::VerifyCa => "verify-ca".into(),
                TlsModeChoice::VerifyFull => "verify-full".into(),
            },
        }
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
            EditorField::Username => EditorField::PasswordSource,
            EditorField::PasswordSource => EditorField::TlsMode,
            EditorField::TlsMode => EditorField::Engine,
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
        self.validation_error = None;
        true
    }
}

const fn engine_label(engine: EngineKind) -> &'static str {
    match engine {
        EngineKind::PostgreSql => "PostgreSQL",
        EngineKind::ClickHouse => "ClickHouse",
        EngineKind::Redis => "Redis",
    }
}
