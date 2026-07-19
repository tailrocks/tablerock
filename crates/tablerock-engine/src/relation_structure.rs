use std::{error::Error, fmt, sync::Arc};

use tablerock_core::{
    Engine, IdParts, PageIdentity, PageLimits, ResultId, Revision, StatementText,
};

use crate::{AdapterError, DriverPageRequest, DriverSession, FilterValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_expression: Option<String>,
    pub comment: Option<String>,
    pub primary_key: bool,
    pub sorting_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationFact {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseColumnFacts {
    pub name: String,
    pub data_type: String,
    pub default_kind: String,
    pub default_expression: String,
    pub comment: String,
    pub primary_key: bool,
    pub sorting_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseEngineFacts {
    pub engine: String,
    pub partition_key: String,
    pub sorting_key: String,
    pub primary_key: String,
    pub create_query: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationIndex {
    pub kind: String,
    pub name: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationConstraint {
    pub kind: String,
    pub name: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationStructureSnapshot {
    pub engine: Engine,
    pub namespace: String,
    pub relation: String,
    pub columns: Vec<RelationColumn>,
    pub indexes: Vec<RelationIndex>,
    pub constraints: Vec<RelationConstraint>,
    pub facts: Vec<RelationFact>,
}

#[derive(Debug)]
pub enum RelationStructureError {
    UnsupportedEngine,
    InvalidStatement,
    Adapter(AdapterError),
    Empty,
    ShapeMismatch,
}

impl fmt::Display for RelationStructureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEngine => formatter.write_str("relation structure is unavailable"),
            Self::InvalidStatement => formatter.write_str("structure statement is invalid"),
            Self::Adapter(error) => error.fmt(formatter),
            Self::Empty => formatter.write_str("relation has no visible columns"),
            Self::ShapeMismatch => formatter.write_str("structure query returned an invalid shape"),
        }
    }
}

impl Error for RelationStructureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Adapter(error) => Some(error),
            _ => None,
        }
    }
}

