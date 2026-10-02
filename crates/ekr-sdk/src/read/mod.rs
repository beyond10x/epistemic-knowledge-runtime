//! Typed reads over `ekr session` and one-shot reads (`story:sdk-read-helpers`).
//!
//! A [`Reader`] sends a read's argv over any [`Transport`] — a
//! [`ProcessSession`](crate::session::ProcessSession), a recording or a replay — and reads the
//! document it answers into a typed value:
//!
//! | method | verb | value |
//! |---|---|---|
//! | [`Reader::overview`] | `overview` | [`Overview`], `ekr.graph-overview/1` |
//! | [`Reader::search`] | `search` | [`NodeMatches`], `ekr.node-matches/1` |
//! | [`Reader::describe`] | `describe` | [`NodeDetail`], `ekr.node-detail/1` |
//! | [`Reader::expand`], [`Reader::expand_page`] | `expand` | [`Slice`] pages, `ekr.graph-slice/1` |
//! | [`Reader::timeline`] | `timeline` | [`Timeline`], `ekr.graph-timeline/1` |
//! | [`Reader::changes`] | `changes` | [`Changes`], `ekr.graph-changes/1` |
//! | [`Reader::head`] | `head` | [`Head`] |
//! | [`Reader::snapshot`] | `snapshot` | [`Snapshot`] |
//! | [`Reader::ontology`] | `ontology` | [`Ontology`] |
//! | [`Reader::transactions`] | `transactions` | [`Transactions`] |
//! | [`Reader::explain`], [`Reader::explain_documents`] | `explain` | [`Explanation`], `ekr.explanation/2` |
//! | [`Reader::quality`] | `quality` | [`StoreQuality`], `ekr.store-quality/1` |
//! | [`Reader::rejections`] | `rejections` | [`Rejections`], `ekr.rejections/1` |
//! | [`Reader::code_names`] | `code-names` | [`CodeNames`], `ekr.code-names/1` |
//!
//! The first six are the `ekr.views` reads, which `ekr` serves in a session only. The last eight
//! are one-shot verbs too, and [`OneShotReader`] runs each as its own `ekr` process, with no
//! session open. Every call is blocking and reads the store as it stands when `ekr` reads the
//! request: a commit another process made is what the next call reads.

mod checks;
mod kernel;
mod ocel;
mod one_shot;
mod views;

use ekr_core::{AssertionId, NodeId, Timestamp, TypeId};
use serde::de::DeserializeOwned;

use crate::binary::EkrBinary;
use crate::reply::{Answer, Fault, Refusal};
use crate::session::{SessionOptions, StoreConfig};
use crate::transport::{Request, Transport, TransportError};

pub use checks::{
    AssertionQuality, CodeNameFinding, CodeNameKind, CodeNameMatch, CodeNameMode, CodeNames,
    CodeNamesMeta, PropertyQuality, QualityMeta, RejectedTransaction, RejectionIssue, Rejections,
    SharedName, StoreQuality,
};
pub use kernel::{
    ExplainedAttachment, ExplainedEvidence, Explanation, ExplanationLink, Head, ListedTransaction,
    NamedType, Ontology, OntologyCardinality, OntologyEdgeType, OntologyNodeType, OntologyProperty,
    OntologyValueType, RecordedTime, Root, Snapshot, SnapshotAssertion, SnapshotAttachment,
    SnapshotEdge, SnapshotEvidence, SnapshotGraph, SnapshotGraphDocument, SnapshotNode,
    SnapshotPredicate, SnapshotRoot, SnapshotSubject, TransactionState, Transactions, ValidTime,
};
pub use ocel::{
    OcelCounts, OcelDocument, OcelEvent, OcelEventAttribute, OcelExport, OcelLog, OcelMeta,
    OcelName, OcelNames, OcelObject, OcelObjectAttribute, OcelQuery, OcelRelationship, OcelType,
    OcelTypeAttribute,
};
pub use views::{
    ChangeKind, Changes, ChangesMeta, DetailMeta, DetailNode, GraphChange, MatchField, MatchTier,
    MatchesMeta, ModifiedProperty, NodeDetail, NodeMatch, NodeMatches, NodeSummary, Overview,
    OverviewMeta, OverviewRevision, OverviewRoles, OverviewSchema, OverviewTimeline,
    ReferencingAssertion, SchemaMember, SchemaVersionChange, Slice, SliceEdge, SliceMeta,
    SliceNode, Timeline, TimelineBucket, TimelineCell, TimelineEvent, TimelineMeta, TimelineRow,
    TimelineRowType, TimelineStep, TypeCount, TypeTiming, ViewAssertion, ViewAssessment, ViewEdge,
    ViewEdgeType, ViewLifecycle, ViewNodeType, ViewOntology, ViewProperty, ViewValue, WidenedEnd,
};

