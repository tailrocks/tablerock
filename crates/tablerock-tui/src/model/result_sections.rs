//! Multi-statement result sections (one section per statement, ordered).
//!
//! Failure of a later statement never hides earlier completed sections.

/// Outcome kind for one statement section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatementSectionKind {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

impl StatementSectionKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

/// One ordered result section for a multi-statement script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementSection {
    pub ordinal: u32,
    pub command_tag: String,
    pub kind: StatementSectionKind,
    pub rows: Option<u64>,
    pub elapsed_ms: Option<u64>,
    pub error: Option<String>,
    pub pinned: bool,
}

impl StatementSection {
    #[must_use]
    pub fn summary_line(&self) -> String {
        let rows = self
            .rows
            .map(|n| format!("{n} rows"))
            .unwrap_or_else(|| "—".into());
        let time = self
            .elapsed_ms
            .map(|ms| format!("{ms} ms"))
            .unwrap_or_else(|| "—".into());
        let pin = if self.pinned { " 📌" } else { "" };
        let err = self
            .error
            .as_deref()
            .map(|e| format!(" · {e}"))
            .unwrap_or_default();
        format!(
            "#{ordinal} {tag} · {kind} · {rows} · {time}{pin}{err}",
            ordinal = self.ordinal,
            tag = if self.command_tag.is_empty() {
                "stmt"
            } else {
                &self.command_tag
            },
            kind = self.kind.label(),
        )
    }
}

/// Ordered multi-statement result panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResultSectionsModel {
    pub sections: Vec<StatementSection>,
    pub selected: usize,
}

impl ResultSectionsModel {
    /// Append a section preserving ordinal order (append-only for script run).
    pub fn push(&mut self, section: StatementSection) {
        self.sections.push(section);
    }

    /// Mark section failed without removing earlier sections.
    pub fn mark_failed(&mut self, ordinal: u32, error: impl Into<String>) {
        if let Some(s) = self.sections.iter_mut().find(|s| s.ordinal == ordinal) {
            s.kind = StatementSectionKind::Failed;
            s.error = Some(error.into());
        }
    }

    pub fn pin_selected(&mut self) {
        if let Some(s) = self.sections.get_mut(self.selected) {
            s.pinned = !s.pinned;
        }
    }

    #[must_use]
    pub fn completion_summary(&self) -> String {
        let total = self.sections.len();
        let failed = self
            .sections
            .iter()
            .filter(|s| s.kind == StatementSectionKind::Failed)
            .count();
        let completed = self
            .sections
            .iter()
            .filter(|s| s.kind == StatementSectionKind::Completed)
            .count();
        format!("statements: {total} · completed {completed} · failed {failed}")
    }

    #[must_use]
    pub fn display_lines(&self) -> Vec<String> {
        let mut lines: Vec<_> = self.sections.iter().map(|s| s.summary_line()).collect();
        lines.push(self.completion_summary());
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_failure_keeps_first_and_third_explicit() {
        let mut m = ResultSectionsModel::default();
        m.push(StatementSection {
            ordinal: 1,
            command_tag: "SELECT".into(),
            kind: StatementSectionKind::Completed,
            rows: Some(2),
            elapsed_ms: Some(3),
            error: None,
            pinned: false,
        });
        m.push(StatementSection {
            ordinal: 2,
            command_tag: "INSERT".into(),
            kind: StatementSectionKind::Running,
            rows: None,
            elapsed_ms: None,
            error: None,
            pinned: false,
        });
        m.push(StatementSection {
            ordinal: 3,
            command_tag: "SELECT".into(),
            kind: StatementSectionKind::Pending,
            rows: None,
            elapsed_ms: None,
            error: None,
            pinned: false,
        });
        m.mark_failed(2, "unique violation");
        // First still completed.
        assert_eq!(m.sections[0].kind, StatementSectionKind::Completed);
        assert!(m.sections[0].summary_line().contains("completed"));
        // Middle failed with error visible.
        assert_eq!(m.sections[1].kind, StatementSectionKind::Failed);
        assert!(m.sections[1].summary_line().contains("unique violation"));
        // Third still present (pending), not hidden.
        assert_eq!(m.sections[2].kind, StatementSectionKind::Pending);
        assert_eq!(m.sections.len(), 3);
        assert!(m.completion_summary().contains("failed 1"));
    }
}
