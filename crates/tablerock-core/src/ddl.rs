//! Reviewed DDL operation plans (PostgreSQL-first).
//!
//! SQL is never built by concatenating unquoted identifiers. Execution uses
//! quote_ident at the engine boundary. ClickHouse/Redis get explicit
//! unsupported states.

use std::fmt;

use crate::{Engine, OperationScope, Revision};

/// Capability-gated DDL kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DdlKind {
    AddColumn,
    DropColumn,
    CreateIndex,
    DropIndex,
    AddConstraint,
    DropConstraint,
    Vacuum,
    Analyze,
    Reindex,
    Optimize, // ClickHouse
}

impl DdlKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AddColumn => "add_column",
            Self::DropColumn => "drop_column",
            Self::CreateIndex => "create_index",
            Self::DropIndex => "drop_index",
            Self::AddConstraint => "add_constraint",
            Self::DropConstraint => "drop_constraint",
            Self::Vacuum => "vacuum",
            Self::Analyze => "analyze",
            Self::Reindex => "reindex",
            Self::Optimize => "optimize",
        }
    }

    /// Engines that support this kind (others → Unsupported).
    #[must_use]
    pub const fn engines(self) -> &'static [Engine] {
        match self {
            Self::AddColumn
            | Self::DropColumn
            | Self::CreateIndex
            | Self::DropIndex
            | Self::AddConstraint
            | Self::DropConstraint
            | Self::Vacuum
            | Self::Analyze
            | Self::Reindex => &[Engine::PostgreSql],
            Self::Optimize => &[Engine::ClickHouse],
        }
    }

    #[must_use]
    pub fn supports(self, engine: Engine) -> bool {
        self.engines().contains(&engine)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdlTarget {
    PostgreSqlRelation { schema: String, relation: String },
    ClickHouseTable { database: String, table: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdlPlan {
    pub kind: DdlKind,
    pub engine: Engine,
    pub scope: OperationScope,
    pub revision: Revision,
    pub target: DdlTarget,
    /// Column/index/constraint name when applicable.
    pub object_name: Option<String>,
    /// Type text for AddColumn (engine validates).
    pub type_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdlBuildError {
    UnsupportedEngine,
    EmptyIdentifier,
    MissingType,
}

impl fmt::Display for DdlBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedEngine => "DDL kind unsupported on this engine",
            Self::EmptyIdentifier => "DDL identifier empty",
            Self::MissingType => "ADD COLUMN requires a type",
        })
    }
}

impl DdlPlan {
    pub fn new(
        kind: DdlKind,
        engine: Engine,
        scope: OperationScope,
        revision: Revision,
        target: DdlTarget,
        object_name: Option<String>,
        type_text: Option<String>,
    ) -> Result<Self, DdlBuildError> {
        if !kind.supports(engine) {
            return Err(DdlBuildError::UnsupportedEngine);
        }
        if let Some(name) = &object_name
            && name.trim().is_empty()
        {
            return Err(DdlBuildError::EmptyIdentifier);
        }
        if kind == DdlKind::AddColumn
            && type_text
                .as_ref()
                .map(|t| t.trim().is_empty())
                .unwrap_or(true)
        {
            return Err(DdlBuildError::MissingType);
        }
        if matches!(
            kind,
            DdlKind::CreateIndex
                | DdlKind::DropIndex
                | DdlKind::AddConstraint
                | DdlKind::DropConstraint
        ) && object_name
            .as_ref()
            .map(|n| n.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(DdlBuildError::EmptyIdentifier);
        }
        if matches!(kind, DdlKind::CreateIndex | DdlKind::AddConstraint)
            && type_text
                .as_ref()
                .map(|t| t.trim().is_empty())
                .unwrap_or(true)
        {
            // CreateIndex: type_text = column list; AddConstraint: type_text = clause body.
            return Err(DdlBuildError::MissingType);
        }
        match &target {
            DdlTarget::PostgreSqlRelation { schema, relation } => {
                if schema.trim().is_empty() || relation.trim().is_empty() {
                    return Err(DdlBuildError::EmptyIdentifier);
                }
            }
            DdlTarget::ClickHouseTable { database, table } => {
                if database.trim().is_empty() || table.trim().is_empty() {
                    return Err(DdlBuildError::EmptyIdentifier);
                }
            }
        }
        Ok(Self {
            kind,
            engine,
            scope,
            revision,
            target,
            object_name,
            type_text,
        })
    }

