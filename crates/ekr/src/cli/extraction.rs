//! `ekr apply-extraction`: an extraction document, `ekr.extraction-document/1`, applied to the
//! store (`ekr.integrate.ApplyExtraction`, `story:extraction-verb-shares-the-sdk-path`).
//!
//! The document is read by the engine's reader, `ekr_integrate`'s `ExtractionDocument`, against
//! the ontology of the store's head; a document it refuses is refused by its code (exit 2) and
//! nothing is written. A fact it refuses is skipped and reported under `rejected`, and the rest
//! applies (`story:extraction-partial-apply`); `--strict` refuses the document for it instead, as
//! `ekr_integrate::read_extraction` does. Otherwise the SDK's own routine,
//! `ekr_sdk::extraction::apply_with`, applies it over [`InProcess`]: a transport that answers each
//! request through the session's own dispatch, against the runtime this verb opened, with no child
//! process. So the verb writes exactly what a consumer writes running that routine over a child
//! `ekr session`, and writes only through `propose`, `validate` and `commit`.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use ekr_core::Timestamp;
use ekr_sdk::document::ExtractionRefusal;
use ekr_sdk::extraction::{apply_with, ApplyError, ApplyOptions, ExtractionReport};
use ekr_sdk::transport::Transport;

use super::session::InProcess;
use super::Store;
use crate::exit::Failure;

/// Reads the document at `document`, or stdin for `-`, checks it against the store's head and
/// applies it, proposing as the host operator; `strict` refuses it on its first bad fact.
pub(super) fn run(
    document: &Path,
    strict: bool,
    stdin: &mut dyn Read,
    store: Store,
    now: &dyn Fn() -> Timestamp,
) -> Result<ExtractionReport, Failure> {
    let text = read(document, stdin)?;
    let (report, transport) = applied(&text, strict, store, now, |transport| transport)?;
    transport.close();
    report
}

/// The reader's refusal as the verb's: its code, and the rest of what it printed.
fn refused(error: &ekr_integrate::ExtractionError) -> Failure {
    let code = error.refusal().code.code();
    let shown = error.to_string();
    let reason = shown.strip_prefix(&format!("{code}: ")).unwrap_or(&shown);
    Failure::refused(code, reason)
}

/// Checks `text` against the store's head, then applies it over the transport `wrap` makes of
/// the in-process one. The outer error is a refusal or fault before anything was proposed; the
/// inner one a fault part-way, which leaves what was committed until then committed.
fn applied<'a, T: Transport>(
    text: &str,
    strict: bool,
    store: Store,
    now: &'a dyn Fn() -> Timestamp,
    wrap: impl FnOnce(InProcess<'a>) -> T,
) -> Result<(Result<ExtractionReport, Failure>, T), Failure> {
    let operator = store.host.context.operator;
    let runtime = store.open()?;
    let mut options = ApplyOptions::default();
    {
        let read = runtime.read(None)?;
        let ontology = &read.graph.ontology;
        if strict {
            ekr_integrate::read_extraction(text, ontology).map_err(|error| refused(&error))?;
            options = options.strict();
        } else {
            let document = ekr_integrate::ExtractionDocument::from_yaml(text)
                .map_err(|error| refused(&error))?;
            let facts = document
                .check_facts(ontology)
                .map_err(|refusal| refused(&ekr_integrate::ExtractionError::Refused(refusal)))?;
            let facts: BTreeMap<usize, ExtractionRefusal> = facts
                .into_iter()
                .map(|(at, refusal)| {
                    let refusal = ExtractionRefusal {
                        code: refusal.code.code().to_owned(),
                        name: refusal.name,
                    };
                    (at, refusal)
                })
                .collect();
            options = options.refusing(facts);
        }
    }
    let mirrored = if strict {
        ekr_sdk::document::ExtractionDocument::from_yaml(text)
    } else {
        ekr_sdk::document::ExtractionDocument::decode(text)
    }
    .map_err(|error| Failure::fault(format!("extraction document: {error}")))?;
    let mut transport = wrap(InProcess::new(store, runtime, now));
    let report =
        apply_with(&mut transport, &mirrored, operator, &options).map_err(|error| match error {
            // The reader refuses every such document first; this keeps the reader's code if not.
            ApplyError::Refused(refusal) => ekr_integrate::ExtractionRefusalCode::ALL
                .into_iter()
                .find(|code| code.code() == refusal.code)
                .map_or_else(
                    || Failure::fault(format!("applying the extraction document: {refusal}")),
                    |code| Failure::refused(code.code(), &refusal.name),
                ),
            error => Failure::fault(format!("applying the extraction document: {error}")),
        });
    Ok((report, transport))
}

