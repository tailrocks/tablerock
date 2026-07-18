//! Resolve profile secret sources for connect/test. No network I/O except
//! the bounded local `op read` subprocess for 1Password references.

use std::{
    error::Error,
    fmt,
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use tablerock_core::{
    OnePasswordReference, ProfileName, ProfileProperty, ProfilePropertyBinding, SecretField,
    SecretSource, SecretSourceKind,
};
use zeroize::Zeroize;

/// Zeroizing secret material for a single connect attempt. Never cloned.
pub struct ResolvedSecret {
    bytes: Vec<u8>,
    field: SecretField,
}

impl ResolvedSecret {
    fn new(bytes: Vec<u8>, field: SecretField) -> Self {
        Self { bytes, field }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub const fn field(&self) -> SecretField {
        self.field
    }
}

impl Drop for ResolvedSecret {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl fmt::Debug for ResolvedSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedSecret")
            .field("field", &self.field)
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretResolutionError {
    SourceNotYetSupported { kind: SecretSourceKindLabel },
    PromptFailed,
    MissingSource,
    /// Named environment variable is unset or empty (fail closed).
    EnvVarMissing,
    /// `op` binary not found on PATH.
    OnePasswordCliMissing,
    /// `op read` exited non-zero or could not be spawned/read (redacted).
    OnePasswordFailed,
    /// `op read` produced empty stdout.
    OnePasswordEmpty,
    /// `op read` exceeded the bounded wait.
    OnePasswordTimeout,
    /// `op read` stdout exceeded the byte cap.
    OnePasswordOutputTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSourceKindLabel {
    OnePassword,
    HostEnvironment,
    Keychain,
    PromptOnConnect,
    DangerousPlaintext,
}

impl fmt::Display for SecretResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SourceNotYetSupported { .. } => {
                "secret source is not supported in this delivery stage"
            }
            Self::PromptFailed => "secret prompt failed",
            Self::MissingSource => "property has no secret source to resolve",
            Self::EnvVarMissing => "environment variable is unset or empty",
            Self::OnePasswordCliMissing => "1Password CLI (op) not found on PATH",
            Self::OnePasswordFailed => "1Password CLI read failed",
            Self::OnePasswordEmpty => "1Password CLI returned an empty secret",
            Self::OnePasswordTimeout => "1Password CLI read timed out",
            Self::OnePasswordOutputTooLarge => "1Password CLI output exceeded size limit",
        })
    }
}

impl Error for SecretResolutionError {}

/// UI/OS port for prompt-on-connect values. Engine never owns the prompt UX.
pub trait SecretPromptPort: Send {
    fn request(
        &mut self,
        field: SecretField,
        profile: &ProfileName,
    ) -> Result<ResolvedSecret, SecretResolutionError>;
}

/// Port for account-pinned `op read`. Default implementation spawns the CLI.
pub trait OnePasswordReadPort: Send {
    fn read(
        &mut self,
        reference: &OnePasswordReference,
    ) -> Result<Vec<u8>, SecretResolutionError>;
}