/// Loads a bounded typed structure snapshot below presentation.
pub async fn load_relation_structure(
    session: Arc<dyn DriverSession>,
    namespace: String,
    relation: String,
) -> Result<RelationStructureSnapshot, RelationStructureError> {
    if session.engine() == Engine::ClickHouse {
        return load_clickhouse_structure(session, namespace, relation).await;
    }
    if session.engine() != Engine::PostgreSql {
        return Err(RelationStructureError::UnsupportedEngine);
    }
    let columns = run_postgres_query(
        &session,
        9_101,
        "SELECT a.attname::text, pg_catalog.format_type(a.atttypid, a.atttypmod), \
         CASE WHEN a.attnotnull THEN 'false' ELSE 'true' END, \
         COALESCE(pg_catalog.pg_get_expr(d.adbin, d.adrelid), '') \
         FROM pg_catalog.pg_attribute a \
         JOIN pg_catalog.pg_class c ON c.oid = a.attrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
         WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped \
         ORDER BY a.attnum LIMIT 256",
        &namespace,
        &relation,
        256,
    )
    .await?;
    if columns.is_empty() {
        return Err(RelationStructureError::Empty);
    }
    let indexes = run_postgres_query(
        &session,
        9_102,
        "SELECT CASE WHEN ix.indisprimary THEN 'PRIMARY' WHEN ix.indisunique THEN 'UNIQUE' \
         ELSE 'INDEX' END, i.relname::text, pg_catalog.pg_get_indexdef(ix.indexrelid) \
         FROM pg_catalog.pg_index ix JOIN pg_catalog.pg_class t ON t.oid = ix.indrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
         JOIN pg_catalog.pg_class i ON i.oid = ix.indexrelid \
         WHERE n.nspname = $1 AND t.relname = $2 \
         ORDER BY ix.indisprimary DESC, i.relname LIMIT 128",
        &namespace,
        &relation,
        128,
    )
    .await?;
    let constraints = run_postgres_query(
        &session,
        9_103,
        "SELECT CASE con.contype WHEN 'p' THEN 'PRIMARY KEY' WHEN 'u' THEN 'UNIQUE' \
         WHEN 'c' THEN 'CHECK' WHEN 'x' THEN 'EXCLUDE' WHEN 'f' THEN 'FOREIGN KEY' \
         ELSE con.contype::text END, con.conname::text, \
         pg_catalog.pg_get_constraintdef(con.oid, true) \
         FROM pg_catalog.pg_constraint con JOIN pg_catalog.pg_class c ON c.oid = con.conrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = $1 AND c.relname = $2 AND con.contype IN ('p','u','c','x','f') \
         ORDER BY con.contype, con.conname LIMIT 128",
        &namespace,
        &relation,
        128,
    )
    .await?;
    let columns = columns
        .into_iter()
        .map(|row| {
            if row.len() != 4 {
                return Err(RelationStructureError::ShapeMismatch);
            }
            Ok(RelationColumn {
                name: row[0].clone(),
                data_type: row[1].clone(),
                nullable: row[2] == "true",
                default_expression: (!row[3].is_empty()).then(|| row[3].clone()),
                comment: None,
                primary_key: false,
                sorting_key: false,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let indexes = indexes
        .into_iter()
        .map(|row| {
            if row.len() != 3 {
                return Err(RelationStructureError::ShapeMismatch);
            }
            Ok(RelationIndex {
                kind: row[0].clone(),
                name: row[1].clone(),
                definition: row[2].clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let constraints = constraints
        .into_iter()
        .map(|row| {
            if row.len() != 3 {
                return Err(RelationStructureError::ShapeMismatch);
            }
            Ok(RelationConstraint {
                kind: row[0].clone(),
                name: row[1].clone(),
                definition: row[2].clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RelationStructureSnapshot {
        engine: Engine::PostgreSql,
        namespace,
        relation,
        columns,
        indexes,
        constraints,
        facts: Vec::new(),
    })
}

async fn load_clickhouse_structure(
    session: Arc<dyn DriverSession>,
    database: String,
    table: String,
) -> Result<RelationStructureSnapshot, RelationStructureError> {
    let columns = session
        .clickhouse_relation_columns(&database, &table)
        .await
        .map_err(RelationStructureError::Adapter)?;
    if columns.is_empty() {
        return Err(RelationStructureError::Empty);
    }
    let engine = session
        .clickhouse_relation_engine(&database, &table)
        .await
        .map_err(RelationStructureError::Adapter)?
        .ok_or(RelationStructureError::Empty)?;
    Ok(RelationStructureSnapshot {
        engine: Engine::ClickHouse,
        namespace: database,
        relation: table,
        columns: columns
            .into_iter()
            .map(|column| RelationColumn {
                nullable: column.data_type.starts_with("Nullable("),
                default_expression: (!column.default_expression.is_empty())
                    .then_some(column.default_expression),
                comment: (!column.comment.is_empty()).then_some(column.comment),
                name: column.name,
                data_type: column.data_type,
                primary_key: column.primary_key,
                sorting_key: column.sorting_key,
            })
            .collect(),
        indexes: Vec::new(),
        constraints: Vec::new(),
        facts: vec![
            RelationFact {
                name: "Engine".into(),
                value: engine.engine,
            },
            RelationFact {
                name: "Partition key".into(),
                value: engine.partition_key,
            },
            RelationFact {
                name: "Sorting key".into(),
                value: engine.sorting_key,
            },
            RelationFact {
                name: "Primary key".into(),
                value: engine.primary_key,
            },
            RelationFact {
                name: "Create query".into(),
                value: engine.create_query,
            },
        ],
    })
}

async fn run_postgres_query(
    session: &Arc<dyn DriverSession>,
    result_low: u64,
    sql: &str,
    namespace: &str,
    relation: &str,
    max_rows: u32,
) -> Result<Vec<Vec<String>>, RelationStructureError> {
    let statement =
        StatementText::new(sql).map_err(|_| RelationStructureError::InvalidStatement)?;
    let mut stream = session
        .start_page_stream(DriverPageRequest::PostgreSqlStatement {
            statement,
            parameters: vec![
                FilterValue::Text(namespace.to_owned()),
                FilterValue::Text(relation.to_owned()),
            ],
            limits: PageLimits::new(max_rows, 8, 256 * 1024, 8 * 1024),
            max_cell_bytes: 8 * 1024,
        })
        .await
        .map_err(RelationStructureError::Adapter)?;
    let identity = PageIdentity::new(
        ResultId::from_parts(IdParts::new(1, result_low).expect("nonzero id parts"))
            .expect("nonzero result id"),
        Revision::INITIAL,
        Engine::PostgreSql,
    );
    let Some(page) = stream
        .next_page(identity, 0)
        .await
        .map_err(RelationStructureError::Adapter)?
    else {
        return Ok(Vec::new());
    };
    Ok((0..page.envelope().row_count())
        .map(|row| {
            (0..page.envelope().column_count())
                .map(|column| {
                    page.cell(row, column)
                        .map(|cell| String::from_utf8_lossy(cell.bytes()).into_owned())
                        .unwrap_or_default()
                })
                .collect()
        })
        .collect())
}
