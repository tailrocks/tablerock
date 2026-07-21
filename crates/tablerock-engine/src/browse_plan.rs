//! Typed browse-plan builder — the only place UI state becomes SQL.
//!
//! Identifiers go through [`quote_ident`]. Filter values are NEVER concatenated;
//! they become `$n` placeholders with a parallel typed parameter list.
//! Raw WHERE fragments are parenthesized and AND-composed; embedded `$n` tokens
//! that collide with plan parameters are rejected (fail closed).

use crate::ident::{QuoteIdentError, qualify_table, quote_ident};

/// Sort direction for one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// One sort key: column identifier + direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortKey {
    pub column: String,
    pub direction: SortDirection,
}

/// Operators allowed in typed filter conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Like,
    ILike,
    /// `NOT LIKE` (pattern still bound as a parameter).
    NotLike,
    /// `NOT ILIKE` (pattern still bound as a parameter).
    NotILike,
    IsNull,
    IsNotNull,
}

impl FilterOperator {
    #[must_use]
    pub const fn needs_value(self) -> bool {
        !matches!(self, Self::IsNull | Self::IsNotNull)
    }

    fn sql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Like => "LIKE",
            Self::ILike => "ILIKE",
            Self::NotLike => "NOT LIKE",
            Self::NotILike => "NOT ILIKE",
            Self::IsNull => "IS NULL",
            Self::IsNotNull => "IS NOT NULL",
        }
    }
}

/// Typed filter value (parameters only; never inlined into SQL text).
#[derive(Clone, PartialEq)]
pub enum FilterValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    /// SQL NULL parameter (typed prepare still required by the engine).
    Null,
}

impl std::fmt::Debug for FilterValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(_) => f.write_str("Text([redacted])"),
            Self::Integer(_) => f.write_str("Integer([redacted])"),
            Self::Float(_) => f.write_str("Float([redacted])"),
            Self::Boolean(b) => f.debug_tuple("Boolean").field(b).finish(),
            Self::Null => f.write_str("Null"),
        }
    }
}

/// Parse a presentation string into a bind value (never for SQL concatenation).
#[must_use]
pub fn parse_bind_text(raw: &str) -> FilterValue {
    let t = raw.trim();
    if t.eq_ignore_ascii_case("null") {
        return FilterValue::Null;
    }
    if t.eq_ignore_ascii_case("true") {
        return FilterValue::Boolean(true);
    }
    if t.eq_ignore_ascii_case("false") {
        return FilterValue::Boolean(false);
    }
    if let Ok(n) = t.parse::<i64>() {
        return FilterValue::Integer(n);
    }
    if let Ok(n) = t.parse::<f64>()
        && (t.contains('.') || t.contains('e') || t.contains('E'))
    {
        return FilterValue::Float(n);
    }
    FilterValue::Text(t.to_owned())
}

/// One typed column condition.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedCondition {
    pub column: String,
    pub operator: FilterOperator,
    pub value: Option<FilterValue>,
}

/// Owned browse plan for a base table.
#[derive(Clone, PartialEq)]
pub struct BrowsePlan {
    pub schema: String,
    pub table: String,
    pub sort: Vec<SortKey>,
    pub filters: Vec<TypedCondition>,
    /// Optional raw WHERE fragment (user text). Parenthesized when composed.
    pub raw_where: Option<String>,
    pub limit: u32,
    pub offset: u64,
}

impl std::fmt::Debug for BrowsePlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowsePlan")
            .field("schema", &self.schema)
            .field("table", &self.table)
            .field("sort", &self.sort)
            .field("filters", &self.filters)
            .field(
                "raw_where",
                &self.raw_where.as_ref().map(|_| "[redacted fragment]"),
            )
            .field("limit", &self.limit)
            .field("offset", &self.offset)
            .finish()
    }
}

/// Rendered SQL + positional parameters (1-based `$n` order).
#[derive(Clone, PartialEq)]
pub struct RenderedBrowseSql {
    pub sql: String,
    pub parameters: Vec<FilterValue>,
}

impl std::fmt::Debug for RenderedBrowseSql {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderedBrowseSql")
            .field("sql", &self.sql)
            .field("parameters", &self.parameters)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowsePlanError {
    Ident(QuoteIdentError),
    MissingValue,
    UnexpectedValue,
    EmptyRawWhere,
    RawWhereParameterCollision,
    InvalidLimit,
}

impl std::fmt::Display for BrowsePlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ident(_) => "invalid identifier in browse plan",
            Self::MissingValue => "filter operator requires a value",
            Self::UnexpectedValue => "filter operator does not take a value",
            Self::EmptyRawWhere => "raw WHERE fragment is empty",
            Self::RawWhereParameterCollision => {
                "raw WHERE must not contain $n placeholders (fail closed)"
            }
            Self::InvalidLimit => "limit must be at least 1",
        })
    }
}