    /// Descriptive preview only — execution quotes idents at the engine.
    #[must_use]
    pub fn preview_label(&self) -> String {
        let target = match &self.target {
            DdlTarget::PostgreSqlRelation { schema, relation } => format!("{schema}.{relation}"),
            DdlTarget::ClickHouseTable { database, table } => format!("{database}.{table}"),
        };
        let obj = self.object_name.as_deref().unwrap_or("—");
        format!(
            "{} on {} object={} (typed plan; not free SQL)",
            self.kind.label(),
            target,
            obj
        )
    }
}

/// FK graph edge for relationship exploration (terminal tree/list first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipEdge {
    pub from_schema: String,
    pub from_table: String,
    pub from_column: String,
    pub to_schema: String,
    pub to_table: String,
    pub to_column: String,
}

/// Bounded relationship graph projection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RelationshipGraph {
    pub edges: Vec<RelationshipEdge>,
}

impl RelationshipGraph {
    pub fn push(&mut self, edge: RelationshipEdge) {
        self.edges.push(edge);
    }

    /// Detect simple self-cycles (table references itself).
    #[must_use]
    pub fn self_cycles(&self) -> Vec<&RelationshipEdge> {
        self.edges
            .iter()
            .filter(|e| e.from_schema == e.to_schema && e.from_table == e.to_table)
            .collect()
    }

    /// Outbound neighbors of a table.
    #[must_use]
    pub fn outbound(&self, schema: &str, table: &str) -> Vec<&RelationshipEdge> {
        self.edges
            .iter()
            .filter(|e| e.from_schema == schema && e.from_table == table)
            .collect()
    }
}

/// Role/privilege inspection row (read-only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolePrivilegeRow {
    pub grantee: String,
    pub privilege: String,
    pub object: String,
    pub is_grantable: bool,
}

/// Direct membership edge: `member` is granted `role` (`GRANT role TO member`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMembershipEdge {
    pub role: String,
    pub member: String,
}

/// Bounded role membership graph for effective-privilege expansion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoleMembershipGraph {
    pub edges: Vec<RoleMembershipEdge>,
}

impl RoleMembershipGraph {
    pub fn push(&mut self, edge: RoleMembershipEdge) {
        self.edges.push(edge);
    }