/// Reads an enum whose unit `Other` is its catch-all. `derived` is the reader serde derives for
/// the enum's private mirror (`#[serde(remote = …)]`), which reads a kind it does not know as
/// `Other` only when that kind carries no content. So a document it refuses is read once more
/// with its content removed — the `content` key of an adjacently tagged enum, or, when `content`
/// is `None`, the value of an externally tagged one, `{"Kind": …}` read as `"Kind"` — and answers
/// only if that reads as `Other`; a known kind whose content is wrong keeps its first error.
fn tolerant<'de, D, T>(
    deserializer: D,
    content: Option<&str>,
    derived: fn(serde_json::Value) -> Result<T, serde_json::Error>,
    is_other: fn(&T) -> bool,
) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    use serde::Deserialize as _;

    let document = serde_json::Value::deserialize(deserializer)?;
    let mut bare = document.clone();
    derived(document).or_else(|error| {
        if let serde_json::Value::Object(fields) = &mut bare {
            match content {
                Some(key) => {
                    fields.remove(key);
                }
                None if fields.len() == 1 => {
                    let kind = fields.keys().next().cloned().unwrap_or_default();
                    bare = serde_json::Value::String(kind);
                }
                None => {}
            }
        }
        match derived(bare) {
            Ok(read) if is_other(&read) => Ok(read),
            _ => Err(D::Error::custom(error)),
        }
    })
}

/// Why a read returned no typed value. Each variant names the verb it read.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// No reply was read.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// A named refusal, such as `ekr.views.NodeNotFound` or `ekr.views.RevisionNotFound`.
    #[error("`{verb}` refused: {}: {}", refusal.code, refusal.reason)]
    Refused {
        /// The verb.
        verb: String,
        /// The refusal.
        refusal: Refusal,
    },
    /// Clap refused the argv: an `ekr` that does not take it, such as one older than the verb.
    #[error("`{verb}`: {message}")]
    Usage {
        /// The verb.
        verb: String,
        /// Clap's usage message.
        message: String,
    },
    /// The read could not be answered: a store that does not open or cannot be read.
    #[error("`{verb}`: {}", fault.message)]
    Fault {
        /// The verb.
        verb: String,
        /// The fault.
        fault: Fault,
    },
    /// The document is not one this SDK reads as the verb's value.
    #[error("`{verb}` answered a document this SDK does not read: {source}")]
    Document {
        /// The verb.
        verb: String,
        /// Why it does not read.
        source: serde_json::Error,
    },
}

/// Exactly one since of [`Reader::changes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Since {
    /// The changes of the revisions after this one.
    Revision(u64),
    /// The assertion changes whose valid time is after this instant.
    Valid(Timestamp),
    /// The changes of the revisions committed after this instant.
    Recorded(Timestamp),
}

/// The finest bucket of a timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bucket {
    /// A day.
    Day,
    /// A week.
    Week,
}

/// A neighbourhood to page: `seeds` (none answers an empty page), `depth` hops (0 to 2), at
/// most `limit` nodes (1 to 2,000) and `edges` edges (1 to 5,000, 5,000 when `None`) a page, at
/// `revision` (the head when `None`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandQuery {
    /// The node ids to start from.
    pub seeds: Vec<NodeId>,
    /// The hops from the seeds.
    pub depth: u64,
    /// The most nodes a page holds.
    pub limit: u64,
    /// The most edges a page holds.
    pub edges: Option<u64>,
    /// The committed revision to read.
    pub revision: Option<u64>,
}

