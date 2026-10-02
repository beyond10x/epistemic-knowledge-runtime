//! `ekr code-names <file>...`: which of the store's names the given source files carry as
//! literals (`ekr.views.FindCodeNames`, `ekr.code-names/1`), so a consumer can hold its
//! store-reading code generic over any ontology.
//!
//! Reads each file, then the store at the head or at `--at`, and writes nothing. Findings are
//! the answer, not a failure: the verb exits 0 whatever `meta.findings` counts. A file that does
//! not read, or is not UTF-8 text, is a fault naming it (exit 1); a revision the store does not
//! hold is `ekr.views.RevisionNotFound` (exit 2).

use std::path::PathBuf;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::SourceText;

use super::view::project_refusal;
use crate::exit::Failure;

/// Every file's text, with its path exactly as given.
pub(super) fn read(files: &[PathBuf]) -> Result<Vec<SourceText>, Failure> {
    files
        .iter()
        .map(|file| {
            let path = file.display().to_string();
            let bytes = std::fs::read(file)
                .map_err(|error| Failure::fault(format!("reading source {path}: {error}")))?;
            let text = String::from_utf8(bytes)
                .map_err(|_| Failure::fault(format!("source {path} is not UTF-8 text")))?;
            Ok(SourceText { path, text })
        })
        .collect()
}

/// The `ekr.code-names/1` document of `sources` against revision `at` (the head when absent).
pub(super) fn run(
    runtime: &Runtime,
    at: Option<u64>,
    sources: &[SourceText],
    words: bool,
) -> Result<serde_json::Value, Failure> {
    let mode = if words {
        ekr_views::CodeNameMode::Words
    } else {
        ekr_views::CodeNameMode::Literals
    };
    let answer =
        ekr_views::find_code_names_with_mode(runtime, at.map(RevisionNumber::new), sources, mode)
            .map_err(|error| match project_refusal(&error) {
            Some(name) => Failure::refused(name, error),
            None => Failure::unread(error),
        })?;
    serde_json::from_slice(&answer.bytes).map_err(Failure::fault)
}
