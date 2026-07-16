use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    BoundedText, CatalogNodeId, Engine, EngineType, OperationScope, Revision, RevisionRelation,
    SafeDiagnostic,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostgreSqlObjectKind {
    Table,
    View,
    MaterializedView,
    ForeignTable,
    PartitionedTable,
    Sequence,
    Function,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickHouseObjectKind {
    Table,
    View,
    MaterializedView,
    Dictionary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedisKeyKind {
    Unknown,
    String,
    Hash,
    List,
    Set,
    SortedSet,
    Stream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatalogNodeKind {
    PostgreSqlDatabase,
    PostgreSqlSchema,
    PostgreSqlObject(PostgreSqlObjectKind),
    PostgreSqlColumn,
    ClickHouseDatabase,
    ClickHouseObject(ClickHouseObjectKind),
    ClickHouseColumn,
    RedisLogicalDatabase,
    RedisNamespace,
    RedisKey(RedisKeyKind),
}

impl CatalogNodeKind {
    const fn engine(self) -> Engine {
        match self {
            Self::PostgreSqlDatabase
            | Self::PostgreSqlSchema
            | Self::PostgreSqlObject(_)
            | Self::PostgreSqlColumn => Engine::PostgreSql,
            Self::ClickHouseDatabase | Self::ClickHouseObject(_) | Self::ClickHouseColumn => {
                Engine::ClickHouse
            }
            Self::RedisLogicalDatabase | Self::RedisNamespace | Self::RedisKey(_) => Engine::Redis,
        }
    }

    const fn is_leaf(self) -> bool {
        matches!(
            self,
            Self::PostgreSqlColumn | Self::ClickHouseColumn | Self::RedisKey(_)
        )
    }

    const fn may_have_parent(self, parent: Self) -> bool {
        match self {
            Self::PostgreSqlDatabase | Self::ClickHouseDatabase | Self::RedisLogicalDatabase => {
                false
            }
            Self::PostgreSqlSchema => matches!(parent, Self::PostgreSqlDatabase),
            Self::PostgreSqlObject(_) => matches!(parent, Self::PostgreSqlSchema),
            Self::PostgreSqlColumn => matches!(parent, Self::PostgreSqlObject(_)),
            Self::ClickHouseObject(_) => matches!(parent, Self::ClickHouseDatabase),
            Self::ClickHouseColumn => matches!(parent, Self::ClickHouseObject(_)),
            Self::RedisNamespace => {
                matches!(parent, Self::RedisLogicalDatabase | Self::RedisNamespace)
            }
            Self::RedisKey(_) => {
                matches!(parent, Self::RedisLogicalDatabase | Self::RedisNamespace)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatalogChildrenState {
    NotApplicable,
    Unrequested,
    Loading,
    Loaded { complete: bool },
    Stale,
    Failed,
}

#[derive(Clone, PartialEq, Eq)]
pub struct CatalogNode {
    id: CatalogNodeId,
    parent_id: Option<CatalogNodeId>,
    depth: u16,
    kind: CatalogNodeKind,
    name: BoundedText,
    engine_type: Option<EngineType>,
    children: CatalogChildrenState,
    diagnostic: Option<SafeDiagnostic>,
}

impl CatalogNode {
    #[must_use]
    pub const fn new(
        id: CatalogNodeId,
        parent_id: Option<CatalogNodeId>,
        depth: u16,
        kind: CatalogNodeKind,
        name: BoundedText,
        engine_type: Option<EngineType>,
        children: CatalogChildrenState,
    ) -> Self {
        Self {
            id,
            parent_id,
            depth,
            kind,
            name,
            engine_type,
            children,
            diagnostic: None,
        }
    }

    #[must_use]
    pub fn with_diagnostic(mut self, diagnostic: SafeDiagnostic) -> Self {
        self.diagnostic = Some(diagnostic);
        self
    }

    #[must_use]
    pub const fn id(&self) -> CatalogNodeId {
        self.id
    }

    #[must_use]
    pub const fn parent_id(&self) -> Option<CatalogNodeId> {
        self.parent_id
    }

    #[must_use]
    pub const fn depth(&self) -> u16 {
        self.depth
    }

    #[must_use]
    pub const fn kind(&self) -> CatalogNodeKind {
        self.kind
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[must_use]
    pub const fn engine_type(&self) -> Option<&EngineType> {
        self.engine_type.as_ref()
    }

    #[must_use]
    pub const fn children(&self) -> CatalogChildrenState {
        self.children
    }

    #[must_use]
    pub const fn diagnostic(&self) -> Option<&SafeDiagnostic> {
        self.diagnostic.as_ref()
    }
}

impl fmt::Debug for CatalogNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CatalogNode")
            .field("id", &self.id)
            .field("parent_id", &self.parent_id)
            .field("depth", &self.depth)
            .field("kind", &self.kind)
            .field("name_bytes", &self.name.len())
            .field("has_engine_type", &self.engine_type.is_some())
            .field("children", &self.children)
            .field("has_diagnostic", &self.diagnostic.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CatalogIdentity {
    scope: OperationScope,
    engine: Engine,
    revision: Revision,
}

impl CatalogIdentity {
    #[must_use]
    pub const fn new(scope: OperationScope, engine: Engine, revision: Revision) -> Self {
        Self {
            scope,
            engine,
            revision,
        }
    }

    #[must_use]
    pub const fn scope(self) -> OperationScope {
        self.scope
    }

    #[must_use]
    pub const fn engine(self) -> Engine {
        self.engine
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CatalogLimits {
    max_nodes: u32,
    max_depth: u16,
    max_text_bytes: u64,
}

impl CatalogLimits {
    pub const fn new(
        max_nodes: u32,
        max_depth: u16,
        max_text_bytes: u64,
    ) -> Result<Self, CatalogBuildError> {
        if max_nodes == 0 || max_text_bytes == 0 {
            return Err(CatalogBuildError::InvalidLimits);
        }
        Ok(Self {
            max_nodes,
            max_depth,
            max_text_bytes,
        })
    }

    #[must_use]
    pub const fn max_nodes(self) -> u32 {
        self.max_nodes
    }

    #[must_use]
    pub const fn max_depth(self) -> u16 {
        self.max_depth
    }

    #[must_use]
    pub const fn max_text_bytes(self) -> u64 {
        self.max_text_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogBuildError {
    InvalidLimits,
    NodeLimitExceeded { actual: u64, limit: u32 },
    TextLimitExceeded { actual: u64, limit: u64 },
    EmptyName { node: u32 },
    DuplicateId { node: u32 },
    EngineMismatch { node: u32 },
    InvalidRoot { node: u32 },
    ParentNotBeforeChild { node: u32 },
    ParentOutsideActivePath { node: u32 },
    InvalidHierarchy { node: u32 },
    InvalidDepth { node: u32 },
    DepthLimitExceeded { node: u32, actual: u16, limit: u16 },
    InvalidLeafState { node: u32 },
    InvalidEngineType { node: u32 },
    InvalidDiagnostic { node: u32 },
}

impl fmt::Display for CatalogBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid catalog snapshot: {self:?}")
    }
}

impl Error for CatalogBuildError {}

#[derive(Clone, PartialEq, Eq)]
pub struct CatalogSnapshot {
    identity: CatalogIdentity,
    nodes: Vec<CatalogNode>,
    text_bytes: u64,
}

impl CatalogSnapshot {
    pub fn new(
        identity: CatalogIdentity,
        nodes: Vec<CatalogNode>,
        limits: CatalogLimits,
    ) -> Result<Self, CatalogBuildError> {
        let node_count = u64::try_from(nodes.len()).unwrap_or(u64::MAX);
        if node_count > u64::from(limits.max_nodes) {
            return Err(CatalogBuildError::NodeLimitExceeded {
                actual: node_count,
                limit: limits.max_nodes,
            });
        }
        let mut seen: BTreeMap<CatalogNodeId, (u16, CatalogNodeKind)> = BTreeMap::new();
        let mut active_path = Vec::new();
        let mut text_bytes = 0_u64;
        for (index, node) in nodes.iter().enumerate() {
            let node_index = u32::try_from(index).unwrap_or(u32::MAX);
            if node.name.is_empty() {
                return Err(CatalogBuildError::EmptyName { node: node_index });
            }
            if node.kind.engine() != identity.engine {
                return Err(CatalogBuildError::EngineMismatch { node: node_index });
            }
            if node.depth > limits.max_depth {
                return Err(CatalogBuildError::DepthLimitExceeded {
                    node: node_index,
                    actual: node.depth,
                    limit: limits.max_depth,
                });
            }
            if node.kind.is_leaf() != matches!(node.children, CatalogChildrenState::NotApplicable) {
                return Err(CatalogBuildError::InvalidLeafState { node: node_index });
            }
            if matches!(node.children, CatalogChildrenState::Failed) != node.diagnostic.is_some()
                || node
                    .diagnostic
                    .as_ref()
                    .is_some_and(|diagnostic| diagnostic.engine() != identity.engine)
            {
                return Err(CatalogBuildError::InvalidDiagnostic { node: node_index });
            }
            let type_bytes = if let Some(engine_type) = &node.engine_type {
                if !matches!(
                    node.kind,
                    CatalogNodeKind::PostgreSqlColumn | CatalogNodeKind::ClickHouseColumn
                ) || engine_type.engine() != identity.engine
                {
                    return Err(CatalogBuildError::InvalidEngineType { node: node_index });
                }
                u64::try_from(engine_type.name().len()).unwrap_or(u64::MAX)
            } else {
                0
            };
            text_bytes = text_bytes
                .checked_add(u64::try_from(node.name.len()).unwrap_or(u64::MAX))
                .and_then(|bytes| bytes.checked_add(type_bytes))
                .unwrap_or(u64::MAX);
            if text_bytes > limits.max_text_bytes {
                return Err(CatalogBuildError::TextLimitExceeded {
                    actual: text_bytes,
                    limit: limits.max_text_bytes,
                });
            }
            match node.parent_id {
                None => {
                    if node.depth != 0
                        || !matches!(
                            node.kind,
                            CatalogNodeKind::PostgreSqlDatabase
                                | CatalogNodeKind::ClickHouseDatabase
                                | CatalogNodeKind::RedisLogicalDatabase
                        )
                    {
                        return Err(CatalogBuildError::InvalidRoot { node: node_index });
                    }
                    active_path.clear();
                }
                Some(parent_id) => {
                    let Some((parent_depth, parent_kind)) = seen.get(&parent_id).copied() else {
                        return Err(CatalogBuildError::ParentNotBeforeChild { node: node_index });
                    };
                    if !node.kind.may_have_parent(parent_kind) {
                        return Err(CatalogBuildError::InvalidHierarchy { node: node_index });
                    }
                    if parent_depth.checked_add(1) != Some(node.depth) {
                        return Err(CatalogBuildError::InvalidDepth { node: node_index });
                    }
                    let parent_depth = usize::from(node.depth - 1);
                    if active_path.get(parent_depth) != Some(&parent_id) {
                        return Err(CatalogBuildError::ParentOutsideActivePath {
                            node: node_index,
                        });
                    }
                }
            }
            if seen.insert(node.id, (node.depth, node.kind)).is_some() {
                return Err(CatalogBuildError::DuplicateId { node: node_index });
            }
            active_path.truncate(usize::from(node.depth));
            active_path.push(node.id);
        }
        Ok(Self {
            identity,
            nodes,
            text_bytes,
        })
    }

    #[must_use]
    pub const fn identity(&self) -> CatalogIdentity {
        self.identity
    }

    #[must_use]
    pub fn nodes(&self) -> &[CatalogNode] {
        &self.nodes
    }

    #[must_use]
    pub const fn text_bytes(&self) -> u64 {
        self.text_bytes
    }
}

impl fmt::Debug for CatalogSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CatalogSnapshot")
            .field("identity", &self.identity)
            .field("nodes", &self.nodes.len())
            .field("text_bytes", &self.text_bytes)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogRejection {
    ForeignScope,
    EngineMismatch,
    StaleOrDuplicate,
    RevisionGap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CatalogCursor {
    scope: OperationScope,
    engine: Engine,
    revision: Revision,
}

impl CatalogCursor {
    #[must_use]
    pub const fn new(identity: CatalogIdentity) -> Self {
        Self {
            scope: identity.scope,
            engine: identity.engine,
            revision: identity.revision,
        }
    }

    pub fn accept(self, snapshot: &CatalogSnapshot) -> Result<Self, CatalogRejection> {
        let identity = snapshot.identity;
        if identity.scope != self.scope {
            return Err(CatalogRejection::ForeignScope);
        }
        if identity.engine != self.engine {
            return Err(CatalogRejection::EngineMismatch);
        }
        match identity.revision.relation_to(self.revision) {
            RevisionRelation::Stale | RevisionRelation::Current => {
                Err(CatalogRejection::StaleOrDuplicate)
            }
            RevisionRelation::Future
                if self.revision.checked_next().ok() == Some(identity.revision) =>
            {
                Ok(Self {
                    revision: identity.revision,
                    ..self
                })
            }
            RevisionRelation::Future => Err(CatalogRejection::RevisionGap),
        }
    }

    #[must_use]
    pub const fn revision(self) -> Revision {
        self.revision
    }
}