/// Bounded `op read --account <id> --no-newline <uri>` implementation.
#[derive(Debug, Clone)]
pub struct OpCliReader {
    /// Path or program name; defaults to `op`.
    pub program: String,
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for OpCliReader {
    fn default() -> Self {
        Self {
            program: "op".into(),
            timeout: Duration::from_secs(30),
            max_output_bytes: 256 * 1024,
        }
    }
}

impl OnePasswordReadPort for OpCliReader {
    fn read(
        &mut self,
        reference: &OnePasswordReference,
    ) -> Result<Vec<u8>, SecretResolutionError> {
        let uri = reference.secret_reference_uri();
        let mut child = Command::new(&self.program)
            .args([
                "read",
                "--account",
                reference.account_id().as_str(),
                "--no-newline",
                &uri,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| SecretResolutionError::OnePasswordCliMissing)?;

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stdout = Vec::new();
                    if let Some(mut pipe) = child.stdout.take() {
                        // Cap read; drop remainder if too large.
                        let mut buf = vec![0_u8; 8 * 1024];
                        loop {
                            match pipe.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if stdout.len() + n > self.max_output_bytes {
                                        stdout.zeroize();
                                        buf.zeroize();
                                        return Err(SecretResolutionError::OnePasswordOutputTooLarge);
                                    }
                                    stdout.extend_from_slice(&buf[..n]);
                                }
                                Err(_) => {
                                    stdout.zeroize();
                                    return Err(SecretResolutionError::OnePasswordFailed);
                                }
                            }
                        }
                        buf.zeroize();
                    }
                    // Drain stderr without retaining content (may name items).
                    if let Some(mut err) = child.stderr.take() {
                        let mut sink = Vec::new();
                        let _ = err.read_to_end(&mut sink);
                        sink.zeroize();
                    }
                    if !status.success() {
                        stdout.zeroize();
                        return Err(SecretResolutionError::OnePasswordFailed);
                    }
                    if stdout.is_empty() {
                        return Err(SecretResolutionError::OnePasswordEmpty);
                    }
                    return Ok(stdout);
                }
                Ok(None) if started.elapsed() >= self.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(SecretResolutionError::OnePasswordTimeout);
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => {
                    let _ = child.kill();
                    return Err(SecretResolutionError::OnePasswordFailed);
                }
            }
        }
    }
}

/// Resolve a binding for connect/test. Literals return `None` (not secrets).
/// Unsupported kinds fail closed before any network I/O.
pub fn resolve_for_connect(
    binding: &ProfilePropertyBinding,
    profile: &ProfileName,
    prompt: &mut dyn SecretPromptPort,
) -> Result<Option<ResolvedSecret>, SecretResolutionError> {
    let mut op = OpCliReader::default();
    resolve_for_connect_with(binding, profile, prompt, &mut op)
}

/// Resolve with an injectable 1Password port (tests + custom runners).
pub fn resolve_for_connect_with(
    binding: &ProfilePropertyBinding,
    profile: &ProfileName,
    prompt: &mut dyn SecretPromptPort,
    op: &mut dyn OnePasswordReadPort,
) -> Result<Option<ResolvedSecret>, SecretResolutionError> {
    match binding.secret_source() {
        None => Ok(None),
        Some(source) => resolve_source(source, binding.property(), profile, prompt, op).map(Some),
    }
}

fn resolve_source(
    source: &SecretSource,
    property: ProfileProperty,
    profile: &ProfileName,
    prompt: &mut dyn SecretPromptPort,
    op: &mut dyn OnePasswordReadPort,
) -> Result<ResolvedSecret, SecretResolutionError> {
    let field = secret_field_for(property);
    match source.kind() {
        SecretSourceKind::PromptOnConnect => prompt.request(field, profile),
        SecretSourceKind::DangerousPlaintext(plaintext) => {
            Ok(ResolvedSecret::new(plaintext.bytes().to_vec(), field))
        }
        SecretSourceKind::OnePassword(reference) => {
            let bytes = op.read(reference)?;
            Ok(ResolvedSecret::new(bytes, field))
        }
        SecretSourceKind::HostEnvironment(env) => resolve_host_environment(env.as_str(), field),
        SecretSourceKind::Keychain(_) => Err(SecretResolutionError::SourceNotYetSupported {
            kind: SecretSourceKindLabel::Keychain,
        }),
    }
}

/// Read host env at connect time. Never logs the value. Missing/empty fail closed.
fn resolve_host_environment(
    name: &str,
    field: SecretField,
) -> Result<ResolvedSecret, SecretResolutionError> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(ResolvedSecret::new(value.into_bytes(), field)),
        Ok(_) | Err(_) => Err(SecretResolutionError::EnvVarMissing),
    }
}

const fn secret_field_for(_property: ProfileProperty) -> SecretField {
    // Core SecretField labels are source-reference fields; password material
    // uses the dangerous-plaintext field tag for resolution diagnostics only.
    SecretField::DangerousPlaintext
}

#[cfg(test)]
mod tests {
    use super::*;
    use tablerock_core::{
        BoundedBytes, BoundedText, ByteLimit, DangerousPlaintext, EnvironmentReference,
        KeychainReference, OnePasswordObjectId, OnePasswordReference, OnePasswordSegment,
        PlaintextAcknowledgement, ProfileProperty, ProfilePropertyBinding, SecretSource,
    };