impl ExpandQuery {
    /// `seeds` at `depth`, `limit` nodes a page, the default edge limit, at the head.
    #[must_use]
    pub fn new(seeds: Vec<NodeId>, depth: u64, limit: u64) -> Self {
        Self {
            seeds,
            depth,
            limit,
            edges: None,
            revision: None,
        }
    }
}

/// A timeline: rows of `row_type` (the first ranked when `None`) with their events within `hops`
/// (1 to 3), at most `limit` rows (1 to 500), `bucket` the finest bucket, `subject` one row
/// alone and its events, at `revision` (the head when `None`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineQuery {
    /// The row type.
    pub row_type: Option<TypeId>,
    /// How far an event may be from its subject.
    pub hops: u64,
    /// The most rows.
    pub limit: u64,
    /// The finest bucket.
    pub bucket: Option<Bucket>,
    /// One subject.
    pub subject: Option<NodeId>,
    /// The committed revision to read.
    pub revision: Option<u64>,
}

impl TimelineQuery {
    /// Rows within `hops`, at most `limit`, at the head.
    #[must_use]
    pub fn new(hops: u64, limit: u64) -> Self {
        Self {
            row_type: None,
            hops,
            limit,
            bucket: None,
            subject: None,
            revision: None,
        }
    }
}

/// Typed reads over one [`Transport`], blocking.
#[derive(Debug)]
pub struct Reader<T: Transport> {
    transport: T,
}

/// An argv under construction: a verb and its flags.
struct Argv(Vec<String>);

impl Argv {
    fn new(verb: &str) -> Self {
        Self(vec![verb.to_owned()])
    }

    fn flag(mut self, name: &str, value: Option<impl ToString>) -> Self {
        if let Some(value) = value {
            self.0.push(format!("--{name}"));
            self.0.push(value.to_string());
        }
        self
    }

    fn arg(mut self, value: impl ToString) -> Self {
        self.0.push(value.to_string());
        self
    }
}

impl<T: Transport> Reader<T> {
    /// A reader sending its requests over `transport`.
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// The transport, back.
    pub fn into_inner(self) -> T {
        self.transport
    }

    /// Sends `argv` and reads the exit-0 document as `V`.
    fn read<V: DeserializeOwned>(&mut self, argv: Argv) -> Result<V, ReadError> {
        self.read_request(Request::new(argv.0))
    }

    /// The same typed reply handling for calls carrying stdin.
    pub(crate) fn read_request<V: DeserializeOwned>(
        &mut self,
        request: Request,
    ) -> Result<V, ReadError> {
        let verb = request.verb().to_owned();
        let reply = self.transport.request(&request)?;
        match reply.answer() {
            Answer::Outcome(outcome) => serde_json::from_value(outcome.document().clone())
                .map_err(|source| ReadError::Document { verb, source }),
            Answer::Refusal(refusal) => Err(ReadError::Refused { verb, refusal }),
            Answer::Usage(message) => Err(ReadError::Usage { verb, message }),
            Answer::Fault(fault) => Err(ReadError::Fault { verb, fault }),
        }
    }

