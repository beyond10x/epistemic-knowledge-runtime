//! `ekr explain`: the kernel's explanation chain over one verified read, in `ekr.explanation/2`
//! or, asked for it, `ekr.explanation/3`: the same document with `names`, the names map of the
//! ids it holds (`kernel.yaml`, `ekr.kernel.ExplanationV3`; ekr-mcp-fixes-20261015 #3).

use std::ops::Range;

use ekr_core::{AssertionId, TransactionId};
use ekr_graph::{CanonicalValue, Object};
use ekr_kernel::{ExplanationLink, ExplanationResult, Runtime, TransactionRecord, VerifiedRead};
use serde_json::{Map, Value};

use crate::exit::Failure;

/// The format literal of `ekr.explanation/3`.
pub(super) const EXPLANATION_V3_FORMAT: &str = "ekr.explanation/3";

/// `ekr.kernel.ExplanationFormat`: which explanation `ekr explain --format` and the MCP tool
/// `explain` answer. `ekr explain` answers /2 when none is named; the MCP tool /3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// `ekr.explanation/2`: the kernel's result as it is.
    #[value(name = "ekr.explanation/2")]
    V2,
    /// `ekr.explanation/3`: /2 with `names`.
    #[value(name = "ekr.explanation/3")]
    V3,
}

impl Format {
    /// The format's literal.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::V2 => ExplanationResult::FORMAT,
            Self::V3 => EXPLANATION_V3_FORMAT,
        }
    }

    /// The format a literal names, if it names one.
    pub(super) fn named(text: &str) -> Option<Self> {
        [Self::V2, Self::V3]
            .into_iter()
            .find(|format| format.name() == text)
    }
}

/// How much of each evidence record `--documents` answers: a byte range of the record's
/// retained bytes, `limit` long, from `offset`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Bounds {
    /// The first byte answered. Absent: the window is centred on the cited text when the record
    /// holds it, and starts at 0 when it does not.
    pub(super) offset: Option<u64>,
    /// The most bytes answered; [`Bounds::DEFAULT_LIMIT`] when absent. At least 1.
    pub(super) limit: Option<u64>,
}

impl Bounds {
    /// 64 KiB: what `--documents` answers of one evidence record when no `limit` is given.
    pub(super) const DEFAULT_LIMIT: u64 = 64 * 1024;

    /// The byte range of a `length`-byte record this answers. With an `offset` it is the raw
    /// range `offset..offset + limit`, clamped to the record, whatever characters it cuts.
    /// Without one it is centred on the first of `cited` the record holds, or starts at 0, and
    /// moves its ends inward to character boundaries when the record is UTF-8, so its `text`
    /// is never lost to a cut it chose itself.
    fn range(self, bytes: &[u8], cited: &[&[u8]]) -> Range<usize> {
        let length = bytes.len();
        let limit =
            usize::try_from(self.limit.unwrap_or(Self::DEFAULT_LIMIT)).unwrap_or(usize::MAX);
        if let Some(offset) = self.offset {
            let start = usize::try_from(offset).map_or(length, |offset| offset.min(length));
            return start..start.saturating_add(limit).min(length);
        }
        if length <= limit {
            return 0..length;
        }
        let at = cited.iter().find_map(|needle| {
            find(bytes, needle).map(|at| {
                if needle.len() >= limit {
                    at
                } else {
                    (at + needle.len() / 2).saturating_sub(limit / 2)
                }
            })
        });
        let start = at.unwrap_or(0).min(length - limit);
        let (mut start, mut end) = (start, start + limit);
        if let Ok(text) = std::str::from_utf8(bytes) {
            while !text.is_char_boundary(start) {
                start += 1;
            }
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            end = end.max(start);
        }
        start..end
    }
}

/// The first position of `needle` in `haystack`; never for an empty needle.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Captures the newest revision once and explains the assertion from that capture alone, in
/// `ekr.explanation/2`: proposals, commit receipts and evidence by reference.
///
/// With `documents`, each link also carries the whole retained record it references, read from
/// the same capture: a Proposal link its proposal record as `record`, every commit (a Commit link,
/// a Lifecycle or an Attachment link's `commit`) its commit receipt as `receipt`, and an Evidence
/// link its retained payload, read through `Runtime::content` at the link's own `content_hash`:
/// `payload` in base64 and, when the bytes are UTF-8, `text`.
///
/// An evidence payload is answered within `documents`' [`Bounds`]: a record longer than the
/// range answered carries `offset` (the range's first byte), `record_length` (the whole record's
/// bytes) and `truncated: true` beside `payload` and `text`, which hold the range alone. The cited
/// text a window is centred on is the string value of an assertion on the chain, in chain order:
/// evidence names no span of its record.
///
/// In `ekr.explanation/3` the document, so rendered, also carries `names`, the names map of every
/// id it holds (`ekr_views::ontology_names`), read from the ontology of the same capture, and
/// `format` names /3.
pub(super) fn run(
    runtime: &Runtime,
    assertion: AssertionId,
    documents: Option<Bounds>,
    format: Format,
) -> Result<Value, Failure> {
    let read = runtime.read(None)?;
    let mut value = rendered(runtime, &read, assertion, documents)?;
    if format == Format::V3 {
        named(&mut value, &read.graph.ontology, EXPLANATION_V3_FORMAT)?;
    }
    Ok(value)
}

