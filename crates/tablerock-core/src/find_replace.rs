//! Shared find/replace for SQL editors (TUI and native-equivalent modes).
//!
//! Modes match the native sheet contract:
//! - `literal` (case-insensitive by default)
//! - `case_sensitive`
//! - `whole_word` (Unicode letter/number/`_` boundaries, case-insensitive)
//! - `regular_expression`
//!
//! Scope is whole document or a frozen selection byte range. Replace-all is
//! bounded at 10_000 matches. Zero-width regex matches advance one scalar so
//! traversal stays finite.

use std::{error::Error, fmt, ops::Range};

use regex::{Regex, RegexBuilder};

/// Hard cap shared with native find/replace (evidence 641).
pub const MAX_FIND_REPLACE_MATCHES: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindReplaceMode {
    /// Case-insensitive literal (default native mode).
    Literal,
    /// Case-sensitive literal.
    CaseSensitive,
    /// Unicode whole-word literal, case-insensitive.
    WholeWord,
    /// Caller-supplied regular expression (no case-insensitivity forced).
    RegularExpression,
}

impl FindReplaceMode {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "literal" | "ci" | "i" | "case_insensitive" | "ignore_case" => Some(Self::Literal),
            "case_sensitive" | "cs" | "sensitive" => Some(Self::CaseSensitive),
            "whole_word" | "word" | "w" => Some(Self::WholeWord),
            "regular_expression" | "regex" | "re" => Some(Self::RegularExpression),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindReplaceScope {
    Document,
    Selection,
}

impl FindReplaceScope {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "document" | "doc" | "all_text" => Some(Self::Document),
            "selection" | "sel" => Some(Self::Selection),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindReplaceError {
    EmptyPattern,
    InvalidPattern(String),
    InvalidScope,
    ReplacementLimit,
}

impl fmt::Display for FindReplaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPattern => f.write_str("find pattern is empty"),
            Self::InvalidPattern(detail) => write!(f, "invalid find pattern: {detail}"),
            Self::InvalidScope => f.write_str("selection scope requires a non-empty selection"),
            Self::ReplacementLimit => {
                write!(f, "replace-all exceeded {MAX_FIND_REPLACE_MATCHES} matches")
            }
        }
    }
}

