use crate::{Engine, OperationScope, ReviewTokenId, Revision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleChangeKind {
    GrantMembership {
        role: String,
        member: String,
    },
    RevokeMembership {
        role: String,
        member: String,
    },
    GrantTablePrivilege {
        schema: String,
        table: String,
        grantee: String,
        privilege: String,
    },
    RevokeTablePrivilege {
        schema: String,
        table: String,
        grantee: String,
        privilege: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleChangePlan {
    kind: RoleChangeKind,
    scope: OperationScope,
    revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleChangeError {
    InvalidIdentifier,
    InvalidPrivilege,
    EngineMismatch,
    SelfLockout,
    InvalidExpiry,
    Expired,
    ScopeMismatch,
    RevisionMismatch,
}

impl RoleChangePlan {
    pub fn new(
        engine: Engine,
        scope: OperationScope,
        revision: Revision,
        current_user: &str,
        kind: RoleChangeKind,
    ) -> Result<Self, RoleChangeError> {
        if engine != Engine::PostgreSql {
            return Err(RoleChangeError::EngineMismatch);
        }
        let valid =
            |value: &str| !value.is_empty() && value.len() <= 256 && !value.as_bytes().contains(&0);
        let valid_privilege = |value: &str| {
            matches!(
                value,
                "SELECT" | "INSERT" | "UPDATE" | "DELETE" | "TRUNCATE" | "REFERENCES" | "TRIGGER"
            )
        };
        match &kind {
            RoleChangeKind::GrantMembership { role, member } => {
                if !valid(role) || !valid(member) {
                    return Err(RoleChangeError::InvalidIdentifier);
                }
            }
            RoleChangeKind::RevokeMembership { role, member } => {
                if !valid(role) || !valid(member) {
                    return Err(RoleChangeError::InvalidIdentifier);
                }
                if member == current_user {
                    return Err(RoleChangeError::SelfLockout);
                }
            }
            RoleChangeKind::GrantTablePrivilege {
                schema,
                table,
                grantee,
                privilege,
            } => {
                if !valid(schema) || !valid(table) || !valid(grantee) {
                    return Err(RoleChangeError::InvalidIdentifier);
                }
                if !valid_privilege(privilege) {
                    return Err(RoleChangeError::InvalidPrivilege);
                }
            }
            RoleChangeKind::RevokeTablePrivilege {
                schema,
                table,
                grantee,
                privilege,
            } => {
                if !valid(schema) || !valid(table) || !valid(grantee) {
                    return Err(RoleChangeError::InvalidIdentifier);
                }
                if !valid_privilege(privilege) {
                    return Err(RoleChangeError::InvalidPrivilege);
                }
                if grantee == current_user {
                    return Err(RoleChangeError::SelfLockout);
                }
            }
        }
        Ok(Self {
            kind,
            scope,
            revision,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> &RoleChangeKind {
        &self.kind
    }
    #[must_use]
    pub const fn scope(&self) -> OperationScope {
        self.scope
    }
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    pub fn review(
        self,
        token_id: ReviewTokenId,
        issued_at_ms: u64,
        expires_at_ms: u64,
    ) -> Result<ReviewedRoleChangePlan, RoleChangeError> {
        if expires_at_ms <= issued_at_ms {
            return Err(RoleChangeError::InvalidExpiry);
        }
        Ok(ReviewedRoleChangePlan {
            plan: self,
            token_id,
            issued_at_ms,
            expires_at_ms,
        })
    }
}

pub struct ReviewedRoleChangePlan {
    plan: RoleChangePlan,
    token_id: ReviewTokenId,
    issued_at_ms: u64,
    expires_at_ms: u64,
}

impl ReviewedRoleChangePlan {
    pub fn authorize(
        self,
        now_ms: u64,
        scope: OperationScope,
        revision: Revision,
    ) -> Result<AuthorizedRoleChangePlan, RoleChangeError> {
        if now_ms < self.issued_at_ms || now_ms >= self.expires_at_ms {
            return Err(RoleChangeError::Expired);
        }
        if self.plan.scope != scope {
            return Err(RoleChangeError::ScopeMismatch);
        }
        if self.plan.revision != revision {
            return Err(RoleChangeError::RevisionMismatch);
        }
        Ok(AuthorizedRoleChangePlan {
            plan: self.plan,
            token_id: self.token_id,
        })
    }
}

pub struct AuthorizedRoleChangePlan {
    plan: RoleChangePlan,
    token_id: ReviewTokenId,
}

impl AuthorizedRoleChangePlan {
    #[must_use]
    pub const fn plan(&self) -> &RoleChangePlan {
        &self.plan
    }
    #[must_use]
    pub const fn token_id(&self) -> ReviewTokenId {
        self.token_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContextId, IdParts, ProfileId, SessionId};

    fn scope() -> OperationScope {
        OperationScope::new(
            ProfileId::from_parts(IdParts::new(1, 1).unwrap()).unwrap(),
            SessionId::from_parts(IdParts::new(1, 2).unwrap()).unwrap(),
            ContextId::from_parts(IdParts::new(1, 3).unwrap()).unwrap(),
        )
    }

    #[test]
    fn blocks_self_lockout_and_unknown_privileges() {
        let revoke_self = RoleChangePlan::new(
            Engine::PostgreSql,
            scope(),
            Revision::INITIAL,
            "operator",
            RoleChangeKind::RevokeMembership {
                role: "reader".into(),
                member: "operator".into(),
            },
        );
        assert_eq!(revoke_self.unwrap_err(), RoleChangeError::SelfLockout);
        let hostile = RoleChangePlan::new(
            Engine::PostgreSql,
            scope(),
            Revision::INITIAL,
            "operator",
            RoleChangeKind::GrantTablePrivilege {
                schema: "public".into(),
                table: "items".into(),
                grantee: "reader".into(),
                privilege: "ALL; DROP TABLE items".into(),
            },
        );
        assert_eq!(hostile.unwrap_err(), RoleChangeError::InvalidPrivilege);
    }
}
