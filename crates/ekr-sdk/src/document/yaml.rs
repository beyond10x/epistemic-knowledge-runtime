//! The tag-aware YAML writer.
//!
//! The readers of both YAML formats take a variant only as a YAML tag, `!Kind value`, and refuse
//! the one-key JSON form `{"!Kind": value}` (`docs/cli.md`, "`ekr schema`"). So a document is
//! never written as JSON: every externally tagged enum of these models — an operation, a subject,
//! a predicate, an object, an evidence source — is written as a tag on its payload, a unit variant
//! (`Proposed`, `Active`) as a plain word, and a value (`{value_kind, value}`) as a mapping whose
//! `value_kind` comes first, as the readers require. A string YAML would read as another type
//! (`"24.90"`, `"true"`, an all-digit hash) is quoted.

use serde::Serialize;

use super::DocumentError;

/// `value` as one YAML document, ending with a newline.
///
/// # Errors
/// [`DocumentError::Yaml`] when the writer refuses a value, such as an enum nested directly in
/// another enum's variant, which none of this module's models holds.
pub fn to_yaml<T: Serialize + ?Sized>(value: &T) -> Result<String, DocumentError> {
    let mut text = serde_yaml_ng::to_string(value)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}
