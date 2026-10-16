//! `ekr explain`: the kernel's explanation chain over one verified read, in `ekr.explanation/2`
//! or, asked for it, `ekr.explanation/3`: the chain scoped to the evidence it cites, bounded as a
//! whole at [`EXPLANATION_CAP`] bytes, with `names`, the names map of the ids it holds
//! (`kernel.yaml`, `ekr.kernel.ExplanationV3`; ekr-mcp-fixes-20261015 #3, #7).

use std::collections::BTreeSet;
use std::ops::Range;

use ekr_core::{AssertionId, TransactionId};
use ekr_graph::{CanonicalValue, Object};
use ekr_kernel::{ExplanationLink, ExplanationResult, Runtime, TransactionRecord, VerifiedRead};
use serde_json::{Map, Value};

use crate::exit::Failure;

/// The format literal of `ekr.explanation/3`.
pub(super) const EXPLANATION_V3_FORMAT: &str = "ekr.explanation/3";

/// The most bytes an `ekr.explanation/3` is printed in while an evidence text is left to cut:
/// 48 KiB (`kernel.yaml`, `ekr.kernel.ExplanationV3`, "the cap").
pub(super) const EXPLANATION_CAP: usize = 48 * 1024;

/// `ekr.kernel.EvidenceScope`: which Evidence links an `ekr.explanation/3` keeps. `cited` when a
/// request names none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Scope {
    /// The Evidence links the chain's Assertion and Attachment links name.
    Cited,
    /// Every Evidence link `ekr.explanation/2` lists.
    All,
}

impl Scope {
    /// The scope's name, as a request and the document write it.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Cited => "cited",
            Self::All => "all",
        }
    }

    /// The scope a name names, if it names one.
    pub(super) fn named(text: &str) -> Option<Self> {
        [Self::Cited, Self::All]
            .into_iter()
            .find(|scope| scope.name() == text)
    }
}

/// What a `--scope` without `ekr.explanation/3` is refused with: the scope is /3's input.
pub(super) const SCOPE_WITHOUT_V3: &str =
    "--scope reads ekr.explanation/3 only; name --format ekr.explanation/3 to use it";

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
/// `ekr.explanation/3` is that rendering scoped by `scope` (`cited` when `None`), with no proposal
/// record and no payload whose text is answered, then `names`, the names map of every id it holds
/// (`ekr_views::ontology_names`) read from the ontology of the same capture, `scope` and
/// `omitted_evidence`, then cut to [`EXPLANATION_CAP`] by [`capped`] (`kernel.yaml`,
/// `ekr.kernel.ExplanationV3`). Scoping comes first, so a record a dropped link would carry is
/// never read. A `scope` with `ekr.explanation/2` is refused as a usage error.
pub(super) fn run(
    runtime: &Runtime,
    assertion: AssertionId,
    documents: Option<Bounds>,
    format: Format,
    scope: Option<Scope>,
) -> Result<Value, Failure> {
    if format == Format::V2 && scope.is_some() {
        return Err(Failure::Usage {
            message: format!("error: {SCOPE_WITHOUT_V3}\n"),
        });
    }
    let read = runtime.read(None)?;
    if format == Format::V2 {
        return Ok(rendered(runtime, &read, assertion, documents, None)?.0);
    }
    let scope = scope.unwrap_or(Scope::Cited);
    let (mut value, omitted) = rendered(runtime, &read, assertion, documents, Some(scope))?;
    named(&mut value, &read.graph.ontology, EXPLANATION_V3_FORMAT)?;
    let Some(fields) = value.as_object_mut() else {
        return Err(Failure::fault("an explanation is not an object"));
    };
    fields.insert("scope".to_owned(), Value::String(scope.name().to_owned()));
    fields.insert("omitted_evidence".to_owned(), Value::from(omitted));
    // The cap measures the document as it is printed, its byte strings already base64.
    super::bytes_as_base64(&mut value);
    capped(&mut value)?;
    Ok(value)
}

/// The document as `ekr` prints it, its length in bytes: pretty JSON and a final newline.
fn printed_length(document: &Value) -> Result<usize, Failure> {
    Ok(serde_json::to_string_pretty(document)
        .map_err(Failure::fault)?
        .len()
        + 1)
}