impl Error for FindReplaceError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindMatch {
    pub range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceOutcome {
    pub text: String,
    /// Cursor/selection start after the operation (byte index).
    pub cursor: usize,
    pub count: usize,
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn escape_literal(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() * 2);
    for ch in pattern.chars() {
        if matches!(
            ch,
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn compile(pattern: &str, mode: FindReplaceMode) -> Result<Regex, FindReplaceError> {
    if pattern.is_empty() {
        return Err(FindReplaceError::EmptyPattern);
    }
    let (source, case_insensitive) = match mode {
        FindReplaceMode::Literal => (escape_literal(pattern), true),
        FindReplaceMode::CaseSensitive => (escape_literal(pattern), false),
        FindReplaceMode::WholeWord => {
            // Unicode-ish boundary: not preceded/followed by letter/number/_.
            let escaped = escape_literal(pattern);
            (format!(r"(?P<tr>{escaped})"), true)
        }
        FindReplaceMode::RegularExpression => (pattern.to_owned(), false),
    };
    RegexBuilder::new(&source)
        .case_insensitive(case_insensitive)
        .unicode(true)
        .build()
        .map_err(|error| FindReplaceError::InvalidPattern(error.to_string()))
}

fn scope_range(
    text: &str,
    scope: FindReplaceScope,
    selection: Option<Range<usize>>,
) -> Result<Range<usize>, FindReplaceError> {
    match scope {
        FindReplaceScope::Document => Ok(0..text.len()),
        FindReplaceScope::Selection => {
            let range = selection.ok_or(FindReplaceError::InvalidScope)?;
            if range.start > range.end || range.end > text.len() || range.start == range.end {
                return Err(FindReplaceError::InvalidScope);
            }
            // Ensure char boundaries.
            if !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
                return Err(FindReplaceError::InvalidScope);
            }
            Ok(range)
        }
    }
}

fn whole_word_ok(text: &str, range: &Range<usize>) -> bool {
    let before = text[..range.start].chars().next_back();
    let after = text[range.end..].chars().next();
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

fn collect_matches(
    text: &str,
    pattern: &str,
    mode: FindReplaceMode,
    scope: Range<usize>,
) -> Result<Vec<FindMatch>, FindReplaceError> {
    let re = compile(pattern, mode)?;
    let hay = &text[scope.clone()];
    let mut matches = Vec::new();
    let mut search_from = 0usize;
    while search_from <= hay.len() {
        let Some(m) = re.find_at(hay, search_from) else {
            break;
        };
        let abs = scope.start + m.start()..scope.start + m.end();
        if mode == FindReplaceMode::WholeWord && !whole_word_ok(text, &abs) {
            // Advance past this match start.
            let step = if m.start() == m.end() {
                hay[m.end()..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1)
            } else {
                (m.end() - m.start()).max(1)
            };
            search_from = m.start().saturating_add(step);
            if search_from <= m.start() {
                break;
            }
            continue;
        }
        matches.push(FindMatch { range: abs });
        if matches.len() > MAX_FIND_REPLACE_MATCHES {
            return Err(FindReplaceError::ReplacementLimit);
        }
        // Finite zero-width advance.
        let step = if m.start() == m.end() {
            hay[m.end()..]
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or(1)
        } else {
            m.end() - m.start()
        };
        let next = m.end().max(m.start() + step);
        if next <= search_from {
            break;
        }
        search_from = next;
    }
    Ok(matches)
}

/// List matches in document order within the chosen scope.
pub fn find_all(
    text: &str,
    pattern: &str,
    mode: FindReplaceMode,
    scope: FindReplaceScope,
    selection: Option<Range<usize>>,
) -> Result<Vec<FindMatch>, FindReplaceError> {
    let range = scope_range(text, scope, selection)?;
    collect_matches(text, pattern, mode, range)
}

/// First match at or after `from` (byte index), wrapping within scope.
pub fn find_next(
    text: &str,
    pattern: &str,
    mode: FindReplaceMode,
    scope: FindReplaceScope,
    selection: Option<Range<usize>>,
    from: usize,
) -> Result<Option<FindMatch>, FindReplaceError> {
    let matches = find_all(text, pattern, mode, scope, selection)?;
    if matches.is_empty() {
        return Ok(None);
    }
    Ok(matches
        .iter()
        .find(|m| m.range.start >= from)
        .cloned()
        .or_else(|| matches.first().cloned()))
}

fn apply_replacement(
    mode: FindReplaceMode,
    re: &Regex,
    hay_slice: &str,
    replacement: &str,
) -> String {
    if mode == FindReplaceMode::RegularExpression {
        re.replace(hay_slice, replacement).into_owned()
    } else {
        replacement.to_owned()
    }
}

/// Replace the first match at/after `from` within scope.
pub fn replace_next(
    text: &str,
    pattern: &str,
    replacement: &str,
    mode: FindReplaceMode,
    scope: FindReplaceScope,
    selection: Option<Range<usize>>,
    from: usize,
) -> Result<Option<ReplaceOutcome>, FindReplaceError> {
    let Some(m) = find_next(text, pattern, mode, scope, selection.clone(), from)? else {
        return Ok(None);
    };
    let re = compile(pattern, mode)?;
    let matched = &text[m.range.clone()];
    let inserted = apply_replacement(mode, &re, matched, replacement);
    let mut out = String::with_capacity(text.len() + inserted.len());
    out.push_str(&text[..m.range.start]);
    out.push_str(&inserted);
    out.push_str(&text[m.range.end..]);
    let cursor = m.range.start + inserted.len();
    Ok(Some(ReplaceOutcome {
        text: out,
        cursor,
        count: 1,
    }))
}

/// Replace all matches in scope (right-to-left safe rebuild via collected ranges).
pub fn replace_all(
    text: &str,
    pattern: &str,
    replacement: &str,
    mode: FindReplaceMode,
    scope: FindReplaceScope,
    selection: Option<Range<usize>>,
) -> Result<ReplaceOutcome, FindReplaceError> {
    let matches = find_all(text, pattern, mode, scope, selection)?;
    if matches.is_empty() {
        return Ok(ReplaceOutcome {
            text: text.to_owned(),
            cursor: 0,
            count: 0,
        });
    }
    let re = compile(pattern, mode)?;
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for m in &matches {
        out.push_str(&text[cursor..m.range.start]);
        let matched = &text[m.range.clone()];
        out.push_str(&apply_replacement(mode, &re, matched, replacement));
        cursor = m.range.end;
    }
    out.push_str(&text[cursor..]);
    Ok(ReplaceOutcome {
        text: out,
        cursor: 0,
        count: matches.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_case_insensitive_and_whole_word() {
        let text = "Foo food FOO";
        // Literal matches Foo, the "foo" prefix inside food, and FOO.
        let all = find_all(
            text,
            "foo",
            FindReplaceMode::Literal,
            FindReplaceScope::Document,
            None,
        )
        .unwrap();
        assert_eq!(all.len(), 3);
        let words = find_all(
            text,
            "foo",
            FindReplaceMode::WholeWord,
            FindReplaceScope::Document,
            None,
        )
        .unwrap();
        assert_eq!(words.len(), 2);
        assert_eq!(&text[words[0].range.clone()], "Foo");
        assert_eq!(&text[words[1].range.clone()], "FOO");
    }

    #[test]
    fn selection_scope_and_replace_all_bound() {
        let text = "aa aa aa";
        let sel = Some(3..5); // middle "aa"
        let out = replace_all(
            text,
            "aa",
            "X",
            FindReplaceMode::CaseSensitive,
            FindReplaceScope::Selection,
            sel,
        )
        .unwrap();
        assert_eq!(out.text, "aa X aa");
        assert_eq!(out.count, 1);
    }

    #[test]
    fn regex_groups_and_zero_width_stays_finite() {
        let text = "a1b2";
        let out = replace_all(
            text,
            r"(\d)",
            "N",
            FindReplaceMode::RegularExpression,
            FindReplaceScope::Document,
            None,
        )
        .unwrap();
        assert_eq!(out.text, "aNbN");
        assert_eq!(out.count, 2);

        // Zero-width: start of line — must not hang.
        let zw = replace_all(
            "ab",
            r"^",
            ">",
            FindReplaceMode::RegularExpression,
            FindReplaceScope::Document,
            None,
        )
        .unwrap();
        assert!(zw.count <= MAX_FIND_REPLACE_MATCHES);
        assert!(zw.text.starts_with('>') || zw.count >= 1);
    }

    #[test]
    fn empty_pattern_and_bad_regex_fail_closed() {
        assert!(matches!(
            find_all(
                "a",
                "",
                FindReplaceMode::Literal,
                FindReplaceScope::Document,
                None
            ),
            Err(FindReplaceError::EmptyPattern)
        ));
        assert!(matches!(
            find_all(
                "a",
                "(",
                FindReplaceMode::RegularExpression,
                FindReplaceScope::Document,
                None
            ),
            Err(FindReplaceError::InvalidPattern(_))
        ));
    }
}