    /// `overview`: the revision's counts, schema, types, roles, timeline and the `limit`
    /// highest-degree nodes (1 to 500, 300 when `None`).
    ///
    /// # Errors
    /// [`ReadError`]; `ekr.views.LimitExceeded`, `ekr.views.NotSeeded` and
    /// `ekr.views.RevisionNotFound` are [`ReadError::Refused`].
    pub fn overview(
        &mut self,
        revision: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Overview, ReadError> {
        self.read(
            Argv::new("overview")
                .flag("revision", revision)
                .flag("limit", limit),
        )
    }

    /// `search`: the nodes whose name or an alias contains `text` (empty matches every node),
    /// exact matches first, at most `limit` (1 to 100, 20 when `None`).
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn search(
        &mut self,
        text: &str,
        limit: Option<u64>,
        revision: Option<u64>,
    ) -> Result<NodeMatches, ReadError> {
        self.read(
            Argv::new("search")
                .flag("limit", limit)
                .flag("revision", revision)
                .arg("--")
                .arg(text),
        )
    }

    /// `describe`: one node, every assertion about it or naming it, its edges and neighbours.
    ///
    /// # Errors
    /// [`ReadError`]; a node the revision does not hold is `ekr.views.NodeNotFound`.
    pub fn describe(
        &mut self,
        node: NodeId,
        revision: Option<u64>,
    ) -> Result<NodeDetail, ReadError> {
        self.read(Argv::new("describe").arg(node).flag("revision", revision))
    }

    /// `expand`: the page of `query`'s neighbourhood starting at cursor `after` (0 for the
    /// first page, then the previous page's `next`).
    ///
    /// # Errors
    /// [`ReadError`]; an unknown seed is `ekr.views.NodeNotFound`.
    pub fn expand_page(&mut self, query: &ExpandQuery, after: u64) -> Result<Slice, ReadError> {
        let mut argv = Argv::new("expand");
        for seed in &query.seeds {
            argv = argv.arg(seed);
        }
        self.read(
            argv.flag("depth", Some(query.depth))
                .flag("limit", Some(query.limit))
                .flag("edges", query.edges)
                .flag("after", Some(after))
                .flag("revision", query.revision),
        )
    }

    /// Every page of `query`'s neighbourhood, in order, each read when the iterator is advanced.
    /// Every page after the first reads the revision the first one did, so a commit between two
    /// pages neither drops nor repeats a record. After an error the iterator ends.
    pub fn expand(&mut self, query: ExpandQuery) -> ExpandPages<'_, T> {
        ExpandPages {
            reader: self,
            query,
            after: Some(0),
        }
    }

    /// `timeline`: rows of subjects with their events counted per bucket.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn timeline(&mut self, query: &TimelineQuery) -> Result<Timeline, ReadError> {
        let bucket = query.bucket.map(|bucket| match bucket {
            Bucket::Day => "day",
            Bucket::Week => "week",
        });
        self.read(
            Argv::new("timeline")
                .flag("type", query.row_type)
                .flag("hops", Some(query.hops))
                .flag("limit", Some(query.limit))
                .flag("bucket", bucket)
                .flag("subject", query.subject)
                .flag("revision", query.revision),
        )
    }

    /// `changes`: the page of what the revisions up to `at` (the head when `None`) changed after
    /// `since`, at most `limit` (1 to 2,000, 500 when `None`) from cursor `after`. Pass the first
    /// page's `meta.revision` as `at` for the rest.
    ///
    /// # Errors
    /// [`ReadError`]; a revision since beyond the head is `ekr.views.RevisionNotFound`.
    pub fn changes(
        &mut self,
        since: Since,
        at: Option<u64>,
        limit: Option<u64>,
        after: Option<u64>,
    ) -> Result<Changes, ReadError> {
        let argv = match since {
            Since::Revision(revision) => {
                Argv::new("changes").flag("since-revision", Some(revision))
            }
            Since::Valid(at) => Argv::new("changes").flag("since-valid", Some(at.millis())),
            Since::Recorded(at) => Argv::new("changes").flag("since-recorded", Some(at.millis())),
        };
        self.read(
            argv.flag("at", at)
                .flag("limit", limit)
                .flag("after", after),
        )
    }

    /// `head`: the newest revision and its root.
    ///
    /// # Errors
    /// [`ReadError`]; a store never seeded is a [`ReadError::Fault`].
    pub fn head(&mut self) -> Result<Head, ReadError> {
        self.read(Argv::new("head"))
    }

    /// `snapshot`: the graph at `at` (the head when `None`); with `valid_at`, the assertions
    /// believed at that instant too.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn snapshot(
        &mut self,
        at: Option<u64>,
        valid_at: Option<Timestamp>,
    ) -> Result<Snapshot, ReadError> {
        self.read(
            Argv::new("snapshot")
                .flag("at", at)
                .flag("valid-at", valid_at.map(Timestamp::millis)),
        )
    }

    /// `ontology`: the schema in force at `at` (the head when `None`), by name and id.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn ontology(&mut self, at: Option<u64>) -> Result<Ontology, ReadError> {
        self.read(Argv::new("ontology").flag("at", at))
    }

    /// `transactions`: every retained transaction, or those in `state`.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn transactions(
        &mut self,
        state: Option<TransactionState>,
    ) -> Result<Transactions, ReadError> {
        self.read(Argv::new("transactions").flag("state", state.map(TransactionState::as_str)))
    }

    /// `explain`: the assertion at the head, where it came from, what later changed it, and its
    /// evidence, each record by hash (`ekr.explanation/2`).
    ///
    /// # Errors
    /// [`ReadError`]; an unknown id is `ekr.kernel.AssertionNotFound`.
    pub fn explain(&mut self, assertion: AssertionId) -> Result<Explanation, ReadError> {
        self.read(Argv::new("explain").arg(assertion))
    }

    /// `explain --documents`: [`Reader::explain`] with the whole records its links name — each
    /// proposal record, commit receipt and evidence payload ([`ExplainedEvidence::payload`]).
    ///
    /// # Errors
    /// [`ReadError`]; an unknown id is `ekr.kernel.AssertionNotFound`.
    pub fn explain_documents(&mut self, assertion: AssertionId) -> Result<Explanation, ReadError> {
        self.read(Argv::new("explain").arg(assertion).arg("--documents"))
    }
}

