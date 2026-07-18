//! Cell inspector projection (text / hex / structured label).

use super::grid::{CellDistinction, ProjectedCell};

/// Full-value inspector for the selected cell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InspectorModel {
    pub open: bool,
    pub title: String,
    pub kind_label: String,
    pub text: String,
    pub hex: String,
    pub byte_len: u64,
    pub original_byte_len: Option<u64>,
    pub stale: bool,
}

impl InspectorModel {
    #[must_use]
    pub fn from_cell(title: impl Into<String>, cell: &ProjectedCell, stale: bool) -> Self {
        let hex = if cell.distinction == CellDistinction::Binary
            || cell.distinction == CellDistinction::Unknown
            || cell.distinction == CellDistinction::Invalid
        {
            cell.text.clone()
        } else {
            cell.text
                .as_bytes()
                .iter()
                .take(32)
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        Self {
            open: true,
            title: title.into(),
            kind_label: cell.distinction.label().into(),
            text: cell.display(),
            hex,
            byte_len: cell.byte_len,
            original_byte_len: cell.original_byte_len,
            stale,
        }
    }

    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        if !self.open {
            return Vec::new();
        }
        let mut out = vec![
            format!("inspector: {}", self.title),
            format!("kind: {}", self.kind_label),
            format!("bytes: {}", self.byte_len),
        ];
        if let Some(orig) = self.original_byte_len {
            out.push(format!("original bytes: {orig} (truncated)"));
        }
        if self.stale {
            out.push("stale: yes".into());
        }
        out.push(format!("text: {}", self.text));
        out.push(format!("hex: {}", self.hex));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::grid::{CellDistinction, ProjectedCell};

    #[test]
    fn inspector_marks_truncation_and_stale() {
        let cell = ProjectedCell {
            text: "hello".into(),
            distinction: CellDistinction::Truncated,
            byte_len: 5,
            original_byte_len: Some(50),
        };
        let insp = InspectorModel::from_cell("users.id", &cell, true);
        let lines = insp.lines().join("\n");
        assert!(lines.contains("truncated"));
        assert!(lines.contains("stale: yes"));
        assert!(lines.contains("kind: truncated"));
    }
}