    struct CountingPrompt {
        calls: u32,
        value: Vec<u8>,
    }

    impl SecretPromptPort for CountingPrompt {
        fn request(
            &mut self,
            field: SecretField,
            _profile: &ProfileName,
        ) -> Result<ResolvedSecret, SecretResolutionError> {
            self.calls += 1;
            Ok(ResolvedSecret::new(self.value.clone(), field))
        }
    }

    struct MockOp {
        result: Result<Vec<u8>, SecretResolutionError>,
        calls: u32,
    }

    impl OnePasswordReadPort for MockOp {
        fn read(
            &mut self,
            _reference: &OnePasswordReference,
        ) -> Result<Vec<u8>, SecretResolutionError> {
            self.calls += 1;
            self.result.clone()
        }
    }

    fn name() -> ProfileName {
        ProfileName::new(BoundedText::copy_from_str("demo", ByteLimit::new(32)).unwrap()).unwrap()
    }

    fn sample_op_ref() -> OnePasswordReference {
        OnePasswordReference::new(
            OnePasswordObjectId::parse("aaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
            OnePasswordObjectId::parse("bbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
            OnePasswordObjectId::parse("cccccccccccccccccccccccccc").unwrap(),
            None,
            OnePasswordSegment::parse("password").unwrap(),
            BoundedText::copy_from_str("password", ByteLimit::new(32)).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn plaintext_resolves_and_redacts_debug() {
        let plaintext = DangerousPlaintext::new(
            b"super-secret".to_vec(),
            PlaintextAcknowledgement::LocalTestingOnly,
        )
        .unwrap();
        let binding = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::DangerousPlaintext(plaintext)),
        );
        let mut prompt = CountingPrompt {
            calls: 0,
            value: Vec::new(),
        };
        let mut op = MockOp {
            result: Err(SecretResolutionError::OnePasswordFailed),
            calls: 0,
        };
        let resolved = resolve_for_connect_with(&binding, &name(), &mut prompt, &mut op)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.as_bytes(), b"super-secret");
        assert_eq!(prompt.calls, 0);
        assert_eq!(op.calls, 0);
        let debug = format!("{resolved:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("byte_len"));
    }

    #[test]
    fn prompt_port_called_once() {
        let binding = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::PromptOnConnect),
        );
        let mut prompt = CountingPrompt {
            calls: 0,
            value: b"typed".to_vec(),
        };
        let mut op = MockOp {
            result: Err(SecretResolutionError::OnePasswordFailed),
            calls: 0,
        };
        let resolved = resolve_for_connect_with(&binding, &name(), &mut prompt, &mut op)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.as_bytes(), b"typed");
        assert_eq!(prompt.calls, 1);
        assert_eq!(op.calls, 0);
    }

    #[test]
    fn host_environment_resolves_and_missing_fails() {
        // Use a host var that is always present (no set_var; unsafe_code forbidden).
        let var = if std::env::var_os("PATH").is_some() {
            "PATH"
        } else if std::env::var_os("HOME").is_some() {
            "HOME"
        } else {
            let missing =
                EnvironmentReference::parse("TABLEROCK_TEST_SECRET_MISSING_XYZ").unwrap();
            let binding_missing = ProfilePropertyBinding::secret(
                ProfileProperty::Password,
                SecretSource::new(SecretSourceKind::HostEnvironment(missing)),
            );
            let mut prompt = CountingPrompt {
                calls: 0,
                value: Vec::new(),
            };
            let mut op = MockOp {
                result: Err(SecretResolutionError::OnePasswordFailed),
                calls: 0,
            };
            assert!(matches!(
                resolve_for_connect_with(&binding_missing, &name(), &mut prompt, &mut op),
                Err(SecretResolutionError::EnvVarMissing)
            ));
            return;
        };
        let expected = std::env::var(var).expect("chosen host env present");
        let env = EnvironmentReference::parse(var).unwrap();
        let binding = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::HostEnvironment(env)),
        );
        let mut prompt = CountingPrompt {
            calls: 0,
            value: Vec::new(),
        };
        let mut op = MockOp {
            result: Err(SecretResolutionError::OnePasswordFailed),
            calls: 0,
        };
        let resolved = resolve_for_connect_with(&binding, &name(), &mut prompt, &mut op)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.as_bytes(), expected.as_bytes());
        assert_eq!(prompt.calls, 0);
        let debug = format!("{resolved:?}");
        assert!(debug.contains("byte_len"));
        assert!(!debug.contains(&format!("bytes: {expected:?}")));

        let missing = EnvironmentReference::parse("TABLEROCK_TEST_SECRET_MISSING_XYZ").unwrap();
        let binding_missing = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::HostEnvironment(missing)),
        );
        assert!(matches!(
            resolve_for_connect_with(&binding_missing, &name(), &mut prompt, &mut op),
            Err(SecretResolutionError::EnvVarMissing)
        ));
    }

    #[test]
    fn one_password_resolves_via_port() {
        let binding = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::OnePassword(sample_op_ref())),
        );
        let mut prompt = CountingPrompt {
            calls: 0,
            value: Vec::new(),
        };
        let mut op = MockOp {
            result: Ok(b"from-op".to_vec()),
            calls: 0,
        };
        let resolved = resolve_for_connect_with(&binding, &name(), &mut prompt, &mut op)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.as_bytes(), b"from-op");
        assert_eq!(op.calls, 1);
        assert_eq!(prompt.calls, 0);
        let debug = format!("{resolved:?}");
        assert!(!debug.contains("from-op"));
        assert!(debug.contains("byte_len"));
    }

    #[test]
    fn one_password_port_failure_is_fail_closed() {
        let binding = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::OnePassword(sample_op_ref())),
        );
        let mut prompt = CountingPrompt {
            calls: 0,
            value: Vec::new(),
        };
        let mut op = MockOp {
            result: Err(SecretResolutionError::OnePasswordCliMissing),
            calls: 0,
        };
        assert!(matches!(
            resolve_for_connect_with(&binding, &name(), &mut prompt, &mut op),
            Err(SecretResolutionError::OnePasswordCliMissing)
        ));
        assert_eq!(op.calls, 1);
    }

    #[test]
    fn keychain_still_unsupported() {
        let key = KeychainReference::new(
            BoundedBytes::copy_from_slice(b"service/account", ByteLimit::new(64)).unwrap(),
        )
        .unwrap();
        let binding = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::Keychain(key)),
        );
        let mut prompt = CountingPrompt {
            calls: 0,
            value: Vec::new(),
        };
        let mut op = MockOp {
            result: Ok(b"x".to_vec()),
            calls: 0,
        };
        assert!(matches!(
            resolve_for_connect_with(&binding, &name(), &mut prompt, &mut op),
            Err(SecretResolutionError::SourceNotYetSupported {
                kind: SecretSourceKindLabel::Keychain
            })
        ));
        assert_eq!(op.calls, 0);
    }

    #[test]
    fn op_cli_missing_program_fails_closed() {
        let mut reader = OpCliReader {
            program: "/nonexistent/tablerock-op-missing".into(),
            timeout: Duration::from_secs(1),
            max_output_bytes: 1024,
        };
        assert!(matches!(
            reader.read(&sample_op_ref()),
            Err(SecretResolutionError::OnePasswordCliMissing)
        ));
    }

    #[test]
    fn literal_binding_needs_no_resolution() {
        let binding = ProfilePropertyBinding::literal(
            ProfileProperty::Host,
            BoundedText::copy_from_str("db.internal", ByteLimit::new(64)).unwrap(),
        )
        .unwrap();
        let mut prompt = CountingPrompt {
            calls: 0,
            value: Vec::new(),
        };
        let mut op = MockOp {
            result: Ok(Vec::new()),
            calls: 0,
        };
        assert!(
            resolve_for_connect_with(&binding, &name(), &mut prompt, &mut op)
                .unwrap()
                .is_none()
        );
        assert_eq!(prompt.calls, 0);
        assert_eq!(op.calls, 0);
    }
}