/// The pages of one neighbourhood: [`Reader::expand`].
#[derive(Debug)]
pub struct ExpandPages<'r, T: Transport> {
    reader: &'r mut Reader<T>,
    query: ExpandQuery,
    /// The next page's cursor; `None` once the last page or an error was returned.
    after: Option<u64>,
}

impl<T: Transport> Iterator for ExpandPages<'_, T> {
    type Item = Result<Slice, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        let after = self.after.take()?;
        let page = self.reader.expand_page(&self.query, after);
        if let Ok(slice) = &page {
            self.query.revision = Some(slice.meta.revision);
            self.after = slice.next;
        }
        Some(page)
    }
}

/// `head`, `snapshot`, `ontology`, `transactions`, `explain`, `quality`, `rejections` and
/// `code-names`, each as its own `ekr` process
/// with no session open: `ekr --host … --store … --backend … <verb>`, started as a session's
/// processes are, in the environment and working directory `options` give, stopped after
/// `options.timeout`.
#[derive(Debug)]
pub struct OneShotReader {
    pub(crate) reader: Reader<one_shot::OneShot>,
}

impl OneShotReader {
    /// A reader of `store` through `binary`, one process per read.
    #[must_use]
    pub fn new(binary: &EkrBinary, store: StoreConfig, options: SessionOptions) -> Self {
        Self {
            reader: Reader::new(one_shot::OneShot {
                binary: binary.clone(),
                store,
                options,
            }),
        }
    }

    /// [`Reader::head`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn head(&mut self) -> Result<Head, ReadError> {
        self.reader.head()
    }

    /// [`Reader::snapshot`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn snapshot(
        &mut self,
        at: Option<u64>,
        valid_at: Option<Timestamp>,
    ) -> Result<Snapshot, ReadError> {
        self.reader.snapshot(at, valid_at)
    }

    /// [`Reader::ontology`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn ontology(&mut self, at: Option<u64>) -> Result<Ontology, ReadError> {
        self.reader.ontology(at)
    }

    /// [`Reader::transactions`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn transactions(
        &mut self,
        state: Option<TransactionState>,
    ) -> Result<Transactions, ReadError> {
        self.reader.transactions(state)
    }

    /// [`Reader::explain`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn explain(&mut self, assertion: AssertionId) -> Result<Explanation, ReadError> {
        self.reader.explain(assertion)
    }

    /// [`Reader::explain_documents`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn explain_documents(&mut self, assertion: AssertionId) -> Result<Explanation, ReadError> {
        self.reader.explain_documents(assertion)
    }
}
