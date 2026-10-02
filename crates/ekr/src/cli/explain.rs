//! `ekr explain`: the kernel's explanation chain over one verified read.

use ekr_core::{AssertionId, TransactionId};
use ekr_kernel::{ExplanationLink, Runtime, TransactionRecord, VerifiedRead};
use serde_json::{Map, Value};

use crate::exit::Failure;

/// Captures the newest revision once and explains the assertion from that capture alone, in
/// `ekr.explanation/2`: proposals, commit receipts and evidence by reference.
///
/// With `documents`, each link also carries the whole retained record it references, read from
/// the same capture: a Proposal link its proposal record as `record`, every commit (a Commit link,
/// a Lifecycle link's `commit`) its commit receipt as `receipt`, and an Evidence link its
/// retained payload, read through `Runtime::content` at the link's own `content_hash`: `payload`
/// in base64 and, when the bytes are UTF-8, `text`.
pub(super) fn run(
    runtime: &Runtime,
    assertion: AssertionId,
    documents: bool,
) -> Result<Value, Failure> {
    let read = runtime.read(None)?;
    let explained = read.explain(assertion)?;
    let mut value = serde_json::to_value(&explained).map_err(Failure::fault)?;
    if !documents {
        return Ok(value);
    }
    let Some(links) = value.get_mut("links").and_then(Value::as_array_mut) else {
        return Err(Failure::fault("explanation without links"));
    };
    for (link, rendered) in explained.links.iter().zip(links.iter_mut()) {
        let Some(fields) = rendered.as_object_mut() else {
            return Err(Failure::fault("a link is not an object"));
        };
        match link {
            ExplanationLink::Proposal(proposal) => {
                let record = held(&read, proposal.transaction_id)?;
                insert(fields, "record", &record.proposal)?;
            }
            ExplanationLink::Commit(commit) => {
                let record = held(&read, commit.transaction_id)?;
                insert(fields, "receipt", &record.committed)?;
            }
            ExplanationLink::Lifecycle(change) => {
                let record = held(&read, change.commit.transaction_id)?;
                let Some(commit) = fields.get_mut("commit").and_then(Value::as_object_mut) else {
                    return Err(Failure::fault("a Lifecycle link without its commit"));
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
                fields.insert("payload".to_owned(), Value::String(super::base64(&bytes)));
                if let Ok(text) = String::from_utf8(bytes) {
                    fields.insert("text".to_owned(), Value::String(text));
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