impl std::error::Error for BrowsePlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ident(e) => Some(e),
            _ => None,
        }
    }
}

impl From<QuoteIdentError> for BrowsePlanError {
    fn from(value: QuoteIdentError) -> Self {
        Self::Ident(value)
    }
}

impl BrowsePlan {
    /// Render `SELECT * FROM schema.table [WHERE …] [ORDER BY …] LIMIT/OFFSET`.
    pub fn render_sql(&self) -> Result<RenderedBrowseSql, BrowsePlanError> {
        if self.limit == 0 {
            return Err(BrowsePlanError::InvalidLimit);
        }
        let qualified = qualify_table(&self.schema, &self.table)?;
        let mut sql = format!("SELECT * FROM {qualified}");
        let mut parameters = Vec::new();
        let mut where_parts: Vec<String> = Vec::new();

        for filter in &self.filters {
            let col = quote_ident(&filter.column)?;
            if filter.operator.needs_value() {
                let Some(value) = filter.value.clone() else {
                    return Err(BrowsePlanError::MissingValue);
                };
                parameters.push(value);
                let n = parameters.len();
                where_parts.push(format!("{col} {} ${n}", filter.operator.sql()));
            } else {
                if filter.value.is_some() {
                    return Err(BrowsePlanError::UnexpectedValue);
                }
                where_parts.push(format!("{col} {}", filter.operator.sql()));
            }
        }

        if let Some(raw) = &self.raw_where {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Err(BrowsePlanError::EmptyRawWhere);
            }
            // Fail closed on any $n-like token so we never renumber ambiguously.
            if contains_dollar_param(trimmed) {
                return Err(BrowsePlanError::RawWhereParameterCollision);
            }
            where_parts.push(format!("({trimmed})"));
        }

        if !where_parts.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_parts.join(" AND "));
        }

        if !self.sort.is_empty() {
            sql.push_str(" ORDER BY ");
            let mut keys = Vec::with_capacity(self.sort.len());
            for key in &self.sort {
                let col = quote_ident(&key.column)?;
                let dir = match key.direction {
                    SortDirection::Asc => "ASC",
                    SortDirection::Desc => "DESC",
                };
                keys.push(format!("{col} {dir}"));
            }
            sql.push_str(&keys.join(", "));
        }

        // LIMIT/OFFSET are plan integers, not user strings.
        sql.push_str(&format!(" LIMIT {} OFFSET {}", self.limit, self.offset));
        Ok(RenderedBrowseSql { sql, parameters })
    }
}

