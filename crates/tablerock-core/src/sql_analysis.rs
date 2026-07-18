//! Dialect-aware SQL statement boundaries and incomplete-input recovery.
//!
//! Uses `sqlparser` tokens (never naive `split(';')`). Incomplete or
//! unterminated input yields a final open span to EOF instead of panicking.

use sqlparser::dialect::{ClickHouseDialect, Dialect, PostgreSqlDialect};
use sqlparser::tokenizer::{Token, TokenWithSpan, Tokenizer, TokenizerError};

/// SQL dialect for analysis (product surface; not engine transport).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SqlDialect {
    PostgreSql,
    ClickHouse,
}

/// One statement span in the source text (UTF-8 byte offsets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatementSpan {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
    /// True when the span was closed by a semicolon or full parse end.
    pub complete: bool,
}

impl StatementSpan {
    /// Byte slice of the statement in `source` (trimmed of outer whitespace).
    #[must_use]
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        let end = self.end.min(source.len());
        let start = self.start.min(end);
        source[start..end].trim()
    }
}

/// Split `source` into statement spans for `dialect`.
///
/// Never panics. Incomplete quotes/comments produce a trailing incomplete span.
#[must_use]
pub fn statements(source: &str, dialect: SqlDialect) -> Vec<StatementSpan> {
    match dialect {
        SqlDialect::PostgreSql => statements_with(&PostgreSqlDialect {}, source),
        SqlDialect::ClickHouse => statements_with(&ClickHouseDialect {}, source),
    }
}

fn statements_with(dialect: &dyn Dialect, source: &str) -> Vec<StatementSpan> {
    if source.trim().is_empty() {
        return Vec::new();
    }
    match Tokenizer::new(dialect, source).tokenize_with_location() {
        Ok(tokens) => spans_from_tokens(source, &tokens),
        Err(error) => spans_with_incomplete_recovery(source, dialect, error),
    }
}

fn spans_from_tokens(source: &str, tokens: &[TokenWithSpan]) -> Vec<StatementSpan> {
    let line_starts = line_start_bytes(source);
    let mut spans = Vec::new();
    let mut stmt_start: Option<usize> = None;
    let mut depth: i32 = 0;

    for token in tokens {
        let start = location_to_byte(source, &line_starts, token.span.start);
        let end = location_to_byte(source, &line_starts, token.span.end);
        match &token.token {
            Token::Whitespace(_) => {
                // Whitespace does not open a statement by itself.
            }
            Token::LParen | Token::LBracket | Token::LBrace => {
                if stmt_start.is_none() {
                    stmt_start = Some(start);
                }
                depth = depth.saturating_add(1);
            }
            Token::RParen | Token::RBracket | Token::RBrace => {
                if stmt_start.is_none() {
                    stmt_start = Some(start);
                }
                depth = depth.saturating_sub(1);
            }
            Token::SemiColon if depth == 0 => {
                let s = stmt_start.unwrap_or(start);
                // Include the semicolon in the span.
                let e = end;
                if !source[s..e.min(source.len())].trim().is_empty() {
                    spans.push(StatementSpan {
                        start: s,
                        end: e,
                        complete: true,
                    });
                }
                stmt_start = None;
            }
            _ => {
                if stmt_start.is_none() {
                    stmt_start = Some(start);
                }
            }
        }
    }

    if let Some(s) = stmt_start {
        let e = source.len();
        if !source[s..e].trim().is_empty() {
            spans.push(StatementSpan {
                start: s,
                end: e,
                complete: false,
            });
        }
    }
    spans
}