/// `ekr.explanation/3`'s cap (`kernel.yaml`, `ekr.kernel.ExplanationV3`, step 4): while the
/// printed document is longer than [`EXPLANATION_CAP`], the longest Evidence `text` — the first
/// in link order among equals — keeps its first half, at a character boundary, and its link is
/// marked `truncated`, with `offset` and `record_length`. It stops when the document fits or no
/// text is left to cut. Each cut shortens one text, so it ends.
fn capped(document: &mut Value) -> Result<(), Failure> {
    while printed_length(document)? > EXPLANATION_CAP {
        let Some(links) = document.get_mut("links").and_then(Value::as_array_mut) else {
            return Ok(());
        };
        let mut longest: Option<(usize, usize)> = None;
        for (at, link) in links.iter().enumerate() {
            if link["kind"] != "Evidence" {
                continue;
            }
            if let Some(text) = link.get("text").and_then(Value::as_str) {
                if text.len() > longest.map_or(0, |(_, length)| length) {
                    longest = Some((at, text.len()));
                }
            }
        }
        let Some((at, length)) = longest else {
            return Ok(());
        };
        let Some(fields) = links[at].as_object_mut() else {
            return Err(Failure::fault("a link is not an object"));
        };
        let text = fields
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let mut cut = length / 2;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        let kept = text[..cut].to_owned();
        fields.entry("offset").or_insert(Value::from(0));
        fields.entry("record_length").or_insert(Value::from(length));
        fields.insert("text".to_owned(), Value::String(kept));
        fields.insert("truncated".to_owned(), Value::Bool(true));
    }
    Ok(())
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
/// records `documents` asks for; and the count of Evidence links left out. With `v3`, the
/// rendering `ekr.explanation/3` starts from: only the links the scope keeps, no proposal record
/// (a Proposal link's `record`, a receipt's `proposal`) and no `payload` whose `text` is
/// answered (`kernel.yaml`, `ekr.kernel.ExplanationV3`, steps 1 and 2).
fn rendered(
    runtime: &Runtime,
    read: &VerifiedRead,
    assertion: AssertionId,
    documents: Option<Bounds>,
    v3: Option<Scope>,
) -> Result<(Value, usize), Failure> {
    let explained = read.explain(assertion)?;
    let mut value = serde_json::to_value(&explained).map_err(Failure::fault)?;
    let Some(links) = value.get_mut("links").and_then(Value::as_array_mut) else {
        return Err(Failure::fault("explanation without links"));
    };
    if links.len() != explained.links.len() {
        return Err(Failure::fault("an explanation's links are not its chain's"));
    }
    let keep: Vec<bool> = match v3 {
        Some(Scope::Cited) => {
            let cited_ids: BTreeSet<&str> = links
                .iter()
                .flat_map(|link| match link["kind"].as_str() {
                    Some("Assertion") => link["evidence"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .collect::<Vec<&str>>(),
                    Some("Attachment") => link["evidence_id"].as_str().into_iter().collect(),
                    _ => Vec::new(),
                })
                .collect();
            links
                .iter()
                .map(|link| {
                    link["kind"] != "Evidence"
                        || link["id"].as_str().is_some_and(|id| cited_ids.contains(id))
                })
                .collect()
        }
        Some(Scope::All) | None => vec![true; links.len()],
    };
    let omitted = keep.iter().filter(|kept| !**kept).count();
    let mut position = 0;
    links.retain(|_| {
        let kept = keep[position];
        position += 1;
        kept
    });
    let Some(bounds) = documents else {
        return Ok((value, omitted));
    };
    let trim = v3.is_some();
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
    let kept = explained
        .links
        .iter()
        .zip(&keep)
        .filter_map(|(link, kept)| kept.then_some(link));
    for (link, rendered) in kept.zip(links.iter_mut()) {
        let Some(fields) = rendered.as_object_mut() else {
            return Err(Failure::fault("a link is not an object"));
        };
        match link {
            ExplanationLink::Proposal(_) if trim => {}
            ExplanationLink::Proposal(proposal) => {
                let record = held(read, proposal.transaction_id)?;
                insert(fields, "record", &record.proposal)?;
            }
            ExplanationLink::Commit(commit) => {
                let record = held(read, commit.transaction_id)?;
                receipt(fields, record, trim)?;
            }
            ExplanationLink::Lifecycle(change) => {
                let record = held(read, change.commit.transaction_id)?;
                let Some(commit) = fields.get_mut("commit").and_then(Value::as_object_mut) else {
                    return Err(Failure::fault("a Lifecycle link without its commit"));
                };
                receipt(commit, record, trim)?;
            }
            ExplanationLink::Attachment(attached) => {
                let record = held(read, attached.commit.transaction_id)?;
                let Some(commit) = fields.get_mut("commit").and_then(Value::as_object_mut) else {
                    return Err(Failure::fault("an Attachment link without its commit"));
                };
                receipt(commit, record, trim)?;
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
                let text = std::str::from_utf8(answered).ok();
                if !(trim && text.is_some()) {
                    fields.insert("payload".to_owned(), Value::String(super::base64(answered)));
                }
                if let Some(text) = text {
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
    Ok((value, omitted))
}

/// A commit's receipt into `fields`, as `receipt`; with `trim`, without its `proposal` (the
/// proposal record, which `ekr.explanation/3` does not answer).
fn receipt(
    fields: &mut Map<String, Value>,
    record: &TransactionRecord,
    trim: bool,
) -> Result<(), Failure> {
    let mut receipt = serde_json::to_value(&record.committed).map_err(Failure::fault)?;
    if trim {
        if let Some(receipt) = receipt.as_object_mut() {
            receipt.remove("proposal");
        }
    }
    fields.insert("receipt".to_owned(), receipt);
    Ok(())
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