fn contains_dollar_param(fragment: &str) -> bool {
    let bytes = fragment.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let mut j = i + 1;
            if j < bytes.len() && bytes[j].is_ascii_digit() {
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                // $1, $12, … — reject. Bare `$` alone is allowed.
                if j > i + 1 {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> BrowsePlan {
        BrowsePlan {
            schema: "public".into(),
            table: "users".into(),
            sort: Vec::new(),
            filters: Vec::new(),
            raw_where: None,
            limit: 500,
            offset: 0,
        }
    }

    #[test]
    fn simple_browse_quotes_identifiers() {
        let plan = base();
        let rendered = plan.render_sql().unwrap();
        assert_eq!(
            rendered.sql,
            "SELECT * FROM \"public\".\"users\" LIMIT 500 OFFSET 0"
        );
        assert!(rendered.parameters.is_empty());
    }

    #[test]
    fn hostile_identifier_is_quoted_not_injected() {
        let mut plan = base();
        plan.table = "\"; DROP TABLE x; --".into();
        let rendered = plan.render_sql().unwrap();
        // quote_ident doubles internal quotes: `"\""; DROP TABLE x; --"`
        let expected_table = quote_ident("\"; DROP TABLE x; --").unwrap();
        assert!(
            rendered
                .sql
                .contains(&format!("\"public\".{expected_table}")),
            "sql={}",
            rendered.sql
        );
        // Only one FROM clause — not a second statement after injection.
        assert_eq!(rendered.sql.matches("FROM ").count(), 1);
        assert!(rendered.sql.starts_with("SELECT * FROM "));
        assert!(rendered.sql.ends_with("LIMIT 500 OFFSET 0"));
    }

    #[test]
    fn sort_keys_use_quoted_columns() {
        let mut plan = base();
        plan.sort = vec![
            SortKey {
                column: "name".into(),
                direction: SortDirection::Asc,
            },
            SortKey {
                column: "id".into(),
                direction: SortDirection::Desc,
            },
        ];
        let sql = plan.render_sql().unwrap().sql;
        assert!(sql.contains("ORDER BY \"name\" ASC, \"id\" DESC"));
    }

    #[test]
    fn typed_filters_parameterize_values() {
        let mut plan = base();
        plan.filters = vec![
            TypedCondition {
                column: "age".into(),
                operator: FilterOperator::Ge,
                value: Some(FilterValue::Integer(21)),
            },
            TypedCondition {
                column: "name".into(),
                operator: FilterOperator::Like,
                value: Some(FilterValue::Text("%a%".into())),
            },
        ];
        let rendered = plan.render_sql().unwrap();
        assert!(rendered.sql.contains("\"age\" >= $1"));
        assert!(rendered.sql.contains("\"name\" LIKE $2"));
        assert_eq!(rendered.parameters.len(), 2);
        // Values never appear inline in SQL.
        assert!(!rendered.sql.contains("21"));
        assert!(!rendered.sql.contains("%a%"));
    }

    #[test]
    fn not_like_operators_parameterize_patterns() {
        let mut plan = base();
        plan.filters = vec![
            TypedCondition {
                column: "name".into(),
                operator: FilterOperator::NotLike,
                value: Some(FilterValue::Text("%spam%".into())),
            },
            TypedCondition {
                column: "email".into(),
                operator: FilterOperator::NotILike,
                value: Some(FilterValue::Text("%test%".into())),
            },
        ];
        let rendered = plan.render_sql().unwrap();
        assert!(
            rendered.sql.contains("\"name\" NOT LIKE $1"),
            "{}",
            rendered.sql
        );
        assert!(
            rendered.sql.contains("\"email\" NOT ILIKE $2"),
            "{}",
            rendered.sql
        );
        assert_eq!(rendered.parameters.len(), 2);
        assert!(!rendered.sql.contains("%spam%"));
        assert!(!rendered.sql.contains("%test%"));
    }

    #[test]
    fn null_operators_have_no_value_parameter() {
        let mut plan = base();
        plan.filters = vec![TypedCondition {
            column: "email".into(),
            operator: FilterOperator::IsNull,
            value: None,
        }];
        let rendered = plan.render_sql().unwrap();
        assert!(rendered.sql.contains("\"email\" IS NULL"));
        assert!(rendered.parameters.is_empty());
    }

    #[test]
    fn is_null_with_value_rejected() {
        let mut plan = base();
        plan.filters = vec![TypedCondition {
            column: "email".into(),
            operator: FilterOperator::IsNull,
            value: Some(FilterValue::Text("x".into())),
        }];
        assert_eq!(plan.render_sql(), Err(BrowsePlanError::UnexpectedValue));
    }

    #[test]
    fn eq_without_value_rejected() {
        let mut plan = base();
        plan.filters = vec![TypedCondition {
            column: "id".into(),
            operator: FilterOperator::Eq,
            value: None,
        }];
        assert_eq!(plan.render_sql(), Err(BrowsePlanError::MissingValue));
    }

    #[test]
    fn raw_where_parenthesized_and_and_composed() {
        let mut plan = base();
        plan.filters = vec![TypedCondition {
            column: "active".into(),
            operator: FilterOperator::Eq,
            value: Some(FilterValue::Boolean(true)),
        }];
        plan.raw_where = Some("status = 'open'".into());
        let rendered = plan.render_sql().unwrap();
        assert!(
            rendered
                .sql
                .contains("WHERE \"active\" = $1 AND (status = 'open')")
        );
    }

    #[test]
    fn raw_where_with_dollar_param_rejected() {
        let mut plan = base();
        plan.raw_where = Some("id = $1".into());
        assert_eq!(
            plan.render_sql(),
            Err(BrowsePlanError::RawWhereParameterCollision)
        );
    }

    #[test]
    fn empty_raw_where_rejected() {
        let mut plan = base();
        plan.raw_where = Some("   ".into());
        assert_eq!(plan.render_sql(), Err(BrowsePlanError::EmptyRawWhere));
    }

    #[test]
    fn debug_redacts_filter_values_and_raw_where() {
        let mut plan = base();
        plan.filters = vec![TypedCondition {
            column: "secret".into(),
            operator: FilterOperator::Eq,
            value: Some(FilterValue::Text("hunter2".into())),
        }];
        plan.raw_where = Some("password = 'x'".into());
        let dbg = format!("{plan:?}");
        assert!(!dbg.contains("hunter2"));
        assert!(!dbg.contains("password = 'x'"));
        assert!(dbg.contains("redacted"));
    }

    #[test]
    fn empty_ident_rejected() {
        let mut plan = base();
        plan.schema = String::new();
        assert!(matches!(
            plan.render_sql(),
            Err(BrowsePlanError::Ident(QuoteIdentError::Empty))
        ));
    }

    #[test]
    fn parse_bind_text_heuristics() {
        assert!(matches!(parse_bind_text("null"), FilterValue::Null));
        assert!(matches!(
            parse_bind_text("TRUE"),
            FilterValue::Boolean(true)
        ));
        assert!(matches!(parse_bind_text("42"), FilterValue::Integer(42)));
        assert!(matches!(parse_bind_text("1.5"), FilterValue::Float(_)));
        assert!(matches!(
            parse_bind_text("hello"),
            FilterValue::Text(s) if s == "hello"
        ));
    }
}