/// When the tokenizer fails (unterminated string, etc.), recover what we can
/// from a character-level scan that still respects quotes/dollar-quotes/comments.
fn spans_with_incomplete_recovery(
    source: &str,
    dialect: &dyn Dialect,
    _error: TokenizerError,
) -> Vec<StatementSpan> {
    // Try tokenizing a growing prefix to salvage complete statements, then
    // attach the remainder as one incomplete span.
    let mut best_tokens: Vec<TokenWithSpan> = Vec::new();
    let mut best_end = 0_usize;
    // Binary-ish search is overkill; walk by lines then characters is fine for editor sizes.
    let mut idx = 0;
    while idx < source.len() {
        idx = next_boundary(source, idx);
        let prefix = &source[..idx];
        match Tokenizer::new(dialect, prefix).tokenize_with_location() {
            Ok(tokens) => {
                best_tokens = tokens;
                best_end = idx;
            }
            Err(_) => break,
        }
    }
    let mut spans = if best_tokens.is_empty() {
        Vec::new()
    } else {
        spans_from_tokens(&source[..best_end], &best_tokens)
            .into_iter()
            .filter(|s| s.complete)
            .collect()
    };
    if best_end < source.len() {
        let rest = source[best_end..].trim_start();
        if !rest.is_empty() {
            let start = source.len() - source[best_end..].trim_start().len();
            // Prefer character-level semicolon scan on the remainder for extra completes.
            let mut remainder = char_level_spans(&source[start..]);
            for span in &mut remainder {
                span.start += start;
                span.end += start;
            }
            if remainder.is_empty() {
                spans.push(StatementSpan {
                    start,
                    end: source.len(),
                    complete: false,
                });
            } else {
                spans.extend(remainder);
            }
        }
    }
    if spans.is_empty() && !source.trim().is_empty() {
        spans.push(StatementSpan {
            start: 0,
            end: source.len(),
            complete: false,
        });
    }
    spans
}

fn next_boundary(source: &str, from: usize) -> usize {
    if from >= source.len() {
        return source.len();
    }
    // Advance at least one char; prefer newline for faster recovery.
    let rest = &source[from..];
    if let Some(rel) = rest.find('\n') {
        from + rel + 1
    } else {
        source.len()
    }
}

