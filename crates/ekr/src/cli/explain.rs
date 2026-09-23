//! `ekr explain`: the kernel's explanation chain over one verified read.

use ekr_core::AssertionId;
use ekr_kernel::{ExplanationLink, Runtime};
use serde_json::Value;

use crate::exit::Failure;

/// Captures the newest revision once and explains the assertion from that capture alone. Each
/// Evidence link then gains its retained payload, read through `Runtime::content` at the link's
/// own `content_hash`: `payload` in base64 and, when the bytes are UTF-8, `text`.
pub(super) fn run(runtime: &Runtime, assertion: AssertionId) -> Result<Value, Failure> {
    let read = runtime.read(None)?;
    let explained = read.explain(assertion)?;
    let mut value = serde_json::to_value(&explained).map_err(Failure::fault)?;
    let Some(links) = value.get_mut("links").and_then(Value::as_array_mut) else {
        return Err(Failure::fault("explanation without links"));
    };
    for (link, rendered) in explained.links.iter().zip(links.iter_mut()) {
        let ExplanationLink::Evidence(evidence) = link else {
            continue;
        };
        let bytes = runtime
            .content(&evidence.content_hash)
            .map_err(Failure::fault)?
            .ok_or_else(|| {
                Failure::fault(format!(
                    "evidence {} payload {} is not retained",
                    evidence.id, evidence.content_hash
                ))
            })?;
        let Some(fields) = rendered.as_object_mut() else {
            return Err(Failure::fault("an Evidence link is not an object"));
        };
        fields.insert("payload".to_owned(), Value::String(super::base64(&bytes)));
        if let Ok(text) = String::from_utf8(bytes) {
            fields.insert("text".to_owned(), Value::String(text));
        }
    }
    Ok(value)
}