/// `document` as the named format `format`: its `format` replaced and `names` added, the names
/// map of every id the document held before, read from `ontology` (`views.yaml`, "Named
/// formats"). Every key of a document `ekr` prints ascends by its text, so where they are
/// inserted does not matter.
pub(super) fn named(
    document: &mut Value,
    ontology: &ekr_ontology::Ontology,
    format: &'static str,
) -> Result<(), Failure> {
    let names = ekr_views::ontology_names(ontology, document);
    let Some(fields) = document.as_object_mut() else {
        return Err(Failure::fault("a named document is not an object"));
    };
    fields.insert("format".to_owned(), Value::String(format.to_owned()));
    fields.insert(
        "names".to_owned(),
        serde_json::to_value(names).map_err(Failure::fault)?,
    );
    Ok(())
}

/// The `ekr.explanation/2` rendering of `assertion`'s chain in the capture `read`, with the
/// records `documents` asks for.
fn rendered(
    runtime: &Runtime,
    read: &VerifiedRead,
    assertion: AssertionId,
    documents: Option<Bounds>,
) -> Result<Value, Failure> {
    let explained = read.explain(assertion)?;
    let mut value = serde_json::to_value(&explained).map_err(Failure::fault)?;
    let Some(bounds) = documents else {
        return Ok(value);
    };
    let cited: Vec<&[u8]> = explained
        .links
        .iter()
        .filter_map(|link| match link {
            ExplanationLink::Assertion(assertion) => match &assertion.object {
                Object::Value(CanonicalValue::String(text)) => Some(text.as_bytes()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let Some(links) = value.get_mut("links").and_then(Value::as_array_mut) else {
        return Err(Failure::fault("explanation without links"));
    };
    for (link, rendered) in explained.links.iter().zip(links.iter_mut()) {
        let Some(fields) = rendered.as_object_mut() else {
            return Err(Failure::fault("a link is not an object"));
        };
        match link {
            ExplanationLink::Proposal(proposal) => {
                let record = held(read, proposal.transaction_id)?;
                insert(fields, "record", &record.proposal)?;
            }
            ExplanationLink::Commit(commit) => {
                let record = held(read, commit.transaction_id)?;
                insert(fields, "receipt", &record.committed)?;
            }
            ExplanationLink::Lifecycle(change) => {
                let record = held(read, change.commit.transaction_id)?;
                let Some(commit) = fields.get_mut("commit").and_then(Value::as_object_mut) else {
                    return Err(Failure::fault("a Lifecycle link without its commit"));
                };
                insert(commit, "receipt", &record.committed)?;
            }
            ExplanationLink::Attachment(attached) => {
                let record = held(read, attached.commit.transaction_id)?;
                let Some(commit) = fields.get_mut("commit").and_then(Value::as_object_mut) else {
                    return Err(Failure::fault("an Attachment link without its commit"));
                };
                insert(commit, "receipt", &record.committed)?;
            }
            ExplanationLink::Evidence(evidence) => {
                let bytes = runtime
                    .content(&evidence.content_hash)
                    .map_err(Failure::store)?
                    .ok_or_else(|| {
                        Failure::fault(format!(
                            "evidence {} payload {} is not retained",
                            evidence.id, evidence.content_hash
                        ))
                    })?;
                let range = bounds.range(&bytes, &cited);
                let answered = &bytes[range.clone()];
                fields.insert("payload".to_owned(), Value::String(super::base64(answered)));
                if let Ok(text) = std::str::from_utf8(answered) {
                    fields.insert("text".to_owned(), Value::String(text.to_owned()));
                }
                if range != (0..bytes.len()) {
                    fields.insert("offset".to_owned(), Value::from(range.start));
                    fields.insert("record_length".to_owned(), Value::from(bytes.len()));
                    fields.insert("truncated".to_owned(), Value::Bool(true));
                }
            }
            ExplanationLink::Assertion(_)
            | ExplanationLink::Seed(_)
            | ExplanationLink::Validation(_) => {}
        }
    }
    Ok(value)
}

/// The capture's retained record of a transaction a link names.
fn held(read: &VerifiedRead, transaction: TransactionId) -> Result<&TransactionRecord, Failure> {
    read.transactions
        .get(&transaction)
        .ok_or_else(|| Failure::fault(format!("transaction {transaction} is not retained")))
}

fn insert(
    fields: &mut Map<String, Value>,
    name: &str,
    record: &impl serde::Serialize,
) -> Result<(), Failure> {
    fields.insert(
        name.to_owned(),
        serde_json::to_value(record).map_err(Failure::fault)?,
    );
    Ok(())
}