    /// Roles that `member` is a direct member of.
    #[must_use]
    pub fn direct_roles_of(&self, member: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.member == member)
            .map(|e| e.role.as_str())
            .collect()
    }

    /// Transitive roles for `member` including itself (BFS, cycle-safe).
    ///
    /// When a cycle is hit, expansion stops that branch; `cycles` lists pairs
    /// `(from, to)` observed on a revisit.
    #[must_use]
    pub fn effective_roles(
        &self,
        member: &str,
        max_roles: usize,
    ) -> (Vec<String>, Vec<(String, String)>) {
        use std::collections::{HashSet, VecDeque};
        let mut seen = HashSet::new();
        let mut order = Vec::new();
        let mut cycles = Vec::new();
        let mut queue = VecDeque::new();
        seen.insert(member.to_owned());
        order.push(member.to_owned());
        queue.push_back(member.to_owned());
        while let Some(current) = queue.pop_front() {
            if order.len() >= max_roles {
                break;
            }
            for role in self.direct_roles_of(&current) {
                if !seen.insert(role.to_owned()) {
                    // Revisit while expanding → cycle/path merge.
                    if role != member {
                        cycles.push((current.clone(), role.to_owned()));
                    }
                    continue;
                }
                order.push(role.to_owned());
                queue.push_back(role.to_owned());
                if order.len() >= max_roles {
                    break;
                }
            }
        }
        (order, cycles)
    }

    /// True when expanding `member` would re-enter itself through grants
    /// (self-lockout / circular grant risk signal for review UI).
    #[must_use]
    pub fn has_self_cycle_through(&self, member: &str) -> bool {
        use std::collections::{HashSet, VecDeque};
        let mut seen = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(member.to_owned());
        while let Some(current) = queue.pop_front() {
            for role in self.direct_roles_of(&current) {
                if role == member {
                    return true;
                }
                if seen.insert(role.to_owned()) {
                    queue.push_back(role.to_owned());
                }
            }
        }
        false
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
    fn redis_add_column_unsupported() {
        assert!(matches!(
            DdlPlan::new(
                DdlKind::AddColumn,
                Engine::Redis,
                scope(),
                Revision::INITIAL,
                DdlTarget::PostgreSqlRelation {
                    schema: "public".into(),
                    relation: "t".into(),
                },
                Some("c".into()),
                Some("int".into()),
            ),
            Err(DdlBuildError::UnsupportedEngine)
        ));
    }

    #[test]
    fn pg_add_column_preview() {
        let p = DdlPlan::new(
            DdlKind::AddColumn,
            Engine::PostgreSql,
            scope(),
            Revision::INITIAL,
            DdlTarget::PostgreSqlRelation {
                schema: "public".into(),
                relation: "users".into(),
            },
            Some("email".into()),
            Some("text".into()),
        )
        .unwrap();
        let prev = p.preview_label();
        assert!(prev.contains("add_column"));
        assert!(prev.contains("users"));
        assert!(!prev.to_ascii_lowercase().contains("execute"));
    }

    #[test]
    fn create_index_requires_name_and_columns() {
        assert!(matches!(
            DdlPlan::new(
                DdlKind::CreateIndex,
                Engine::PostgreSql,
                scope(),
                Revision::INITIAL,
                DdlTarget::PostgreSqlRelation {
                    schema: "public".into(),
                    relation: "t".into(),
                },
                None,
                Some("c".into()),
            ),
            Err(DdlBuildError::EmptyIdentifier)
        ));
        assert!(matches!(
            DdlPlan::new(
                DdlKind::CreateIndex,
                Engine::PostgreSql,
                scope(),
                Revision::INITIAL,
                DdlTarget::PostgreSqlRelation {
                    schema: "public".into(),
                    relation: "t".into(),
                },
                Some("i".into()),
                None,
            ),
            Err(DdlBuildError::MissingType)
        ));
        assert!(
            DdlPlan::new(
                DdlKind::CreateIndex,
                Engine::PostgreSql,
                scope(),
                Revision::INITIAL,
                DdlTarget::PostgreSqlRelation {
                    schema: "public".into(),
                    relation: "t".into(),
                },
                Some("i".into()),
                Some("c".into()),
            )
            .is_ok()
        );
    }

    #[test]
    fn role_membership_effective_roles_and_self_cycle() {
        let mut g = RoleMembershipGraph::default();
        // child → parent → grand (child is member of parent, parent of grand)
        g.push(RoleMembershipEdge {
            role: "parent".into(),
            member: "child".into(),
        });
        g.push(RoleMembershipEdge {
            role: "grand".into(),
            member: "parent".into(),
        });
        // cycle: grand also member of child
        g.push(RoleMembershipEdge {
            role: "child".into(),
            member: "grand".into(),
        });
        let (roles, _cycles) = g.effective_roles("child", 16);
        assert!(roles.contains(&"child".into()));
        assert!(roles.contains(&"parent".into()));
        assert!(roles.contains(&"grand".into()));
        assert!(g.has_self_cycle_through("child"));
        assert!(!g.has_self_cycle_through("lonely"));
        let (bounded, _) = g.effective_roles("child", 2);
        assert_eq!(bounded.len(), 2);
    }

    #[test]
    fn relationship_graph_cycle_and_outbound() {
        let mut g = RelationshipGraph::default();
        g.push(RelationshipEdge {
            from_schema: "public".into(),
            from_table: "users".into(),
            from_column: "manager_id".into(),
            to_schema: "public".into(),
            to_table: "users".into(),
            to_column: "id".into(),
        });
        g.push(RelationshipEdge {
            from_schema: "public".into(),
            from_table: "orders".into(),
            from_column: "user_id".into(),
            to_schema: "public".into(),
            to_table: "users".into(),
            to_column: "id".into(),
        });
        assert_eq!(g.self_cycles().len(), 1);
        assert_eq!(g.outbound("public", "orders").len(), 1);
    }

    #[test]
    fn relationship_graph_replays_large_cycles_without_recursion() {
        let mut graph = RelationshipGraph::default();
        for index in 0..4_096 {
            graph.push(RelationshipEdge {
                from_schema: "public".into(),
                from_table: format!("node_{index}"),
                from_column: "parent_id".into(),
                to_schema: "public".into(),
                to_table: format!("node_{}", (index + 1) % 4_096),
                to_column: "id".into(),
            });
        }
        graph.push(RelationshipEdge {
            from_schema: "public".into(),
            from_table: "node_0".into(),
            from_column: "self_id".into(),
            to_schema: "public".into(),
            to_table: "node_0".into(),
            to_column: "id".into(),
        });
        assert_eq!(graph.edges.len(), 4_097);
        assert_eq!(graph.self_cycles().len(), 1);
        assert_eq!(graph.outbound("public", "node_0").len(), 2);
    }
}
