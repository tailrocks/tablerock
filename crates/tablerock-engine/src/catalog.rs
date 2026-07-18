//! Engine catalog listing requests and bounded subtrees.

use std::fmt;

use tablerock_core::{
    BoundedText, CatalogChildrenState, CatalogNodeKind, Engine, EngineType, PageLimits,
};

/// How complete the returned catalog level is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatalogExactness {
    /// Server returned a complete level under the applied limit.
    Exact,
    /// Limit stopped enumeration; more rows may exist.
    Truncated,
    /// Value assumed because the server denied authoritative config.
    DefaultAssumed,
}

/// One catalog child before stable IDs are assigned by the service.
#[derive(Clone, PartialEq, Eq)]
pub struct CatalogNodeSeed {
    kind: CatalogNodeKind,
    name: BoundedText,
    children: CatalogChildrenState,
    engine_type: Option<EngineType>,
}

impl CatalogNodeSeed {
    #[must_use]
    pub const fn new(
        kind: CatalogNodeKind,
        name: BoundedText,
        children: CatalogChildrenState,
        engine_type: Option<EngineType>,
    ) -> Self {
        Self {
            kind,
            name,
            children,
            engine_type,
        }
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
    pub fn into_name(self) -> BoundedText {
        self.name
    }

    #[must_use]
    pub const fn children(&self) -> CatalogChildrenState {
        self.children
    }

    #[must_use]
    pub const fn engine_type(&self) -> Option<&EngineType> {
        self.engine_type.as_ref()
    }

    #[must_use]
    pub fn take_engine_type(self) -> Option<EngineType> {
        self.engine_type
    }
}

impl fmt::Debug for CatalogNodeSeed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CatalogNodeSeed")
            .field("kind", &self.kind)
            .field("name_bytes", &self.name.len())
            .field("children", &self.children)
            .field("has_engine_type", &self.engine_type.is_some())
            .finish()
    }
}

/// Bounded owned list of catalog children for one refresh level.
#[derive(Clone, PartialEq, Eq)]
pub struct CatalogSubtree {
    engine: Engine,
    nodes: Vec<CatalogNodeSeed>,
    complete: bool,
    exactness: CatalogExactness,
}

impl CatalogSubtree {
    #[must_use]
    pub fn new(
        engine: Engine,
        nodes: Vec<CatalogNodeSeed>,
        complete: bool,
        exactness: CatalogExactness,
    ) -> Self {
        Self {
            engine,
            nodes,
            complete,
            exactness,
        }
    }

    #[must_use]
    pub const fn engine(&self) -> Engine {
        self.engine
    }

    #[must_use]
    pub fn nodes(&self) -> &[CatalogNodeSeed] {
        &self.nodes
    }

    #[must_use]
    pub fn into_nodes(self) -> Vec<CatalogNodeSeed> {
        self.nodes
    }

    #[must_use]
    pub const fn complete(&self) -> bool {
        self.complete
    }

    #[must_use]
    pub const fn exactness(&self) -> CatalogExactness {
        self.exactness
    }
}

impl fmt::Debug for CatalogSubtree {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CatalogSubtree")
            .field("engine", &self.engine)
            .field("nodes", &self.nodes.len())
            .field("complete", &self.complete)
            .field("exactness", &self.exactness)
            .finish()
    }
}

/// Catalog listing intents. Names are never Debug-printed.
#[derive(Clone, PartialEq, Eq)]
pub enum CatalogRequest {
    PostgreSqlDatabases {
        limits: PageLimits,
    },
    PostgreSqlSchemas {
        database: BoundedText,
        limits: PageLimits,
    },
    PostgreSqlRelations {
        database: BoundedText,
        schema: BoundedText,
        limits: PageLimits,
    },
    ClickHouseDatabases {
        limits: PageLimits,
    },
    ClickHouseObjects {
        database: BoundedText,
        limits: PageLimits,
    },
    RedisLogicalDatabases {
        limits: PageLimits,
    },
}

impl CatalogRequest {
    #[must_use]
    pub const fn engine(&self) -> Engine {
        match self {
            Self::PostgreSqlDatabases { .. }
            | Self::PostgreSqlSchemas { .. }
            | Self::PostgreSqlRelations { .. } => Engine::PostgreSql,
            Self::ClickHouseDatabases { .. } | Self::ClickHouseObjects { .. } => Engine::ClickHouse,
            Self::RedisLogicalDatabases { .. } => Engine::Redis,
        }
    }

    #[must_use]
    pub const fn limits(&self) -> PageLimits {
        match self {
            Self::PostgreSqlDatabases { limits }
            | Self::PostgreSqlSchemas { limits, .. }
            | Self::PostgreSqlRelations { limits, .. }
            | Self::ClickHouseDatabases { limits }
            | Self::ClickHouseObjects { limits, .. }
            | Self::RedisLogicalDatabases { limits } => *limits,
        }
    }
}

impl fmt::Debug for CatalogRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("CatalogRequest");
        debug.field("engine", &self.engine());
        match self {
            Self::PostgreSqlDatabases { limits } => debug
                .field("level", &"databases")
                .field("max_rows", &limits.max_rows()),
            Self::PostgreSqlSchemas { database, limits } => debug
                .field("level", &"schemas")
                .field("database_bytes", &database.len())
                .field("max_rows", &limits.max_rows()),
            Self::PostgreSqlRelations {
                database,
                schema,
                limits,
            } => debug
                .field("level", &"relations")
                .field("database_bytes", &database.len())
                .field("schema_bytes", &schema.len())
                .field("max_rows", &limits.max_rows()),
            Self::ClickHouseDatabases { limits } => debug
                .field("level", &"databases")
                .field("max_rows", &limits.max_rows()),
            Self::ClickHouseObjects { database, limits } => debug
                .field("level", &"objects")
                .field("database_bytes", &database.len())
                .field("max_rows", &limits.max_rows()),
            Self::RedisLogicalDatabases { limits } => debug
                .field("level", &"logical_databases")
                .field("max_rows", &limits.max_rows()),
        };
        debug.finish()
    }
}

/// Default logical DB count when Redis CONFIG is denied.
pub const REDIS_DEFAULT_LOGICAL_DATABASES: u32 = 16;

const CATALOG_NAME_BYTE_LIMIT: u64 = 256;

#[must_use]
pub(crate) fn catalog_seed(
    kind: CatalogNodeKind,
    name: &str,
    children: CatalogChildrenState,
    engine_type: Option<EngineType>,
) -> Option<CatalogNodeSeed> {
    let name = BoundedText::copy_from_str(
        name,
        tablerock_core::ByteLimit::new(CATALOG_NAME_BYTE_LIMIT),
    )
    .ok()?;
    if name.is_empty() {
        return None;
    }
    Some(CatalogNodeSeed::new(kind, name, children, engine_type))
}

pub(crate) fn catalog_name_list(
    engine: Engine,
    names: impl IntoIterator<Item = String>,
    kind: CatalogNodeKind,
    children: CatalogChildrenState,
    limit: u32,
) -> CatalogSubtree {
    let mut nodes = Vec::new();
    let mut truncated = false;
    for name in names {
        if nodes.len() as u32 >= limit {
            truncated = true;
            break;
        }
        if let Some(seed) = catalog_seed(kind, &name, children, None) {
            nodes.push(seed);
        }
    }
    CatalogSubtree::new(
        engine,
        nodes,
        !truncated,
        if truncated {
            CatalogExactness::Truncated
        } else {
            CatalogExactness::Exact
        },
    )
}
