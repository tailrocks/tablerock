//! Streaming export encoder → atomic file.
//!
//! Pages are appended as they arrive. Cancel/failure calls [`StreamExporter::abort`],
//! which removes the incomplete temp (and never leaves a partial destination).

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tablerock_files::{AtomicFileWriter, FileEffectError, validate_export_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamExportFormat {
    Csv,
    Tsv,
    Json,
}

impl StreamExportFormat {
    pub fn parse(label: &str) -> Self {
        match label.to_ascii_lowercase().as_str() {
            "json" => Self::Json,
            "tsv" => Self::Tsv,
            _ => Self::Csv,
        }
    }
}

#[derive(Debug)]
pub enum StreamExportError {
    Path(FileEffectError),
    Cancelled { bytes_written: u64 },
    Io(String),
}

impl std::fmt::Display for StreamExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(e) => write!(f, "{e}"),
            Self::Cancelled { bytes_written } => {
                write!(f, "export cancelled after {bytes_written} bytes")
            }
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StreamExportError {}

/// Incremental exporter over an [`AtomicFileWriter`].
pub struct StreamExporter {
    writer: AtomicFileWriter,
    format: StreamExportFormat,
    header_written: bool,
    json_row_count: u64,
    rows_written: u64,
    cancel: Option<Arc<AtomicBool>>,
}

impl StreamExporter {
    pub fn create(
        path: &str,
        format: StreamExportFormat,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<Self, StreamExportError> {
        let dest = validate_export_path(path).map_err(StreamExportError::Path)?;
        let mut writer = AtomicFileWriter::create(dest).map_err(StreamExportError::Path)?;
        if format == StreamExportFormat::Json {
            writer.write_all(b"[\n").map_err(StreamExportError::Path)?;
        }
        Ok(Self {
            writer,
            format,
            header_written: false,
            json_row_count: 0,
            rows_written: 0,
            cancel,
        })
    }

    #[must_use]
    pub fn rows_written(&self) -> u64 {
        self.rows_written
    }

    #[must_use]
    pub fn bytes_written(&self) -> u64 {
        self.writer.bytes_written()
    }

    fn check_cancel(&self) -> Result<(), StreamExportError> {
        if self
            .cancel
            .as_ref()
            .is_some_and(|c| c.load(Ordering::SeqCst))
        {
            Err(StreamExportError::Cancelled {
                bytes_written: self.writer.bytes_written(),
            })
        } else {
            Ok(())
        }
    }

    /// Append one page of display rows. Writes header once for CSV/TSV.
    pub fn write_page(
        &mut self,
        columns: &[String],
        rows: &[Vec<String>],
    ) -> Result<(), StreamExportError> {
        self.check_cancel()?;
        if columns.is_empty() {
            return Ok(());
        }
        match self.format {
            StreamExportFormat::Csv => self.write_csv_page(columns, rows)?,
            StreamExportFormat::Tsv => self.write_tsv_page(columns, rows)?,
            StreamExportFormat::Json => self.write_json_page(columns, rows)?,
        }
        self.rows_written = self.rows_written.saturating_add(rows.len() as u64);
        self.check_cancel()
    }

    fn write_csv_page(
        &mut self,
        columns: &[String],
        rows: &[Vec<String>],
    ) -> Result<(), StreamExportError> {
        if !self.header_written {
            let header = columns
                .iter()
                .map(|c| csv_escape(c))
                .collect::<Vec<_>>()
                .join(",");
            self.writer
                .write_all(header.as_bytes())
                .map_err(StreamExportError::Path)?;
            self.writer
                .write_all(b"\n")
                .map_err(StreamExportError::Path)?;
            self.header_written = true;
        }
        for row in rows {
            let line = columns
                .iter()
                .enumerate()
                .map(|(i, _)| csv_escape(row.get(i).map(String::as_str).unwrap_or("")))
                .collect::<Vec<_>>()
                .join(",");
            self.writer
                .write_all(line.as_bytes())
                .map_err(StreamExportError::Path)?;
            self.writer
                .write_all(b"\n")
                .map_err(StreamExportError::Path)?;
        }
        Ok(())
    }

    fn write_tsv_page(
        &mut self,
        columns: &[String],
        rows: &[Vec<String>],
    ) -> Result<(), StreamExportError> {
        if !self.header_written {
            let header = columns.join("\t");
            self.writer
                .write_all(header.as_bytes())
                .map_err(StreamExportError::Path)?;
            self.writer
                .write_all(b"\n")
                .map_err(StreamExportError::Path)?;
            self.header_written = true;
        }
        for row in rows {
            let line = columns
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    row.get(i)
                        .map(|c| c.replace(['\t', '\n', '\r'], " "))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join("\t");
            self.writer
                .write_all(line.as_bytes())
                .map_err(StreamExportError::Path)?;
            self.writer
                .write_all(b"\n")
                .map_err(StreamExportError::Path)?;
        }
        Ok(())
    }

    fn write_json_page(
        &mut self,
        columns: &[String],
        rows: &[Vec<String>],
    ) -> Result<(), StreamExportError> {
        for row in rows {
            if self.json_row_count > 0 {
                self.writer
                    .write_all(b",\n")
                    .map_err(StreamExportError::Path)?;
            }
            self.writer
                .write_all(b"  {")
                .map_err(StreamExportError::Path)?;
            for (ci, col) in columns.iter().enumerate() {
                if ci > 0 {
                    self.writer
                        .write_all(b", ")
                        .map_err(StreamExportError::Path)?;
                }
                let val = row.get(ci).map(String::as_str).unwrap_or("");
                let piece = format!("\"{}\":{}", json_escape_key(col), json_value(val));
                self.writer
                    .write_all(piece.as_bytes())
                    .map_err(StreamExportError::Path)?;
            }
            self.writer
                .write_all(b"}")
                .map_err(StreamExportError::Path)?;
            self.json_row_count = self.json_row_count.saturating_add(1);
        }
        Ok(())
    }

    /// Finalize the file (JSON closes the array). Success renames temp → dest.
    pub fn finish(mut self) -> Result<StreamExportOutcome, StreamExportError> {
        self.check_cancel()?;
        if self.format == StreamExportFormat::Json {
            self.writer
                .write_all(b"\n]\n")
                .map_err(StreamExportError::Path)?;
        }
        let bytes = self.writer.finish().map_err(StreamExportError::Path)?;
        Ok(StreamExportOutcome {
            bytes,
            rows: self.rows_written,
        })
    }

    /// Drop incomplete output (temp removed; destination untouched).
    pub fn abort(self) {
        self.writer.abort();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamExportOutcome {
    pub bytes: u64,
    pub rows: u64,
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn json_escape_key(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_value(s: &str) -> String {
    if s == "NULL" {
        "null".into()
    } else if s == "true" || s == "false" || s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
        s.to_owned()
    } else {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// Re-query stream export: pull pages from a callback until exhausted or cancelled.
///
/// `next_page` returns `None` when the stream ends. On cancel, aborts the file
/// and returns [`StreamExportError::Cancelled`].
pub fn run_stream_export<F>(
    path: &str,
    format: StreamExportFormat,
    cancel: Arc<AtomicBool>,
    mut next_page: F,
) -> Result<StreamExportOutcome, StreamExportError>
where
    F: FnMut() -> Result<Option<(Vec<String>, Vec<Vec<String>>)>, StreamExportError>,
{
    let mut exporter = StreamExporter::create(path, format, Some(Arc::clone(&cancel)))?;
    loop {
        if cancel.load(Ordering::SeqCst) {
            let bytes = exporter.bytes_written();
            exporter.abort();
            return Err(StreamExportError::Cancelled {
                bytes_written: bytes,
            });
        }
        match next_page()? {
            Some((columns, rows)) => {
                if let Err(e) = exporter.write_page(&columns, &rows) {
                    exporter.abort();
                    return Err(e);
                }
            }
            None => break,
        }
    }
    exporter.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn scratch(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tablerock-stream-export-{}-{}-{label}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn multi_page_csv_finishes_atomically() {
        let dir = scratch("csv-ok");
        let dest = dir.join("out.csv");
        let path = dest.to_string_lossy().into_owned();
        let columns = vec!["id".into(), "name".into()];
        let mut pages = vec![
            Some((
                columns.clone(),
                vec![vec!["1".into(), "a".into()], vec!["2".into(), "b".into()]],
            )),
            Some((columns.clone(), vec![vec!["3".into(), "c".into()]])),
            None,
        ]
        .into_iter();
        let cancel = Arc::new(AtomicBool::new(false));
        let outcome = run_stream_export(&path, StreamExportFormat::Csv, cancel, || {
            Ok(pages.next().flatten())
        })
        .unwrap();
        assert_eq!(outcome.rows, 3);
        assert!(outcome.bytes > 0);
        let body = fs::read_to_string(&dest).unwrap();
        assert!(body.starts_with("id,name\n"));
        assert!(body.contains("3,c\n"));
        // No temp left.
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, vec!["out.csv"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_mid_stream_removes_partial_and_never_writes_dest() {
        let dir = scratch("csv-cancel");
        let dest = dir.join("partial.csv");
        let path = dest.to_string_lossy().into_owned();
        let cancel = Arc::new(AtomicBool::new(false));
        let columns = vec!["id".into()];
        let mut page_idx = 0_u32;
        let cancel_flag = Arc::clone(&cancel);
        let err = run_stream_export(&path, StreamExportFormat::Csv, cancel, || {
            page_idx += 1;
            if page_idx == 1 {
                Ok(Some((
                    columns.clone(),
                    vec![vec!["1".into()], vec!["2".into()]],
                )))
            } else if page_idx == 2 {
                cancel_flag.store(true, Ordering::SeqCst);
                Ok(Some((columns.clone(), vec![vec!["3".into()]])))
            } else {
                Ok(None)
            }
        })
        .unwrap_err();
        assert!(matches!(err, StreamExportError::Cancelled { .. }), "{err}");
        assert!(!dest.exists(), "destination must not exist after cancel");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            leftovers.iter().all(|n| !n.contains("tablerock-tmp")),
            "temp must be removed: {leftovers:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_stream_is_valid_array() {
        let dir = scratch("json-ok");
        let dest = dir.join("out.json");
        let path = dest.to_string_lossy().into_owned();
        let columns = vec!["n".into()];
        let mut pages = vec![
            Some((columns.clone(), vec![vec!["1".into()]])),
            Some((columns, vec![vec!["2".into()]])),
            None,
        ]
        .into_iter();
        let cancel = Arc::new(AtomicBool::new(false));
        run_stream_export(&path, StreamExportFormat::Json, cancel, || {
            Ok(pages.next().flatten())
        })
        .unwrap();
        let body = fs::read_to_string(&dest).unwrap();
        assert!(body.starts_with("[\n"));
        assert!(body.contains("\"n\":1"));
        assert!(body.contains("\"n\":2"));
        assert!(body.trim_end().ends_with(']'));
        let _ = fs::remove_dir_all(&dir);
    }
}
