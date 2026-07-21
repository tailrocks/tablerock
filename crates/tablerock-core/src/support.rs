use std::{error::Error, fmt};

use crate::{
    DiagnosticPosition, Engine, FailureClass, OperationOutcome, OperationSafety, OperatorAction,
    OutcomeCertainty, RetryAdvice, SafeCode, SafeDiagnostic, Severity,
};

pub const SUPPORT_BUNDLE_SCHEMA_VERSION: u16 = 2;
pub const MAX_SUPPORT_DIAGNOSTICS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportOperatingSystem {
    MacOs,
    Linux,
    Windows,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportArchitecture {
    Arm64,
    X86_64,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SupportPlatform {
    operating_system: SupportOperatingSystem,
    architecture: SupportArchitecture,
}

impl SupportPlatform {
    #[must_use]
    pub const fn new(
        operating_system: SupportOperatingSystem,
        architecture: SupportArchitecture,
    ) -> Self {
        Self {
            operating_system,
            architecture,
        }
    }

    #[must_use]
    pub const fn current() -> Self {
        let operating_system = if cfg!(target_os = "macos") {
            SupportOperatingSystem::MacOs
        } else if cfg!(target_os = "linux") {
            SupportOperatingSystem::Linux
        } else if cfg!(target_os = "windows") {
            SupportOperatingSystem::Windows
        } else {
            SupportOperatingSystem::Other
        };
        let architecture = if cfg!(target_arch = "aarch64") {
            SupportArchitecture::Arm64
        } else if cfg!(target_arch = "x86_64") {
            SupportArchitecture::X86_64
        } else {
            SupportArchitecture::Other
        };
        Self::new(operating_system, architecture)
    }

    #[must_use]
    pub const fn operating_system(self) -> SupportOperatingSystem {
        self.operating_system
    }

    #[must_use]
    pub const fn architecture(self) -> SupportArchitecture {
        self.architecture
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SupportDiagnostic {
    class: FailureClass,
    engine: Engine,
    code: Option<SafeCode>,
    severity: Severity,
    position: Option<DiagnosticPosition>,
    action: OperatorAction,
    certainty: OutcomeCertainty,
    safety: OperationSafety,
    retry: RetryAdvice,
}

impl From<&SafeDiagnostic> for SupportDiagnostic {
    fn from(value: &SafeDiagnostic) -> Self {
        Self {
            class: value.class(),
            engine: value.engine(),
            code: value.code(),
            severity: value.severity(),
            position: value.position(),
            action: value.action(),
            certainty: value.certainty(),
            safety: value.safety(),
            retry: value.retry(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportBundle {
    platform: SupportPlatform,
    diagnostics: Vec<SupportDiagnostic>,
    omitted_diagnostics: u64,
    operation_outcomes: Vec<(Engine, OperationOutcome)>,
    omitted_operation_outcomes: u64,
}

impl SupportBundle {
    #[must_use]
    pub const fn new(platform: SupportPlatform) -> Self {
        Self {
            platform,
            diagnostics: Vec::new(),
            omitted_diagnostics: 0,
            operation_outcomes: Vec::new(),
            omitted_operation_outcomes: 0,
        }
    }

    /// Retains a closed runtime outcome without server text, SQL, or values.
    pub fn push_operation_outcome(
        &mut self,
        engine: Engine,
        outcome: OperationOutcome,
    ) -> Result<(), SupportBundleError> {
        if self.operation_outcomes.len() == MAX_SUPPORT_DIAGNOSTICS {
            self.omitted_operation_outcomes = self.omitted_operation_outcomes.saturating_add(1);
            return Err(SupportBundleError::DiagnosticLimit);
        }
        self.operation_outcomes.push((engine, outcome));
        Ok(())
    }

    pub fn push(&mut self, diagnostic: &SafeDiagnostic) -> Result<(), SupportBundleError> {
        if self.diagnostics.len() == MAX_SUPPORT_DIAGNOSTICS {
            self.omitted_diagnostics = self.omitted_diagnostics.saturating_add(1);
            return Err(SupportBundleError::DiagnosticLimit);
        }
        self.diagnostics.push(diagnostic.into());
        Ok(())
    }

    #[must_use]
    pub const fn platform(&self) -> SupportPlatform {
        self.platform
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[SupportDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub const fn omitted_diagnostics(&self) -> u64 {
        self.omitted_diagnostics
    }

    #[must_use]
    pub fn render(&self, client_version: &str) -> String {
        let mut output = format!(
            "schema={}\nclient.version={}\nplatform.os={}\nplatform.arch={}\ndiagnostics.count={}\ndiagnostics.omitted={}\noperation_outcomes.count={}\noperation_outcomes.omitted={}\n",
            SUPPORT_BUNDLE_SCHEMA_VERSION,
            safe_version(client_version),
            os_label(self.platform.operating_system),
            architecture_label(self.platform.architecture),
            self.diagnostics.len(),
            self.omitted_diagnostics,
            self.operation_outcomes.len(),
            self.omitted_operation_outcomes,
        );
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            use std::fmt::Write as _;
            let _ = writeln!(
                output,
                "diagnostic.{index}={:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
                diagnostic.engine,
                diagnostic.class,
                diagnostic.code,
                diagnostic.severity,
                diagnostic.position,
                diagnostic.action,
                diagnostic.certainty,
                diagnostic.safety,
                diagnostic.retry,
            );
        }
        for (index, (engine, outcome)) in self.operation_outcomes.iter().enumerate() {
            use std::fmt::Write as _;
            let _ = writeln!(output, "operation_outcome.{index}={engine:?}|{outcome:?}");
        }
        output
    }
}

fn safe_version(value: &str) -> &str {
    if !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
    {
        value
    } else {
        "invalid"
    }
}

const fn os_label(value: SupportOperatingSystem) -> &'static str {
    match value {
        SupportOperatingSystem::MacOs => "macos",
        SupportOperatingSystem::Linux => "linux",
        SupportOperatingSystem::Windows => "windows",
        SupportOperatingSystem::Other => "other",
    }
}

const fn architecture_label(value: SupportArchitecture) -> &'static str {
    match value {
        SupportArchitecture::Arm64 => "arm64",
        SupportArchitecture::X86_64 => "x86_64",
        SupportArchitecture::Other => "other",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportBundleError {
    DiagnosticLimit,
}

impl fmt::Display for SupportBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("support bundle diagnostic limit reached")
    }
}

impl Error for SupportBundleError {}