/// The document's text: at most one byte past the reader's limit is read, so a larger one is
/// refused by the reader as `extraction-document-too-large` without being held whole.
fn read(document: &Path, stdin: &mut dyn Read) -> Result<String, Failure> {
    let limit = u64::try_from(ekr_integrate::EXTRACTION_INPUT_BYTES).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    super::input::open(document, stdin)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Failure::fault(format!("reading {}: {error}", document.display())))?;
    String::from_utf8(bytes).map_err(|error| {
        Failure::refused(
            ekr_integrate::ExtractionRefusalCode::ExtractionDocumentMalformed.code(),
            format!("not an ekr.extraction-document/1 document: not UTF-8: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use ekr_sdk::transport::RecordingTransport;

    use super::super::session::fixture::{seeded, BACKENDS};
    use super::super::{Access, Cli};
    use super::*;
    use clap::Parser as _;

    /// The example seed's own types; Carol and Globex are new, both facts cite item 0403.
    const DOCUMENT: &str = "format: ekr.extraction-document/1
entities:
- node_type: Person
  aliases: [Carol]
facts:
- !Property
  subject: {node_type: Organization, aliases: [Globex]}
  property: legal_name
  value: {value_kind: String, value: Globex Corporation}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Relation
  subject: {node_type: Person, aliases: [Carol]}
  relation: CEO_OF
  object: {node_type: Organization, aliases: [Globex]}
  evidence: [00000000-0000-4000-8000-000000000403]
evidence:
- evidence:
    id: 00000000-0000-4000-8000-000000000403
    source: !HumanStatement
      identity: Quarterly report
    content_hash: 7afeb9c852d895a2cdf8d7717a49354badd8b8dc79049925137b1f95f4651af5
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773273600000
    confidence: 10000
  payload: [67, 97, 114, 111, 108, 44, 32, 67, 69, 79, 32, 111, 102, 32, 71, 108, 111, 98, 101, 120, 32, 67, 111, 114, 112, 111, 114, 97, 116, 105, 111, 110, 44, 32, 108, 101, 97, 100, 115, 32, 112, 114, 111, 106, 101, 99, 116, 32, 65, 112, 111, 108, 108, 111, 46]
";

    /// Acceptance 2: every request the verb sends that writes is a `propose`, `validate` or
    /// `commit` — read off each request's own verb, as the session parses it — and those three
    /// are all it writes with.
    #[test]
    fn the_verbs_write_requests_are_only_propose_validate_and_commit() {
        for backend in BACKENDS {
            let directory = tempfile::tempdir().unwrap();
            let store = Store {
                access: Access::Write,
                ..seeded(directory.path(), backend, "store")
            };
            let clock = || Timestamp::from_millis(1_790_000_000_000);
            let (report, recording) =
                applied(DOCUMENT, false, store, &clock, RecordingTransport::record).unwrap();
            let report = report.unwrap();
            assert!(report.rejected.is_empty(), "{report:?}");
            assert_eq!(report.committed.len(), 2, "{report:?}");
            let mut writes = std::collections::BTreeSet::new();
            let mut verbs = std::collections::BTreeSet::new();
            for exchange in &recording.recording().exchanges {
                let argv = &exchange.request.argv;
                let cli = Cli::try_parse_from(
                    std::iter::once("ekr").chain(argv.iter().map(String::as_str)),
                )
                .unwrap_or_else(|error| panic!("{argv:?}: {error}"));
                verbs.insert(argv[0].clone());
                if cli.command.access() == Access::Write {
                    writes.insert(argv[0].clone());
                }
                assert_eq!(exchange.reply.exit, 0, "{argv:?}: {:?}", exchange.reply);
            }
            assert_eq!(
                writes,
                ["commit", "propose", "validate"].map(String::from).into(),
                "{backend:?}"
            );
            assert!(
                verbs.is_subset(
                    &["commit", "head", "ontology", "propose", "resolve", "snapshot", "validate"]
                        .map(String::from)
                        .into()
                ),
                "{backend:?}: {verbs:?}"
            );
        }
    }
}
