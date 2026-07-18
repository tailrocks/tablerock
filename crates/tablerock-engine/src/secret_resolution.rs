//! Resolve profile secret sources for connect/test. No network I/O.

use std::{error::Error, fmt};

use tablerock_core::{
    ProfileName, ProfileProperty, ProfilePropertyBinding, SecretField, SecretSource,
    SecretSourceKind,
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

/// Resolve a binding for connect/test. Literals return `None` (not secrets).
/// Unsupported kinds fail closed before any network I/O.
pub fn resolve_for_connect(
    binding: &ProfilePropertyBinding,
    profile: &ProfileName,
    prompt: &mut dyn SecretPromptPort,
) -> Result<Option<ResolvedSecret>, SecretResolutionError> {
    match binding.secret_source() {
        None => Ok(None),
        Some(source) => resolve_source(source, binding.property(), profile, prompt).map(Some),
    }
}

fn resolve_source(
    source: &SecretSource,
    property: ProfileProperty,
    profile: &ProfileName,
    prompt: &mut dyn SecretPromptPort,
) -> Result<ResolvedSecret, SecretResolutionError> {
    let field = secret_field_for(property);
    match source.kind() {
        SecretSourceKind::PromptOnConnect => prompt.request(field, profile),
        SecretSourceKind::DangerousPlaintext(plaintext) => {
            Ok(ResolvedSecret::new(plaintext.bytes().to_vec(), field))
        }
        SecretSourceKind::OnePassword(_) => Err(SecretResolutionError::SourceNotYetSupported {
            kind: SecretSourceKindLabel::OnePassword,
        }),
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
        Ok(value) if !value.is_empty() => {
            Ok(ResolvedSecret::new(value.into_bytes(), field))
        }
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
        BoundedText, ByteLimit, DangerousPlaintext, PlaintextAcknowledgement, ProfileProperty,
        ProfilePropertyBinding, SecretSource,
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

    fn name() -> ProfileName {
        ProfileName::new(BoundedText::copy_from_str("demo", ByteLimit::new(32)).unwrap()).unwrap()
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
        let resolved = resolve_for_connect(&binding, &name(), &mut prompt)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.as_bytes(), b"super-secret");
        assert_eq!(prompt.calls, 0);
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
        let resolved = resolve_for_connect(&binding, &name(), &mut prompt)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.as_bytes(), b"typed");
        assert_eq!(prompt.calls, 1);
    }

    #[test]
    fn host_environment_resolves_and_missing_fails() {
        use tablerock_core::EnvironmentReference;

        // Use a host var that is always present (no set_var; unsafe_code forbidden).
        let var = if std::env::var_os("PATH").is_some() {
            "PATH"
        } else if std::env::var_os("HOME").is_some() {
            "HOME"
        } else {
            // Extremely constrained host: skip positive path; still test missing.
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
            assert!(matches!(
                resolve_for_connect(&binding_missing, &name(), &mut prompt),
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
        let resolved = resolve_for_connect(&binding, &name(), &mut prompt)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.as_bytes(), expected.as_bytes());
        assert_eq!(prompt.calls, 0);
        let debug = format!("{resolved:?}");
        // Debug must not dump secret material as a free string field.
        assert!(debug.contains("byte_len"));
        assert!(!debug.contains(&format!("bytes: {expected:?}")));

        let missing = EnvironmentReference::parse("TABLEROCK_TEST_SECRET_MISSING_XYZ").unwrap();
        let binding_missing = ProfilePropertyBinding::secret(
            ProfileProperty::Password,
            SecretSource::new(SecretSourceKind::HostEnvironment(missing)),
        );
        assert!(matches!(
            resolve_for_connect(&binding_missing, &name(), &mut prompt),
            Err(SecretResolutionError::EnvVarMissing)
        ));
    }

    #[test]
    fn keychain_still_unsupported() {
        use tablerock_core::{BoundedBytes, ByteLimit, KeychainReference};

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
        assert!(matches!(
            resolve_for_connect(&binding, &name(), &mut prompt),
            Err(SecretResolutionError::SourceNotYetSupported {
                kind: SecretSourceKindLabel::Keychain
            })
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
        assert!(
            resolve_for_connect(&binding, &name(), &mut prompt)
                .unwrap()
                .is_none()
        );
        assert_eq!(prompt.calls, 0);
    }
}