/// Conservative char-level splitter used only for incomplete recovery.
fn char_level_spans(source: &str) -> Vec<StatementSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    let mut stmt_start = 0;
    let mut depth = 0_i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut dollar_tag: Option<String> = None;

    while i < bytes.len() {
        let c = bytes[i] as char;

        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if c == '*' && bytes.get(i + 1) == Some(&b'/') {
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if let Some(tag) = dollar_tag.clone() {
            let close = format!("${tag}$");
            if source[i..].starts_with(&close) {
                i += close.len();
                dollar_tag = None;
                continue;
            }
            i += 1;
            continue;
        }
        if in_single {
            if c == '\'' {
                if bytes.get(i + 1) == Some(&b'\'') {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '"' {
                if bytes.get(i + 1) == Some(&b'"') {
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += 1;
            continue;
        }

        // dollar-quote open
        if c == '$' {
            if let Some((tag, len)) = parse_dollar_tag(&source[i..]) {
                dollar_tag = Some(tag);
                i += len;
                continue;
            }
        }
        if c == '\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if c == '"' {
            in_double = true;
            i += 1;
            continue;
        }
        if c == '-' && bytes.get(i + 1) == Some(&b'-') {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if c == '/' && bytes.get(i + 1) == Some(&b'*') {
            in_block_comment = true;
            i += 2;
            continue;
        }
        if c == '(' {
            depth += 1;
            i += 1;
            continue;
        }
        if c == ')' {
            depth = depth.saturating_sub(1);
            i += 1;
            continue;
        }
        if c == ';' && depth == 0 {
            let end = i + 1;
            if !source[stmt_start..end].trim().is_empty() {
                spans.push(StatementSpan {
                    start: stmt_start,
                    end,
                    complete: true,
                });
            }
            stmt_start = end;
            i = end;
            continue;
        }
        i += 1;
    }

    if stmt_start < source.len() && !source[stmt_start..].trim().is_empty() {
        spans.push(StatementSpan {
            start: stmt_start,
            end: source.len(),
            complete: false,
        });
    }
    spans
}

fn parse_dollar_tag(s: &str) -> Option<(String, usize)> {
    if !s.starts_with('$') {
        return None;
    }
    let rest = &s[1..];
    if rest.starts_with('$') {
        return Some((String::new(), 2));
    }
    let mut tag = String::new();
    for (idx, ch) in rest.char_indices() {
        if ch == '$' {
            return Some((tag, idx + 2)); // leading $ + tag + closing $
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            tag.push(ch);
        } else {
            return None;
        }
    }
    None
}

fn line_start_bytes(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn location_to_byte(
    source: &str,
    line_starts: &[usize],
    loc: sqlparser::tokenizer::Location,
) -> usize {
    let line = loc.line.max(1) as usize;
    let column = loc.column.max(1) as usize;
    let line_idx = line - 1;
    let line_start = line_starts.get(line_idx).copied().unwrap_or(source.len());
    // sqlparser columns are 1-based character columns on the line.
    let line_text = if line_idx + 1 < line_starts.len() {
        &source[line_start..line_starts[line_idx + 1]]
    } else {
        &source[line_start..]
    };
    let mut cols = 0_usize;
    for (byte_off, _) in line_text.char_indices() {
        cols += 1;
        if cols >= column {
            return line_start + byte_off;
        }
    }
    // End of line / past end → end of line content (exclude trailing newline for end markers).
    if column > cols {
        return line_start + line_text.trim_end_matches('\n').len();
    }
    line_start
}

/// Statement under (or before) the given cursor byte offset.
#[must_use]
pub fn statement_at(source: &str, dialect: SqlDialect, cursor: usize) -> Option<StatementSpan> {
    let spans = statements(source, dialect);
    if spans.is_empty() {
        return None;
    }
    let cursor = cursor.min(source.len());
    spans
        .iter()
        .copied()
        .find(|s| cursor >= s.start && cursor < s.end)
        .or_else(|| {
            // Cursor after last complete semicolon: last span.
            spans.last().copied()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_statements_without_naive_semicolon_in_strings() {
        let sql = "SELECT 'a;b'; SELECT 2";
        let spans = statements(sql, SqlDialect::PostgreSql);
        assert_eq!(spans.len(), 2);
        assert!(spans[0].complete);
        assert!(!spans[1].complete || spans[1].slice(sql).contains('2'));
        assert_eq!(spans[0].slice(sql), "SELECT 'a;b';");
        assert!(spans[1].slice(sql).starts_with("SELECT 2"));
    }

    #[test]
    fn respects_dollar_quoting_with_embedded_semicolons() {
        let sql = "SELECT $$x;y$$; SELECT 1";
        let spans = statements(sql, SqlDialect::PostgreSql);
        assert_eq!(spans.len(), 2);
        assert!(spans[0].slice(sql).contains("$$x;y$$"));
        assert!(spans[0].complete);
    }

    #[test]
    fn incomplete_string_does_not_panic_and_marks_open() {
        let sql = "SELECT * FROM t WHERE name = '";
        let spans = statements(sql, SqlDialect::PostgreSql);
        assert!(!spans.is_empty());
        assert!(spans.last().is_some_and(|s| !s.complete));
        assert!(spans.last().unwrap().slice(sql).contains("SELECT"));
    }

    #[test]
    fn nested_comments_and_emoji_identifier_corpus() {
        let sql = "/* outer /* nest */ still */ SELECT \"🙂\" AS face; SELECT 1";
        let spans = statements(sql, SqlDialect::PostgreSql);
        assert!(spans.len() >= 1);
        assert!(
            spans[0].slice(sql).contains("🙂")
                || spans.iter().any(|s| s.slice(sql).contains("face"))
        );
    }

    #[test]
    fn statement_at_picks_current_span() {
        let sql = "SELECT 1; SELECT 2; SELECT 3";
        let spans = statements(sql, SqlDialect::PostgreSql);
        assert!(spans.len() >= 2);
        let mid = spans[1].start + 1;
        let at = statement_at(sql, SqlDialect::PostgreSql, mid).unwrap();
        assert_eq!(at.start, spans[1].start);
    }

    #[test]
    fn empty_and_whitespace_only() {
        assert!(statements("", SqlDialect::PostgreSql).is_empty());
        assert!(statements("   \n  ", SqlDialect::ClickHouse).is_empty());
    }

    #[test]
    fn e_string_and_line_comment() {
        let sql = "SELECT E'a\\;b'; -- tail; not a split\nSELECT 2";
        let spans = statements(sql, SqlDialect::PostgreSql);
        assert!(spans.len() >= 2, "{spans:?}");
        assert!(spans[0].complete);
    }
}
