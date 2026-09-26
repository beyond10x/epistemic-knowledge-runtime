//! Observations, evidence and support: design § 15–16, amendment 86, and the `ekr.graph` entities
//! of the same names.
//!
//! An observation says *these bytes were received from this source at this time*. Evidence says
//! *this claim rests on that*. Neither says the claim is true; that is what validation is for.

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, AssertionId, ContentHash, EvidenceId, ObservationId, SupportId, Timestamp,
};
use serde::{Deserialize, Serialize};

use crate::assertion::Assertion;
use crate::canonical::CanonicalRef;

/// The form of an evidence source: `ekr.graph.EvidenceKind`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// A URL.
    Url,
    /// A document, optionally a section of one.
    Document,
    /// A record in an external database.
    DatabaseRecord,
    /// Another assertion in this graph.
    GraphAssertion,
    /// An observation the runtime captured.
    Observation,
    /// A statement by a person.
    HumanStatement,
}

/// Where a piece of evidence came from: design § 16.
///
/// `graph.yaml` flattens this to `kind` plus a `locator` and an optional `section` on
/// `ekr.graph.Evidence`, because `ess/1` has no sum type with per-variant payloads. The crate
/// holds the sum type — a `DatabaseRecord` with no table is not an evidence source — and projects
/// the flat fields back out through [`kind`](EvidenceSource::kind),
/// [`locator`](EvidenceSource::locator), [`section`](EvidenceSource::section) and
/// [`observation`](EvidenceSource::observation).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub enum EvidenceSource {
    /// A URL.
    #[cfg_attr(feature = "schema", schemars(rename = "!Url"))]
    Url(String),
    /// A document, optionally a section of one.
    #[cfg_attr(feature = "schema", schemars(rename = "!Document"))]
    Document {
        /// The document's identifier in its own system.
        document_id: String,
        /// The section within it, if the evidence is narrower than the whole.
        section: Option<String>,
    },
    /// A record in an external database.
    #[cfg_attr(feature = "schema", schemars(rename = "!DatabaseRecord"))]
    DatabaseRecord {
        /// The database.
        database: String,
        /// The table.
        table: String,
        /// The record's key.
        key: String,
    },
    /// Another assertion in this graph: a [`CanonicalRef`], because retained evidence is canonical
    /// state and P1 has no transient evidence.
    /// `tests/adversary_p1_14_exit_compile_fail/retained_evidence_names_its_source_assertion_by_canonical_reference.rs`
    /// holds it.
    #[cfg_attr(feature = "schema", schemars(rename = "!GraphAssertion"))]
    GraphAssertion(CanonicalRef<Assertion>),
    /// An observation the runtime captured.
    #[cfg_attr(feature = "schema", schemars(rename = "!Observation"))]
    Observation(ObservationId),
    /// A statement by a person, identified where policy allows it.
    #[cfg_attr(feature = "schema", schemars(rename = "!HumanStatement"))]
    HumanStatement {
        /// Who said it, where the source's authorisation permits recording that.
        identity: Option<String>,
    },
}

impl Canonical for EvidenceSource {
    /// The variant marker, then the variant's fields in declaration order.
    ///
    /// A sum type, so it carries a tag: rule 5 of `ekr_core::canonical`. The collision it prevents
    /// is in front of us — `GraphAssertion(AssertionId)` and `Observation(ObservationId)` are two
    /// id newtypes, which rule 5 makes encode identically, and `Url(String)` would meet a
    /// `Document` carrying no section.
    ///
    /// The index is a literal and it is the contract: changing a number moves every content
    /// address that contains that variant, and reordering the declaration moves nothing.
    /// `crates/ekr-graph/tests/canonical_value_and_assertion.rs` holds the two in step.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Url(url) => {
                out.variant(0);
                url.encode(out);
            }
            Self::Document {
                document_id,
                section,
            } => {
                out.variant(1);
                document_id.encode(out);
                out.option(section.as_ref());
            }
            Self::DatabaseRecord {
                database,
                table,
                key,
            } => {
                out.variant(2);
                database.encode(out);
                table.encode(out);
                key.encode(out);
            }
            Self::GraphAssertion(assertion) => {
                out.variant(3);
                assertion.encode(out);
            }
            Self::Observation(observation) => {
                out.variant(4);
                observation.encode(out);
            }
            Self::HumanStatement { identity } => {
                out.variant(5);
                out.option(identity.as_ref());
            }
        }
    }
}

impl EvidenceSource {
    /// The kind, as `ekr.graph.Evidence.kind` carries it.
    #[must_use]
    pub const fn kind(&self) -> EvidenceKind {
        match self {
            Self::Url(_) => EvidenceKind::Url,
            Self::Document { .. } => EvidenceKind::Document,
            Self::DatabaseRecord { .. } => EvidenceKind::DatabaseRecord,
            Self::GraphAssertion(_) => EvidenceKind::GraphAssertion,
            Self::Observation(_) => EvidenceKind::Observation,
            Self::HumanStatement { .. } => EvidenceKind::HumanStatement,
        }
    }

    /// The locator, as `ekr.graph.Evidence.locator` carries it: enough to find the source again.
    ///
    /// A `HumanStatement` with no recorded identity has an empty locator, which is the honest
    /// answer — the statement is still evidence and there is nothing to point at.
    #[must_use]
    pub fn locator(&self) -> String {
        match self {
            Self::Url(url) => url.clone(),
            Self::Document { document_id, .. } => document_id.clone(),
            Self::DatabaseRecord {
                database,
                table,
                key,
            } => format!("{database}/{table}/{key}"),
            Self::GraphAssertion(assertion) => assertion.id().to_string(),
            Self::Observation(id) => id.to_string(),
            Self::HumanStatement { identity } => identity.clone().unwrap_or_default(),
        }
    }

    /// The section, as `ekr.graph.Evidence.section` carries it. Only a document has one.
    #[must_use]
    pub fn section(&self) -> Option<&str> {
        match self {
            Self::Document { section, .. } => section.as_deref(),
            _ => None,
        }
    }

    /// The observation this evidence was extracted from, as
    /// `ekr.graph.Evidence.observation_id` carries it. Only an `Observation` source has one.
    #[must_use]
    pub const fn observation(&self) -> Option<ObservationId> {
        match self {
            Self::Observation(id) => Some(*id),
            _ => None,
        }
    }
}

/// How much weight the extractor puts on a piece of evidence, in basis points.
///
/// `graph.yaml` declares `confidence_bp` an `Integer` with the invariants `>= 0` and `<= 10000`.
/// Basis points rather than a float because design § 57 content-addresses evidence and
/// `crates/ekr-core/src/canonical.rs` rule 4 has no encoding for a float — and because an
/// invariant a suite can compare exactly is an invariant a suite can check.
///
/// The range is enforced by construction rather than by a validator, so a value outside it never
/// reaches one.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Confidence(u16);

impl Confidence {
    /// The upper bound: ten thousand basis points.
    pub const CERTAIN: Self = Self(10_000);

    /// A confidence of `basis_points`, or `None` above ten thousand.
    #[must_use]
    pub const fn from_basis_points(basis_points: u16) -> Option<Self> {
        if basis_points > Self::CERTAIN.0 {
            None
        } else {
            Some(Self(basis_points))
        }
    }

    /// The basis points, as `ekr.graph.Evidence.confidence_bp` carries them.
    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.0
    }
}

impl Canonical for Confidence {
    /// The basis points it wraps, with no discriminant: a newtype is structural, which is rule 5
    /// of `ekr_core::canonical`. Basis points are exactly why this type is not an `f64` — rule 4
    /// admits no float, and evidence is content-addressed.
    fn encode(&self, out: &mut Encoder) {
        self.0.encode(out);
    }
}

impl From<Confidence> for u16 {
    fn from(confidence: Confidence) -> Self {
        confidence.0
    }
}

impl TryFrom<u16> for Confidence {
    type Error = ConfidenceOutOfRange;

    fn try_from(basis_points: u16) -> Result<Self, Self::Error> {
        Self::from_basis_points(basis_points).ok_or(ConfidenceOutOfRange(basis_points))
    }
}

/// A confidence outside the range `graph.yaml` declares.
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not a confidence: expected basis points between 0 and 10000")]
pub struct ConfidenceOutOfRange(u16);

/// A piece of evidence: design § 16, `ekr.graph.Evidence`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Its stable id, so that a claim can cite it and a retention policy can find it.
    pub id: EvidenceId,
    /// Where it came from.
    pub source: EvidenceSource,
    /// The content address of what was read, so that "the source changed" is detectable.
    pub content_hash: ContentHash,
    /// The agent that extracted it.
    pub extracted_by: AgentId,
    /// When the source was observed.
    pub observed_at: Timestamp,
    /// How much weight the extractor puts on it.
    pub confidence: Confidence,
}

impl Canonical for Evidence {
    /// The six fields in declaration order, structural and untagged.
    ///
    /// `Root.evidence_root` is "the retained evidence at this revision" (design § 34), and this is
    /// what it is an address over. `content_hash` is the address of what was *read* and is a field
    /// like any other here: it says the source has not changed, not that this record has not.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.source.encode(out);
        self.content_hash.encode(out);
        self.extracted_by.encode(out);
        self.observed_at.encode(out);
        self.confidence.encode(out);
    }
}

/// The form of an observation's content: `ekr.graph.ObservationKind`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ObservationKind {
    /// A document.
    Document,
    /// A response from an API.
    ApiResponse,
    /// A record read from a database.
    DatabaseRecord,
    /// An item from a feed.
    FeedItem,
    /// A fragment of an external graph.
    GraphFragment,
    /// A batch of messages.
    MessageBatch,
    /// A diff from a repository.
    GitDiff,
    /// Bytes the runtime stores and does not parse (amendment 86).
    Blob,
}

/// What an observation received: design § 15, plus `Blob` from amendment 86.
///
/// # The payloads design § 15 names are not here
///
/// § 15 gives each variant a parsed payload — `Document`, `StructuredValue`, `MessageBatch`,
/// `GitDiff`. None of those types exists: they belong to the source adapters, which are P2, and
/// P1 declares no dependency that could parse into them. What P1 can carry without inventing them
/// is the form plus the content address of the bytes that were received — which is exactly what
/// `graph.yaml` projects as `kind` and `content_hash`, and is enough for the one thing an
/// observation claims: *these bytes came from this source at this time*.
///
/// `Blob` is the exception, because amendment 86 gives its payload in full and every part of it is
/// representable today.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationContent {
    /// A document.
    Document(ContentHash),
    /// A response from an API.
    ApiResponse(ContentHash),
    /// A record read from a database.
    DatabaseRecord(ContentHash),
    /// An item from a feed.
    FeedItem(ContentHash),
    /// A fragment of an external graph.
    GraphFragment(ContentHash),
    /// A batch of messages.
    MessageBatch(ContentHash),
    /// A diff from a repository.
    GitDiff(ContentHash),
    /// Bytes stored and not parsed: amendment 86. Extraction from a blob is an interpretation,
    /// whose evidence is this hash and the extractor's identity.
    Blob {
        /// The content address of the bytes.
        hash: ContentHash,
        /// What the source said they are.
        media_type: String,
        /// How many there are.
        byte_len: u64,
    },
}

impl Canonical for ObservationContent {
    /// The variant marker, then the payload.
    ///
    /// Seven of the eight variants carry one [`ContentHash`] and nothing else, so without the tag
    /// a document and an API response over the same bytes would be one value. That is the exact
    /// collision rule 5 of `ekr_core::canonical` describes, and here it is seven-fold.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Document(hash) => {
                out.variant(0);
                hash.encode(out);
            }
            Self::ApiResponse(hash) => {
                out.variant(1);
                hash.encode(out);
            }
            Self::DatabaseRecord(hash) => {
                out.variant(2);
                hash.encode(out);
            }
            Self::FeedItem(hash) => {
                out.variant(3);
                hash.encode(out);
            }
            Self::GraphFragment(hash) => {
                out.variant(4);
                hash.encode(out);
            }
            Self::MessageBatch(hash) => {
                out.variant(5);
                hash.encode(out);
            }
            Self::GitDiff(hash) => {
                out.variant(6);
                hash.encode(out);
            }
            Self::Blob {
                hash,
                media_type,
                byte_len,
            } => {
                out.variant(7);
                hash.encode(out);
                media_type.encode(out);
                byte_len.encode(out);
            }
        }
    }
}

impl ObservationContent {
    /// The kind, as `ekr.graph.Observation.kind` carries it.
    #[must_use]
    pub const fn kind(&self) -> ObservationKind {
        match self {
            Self::Document(_) => ObservationKind::Document,
            Self::ApiResponse(_) => ObservationKind::ApiResponse,
            Self::DatabaseRecord(_) => ObservationKind::DatabaseRecord,
            Self::FeedItem(_) => ObservationKind::FeedItem,
            Self::GraphFragment(_) => ObservationKind::GraphFragment,
            Self::MessageBatch(_) => ObservationKind::MessageBatch,
            Self::GitDiff(_) => ObservationKind::GitDiff,
            Self::Blob { .. } => ObservationKind::Blob,
        }
    }

    /// The content address of what was received, as `ekr.graph.Observation.content_hash` carries
    /// it.
    #[must_use]
    pub const fn content_hash(&self) -> &ContentHash {
        match self {
            Self::Document(hash)
            | Self::ApiResponse(hash)
            | Self::DatabaseRecord(hash)
            | Self::FeedItem(hash)
            | Self::GraphFragment(hash)
            | Self::MessageBatch(hash)
            | Self::GitDiff(hash)
            | Self::Blob { hash, .. } => hash,
        }
    }
}

/// What the runtime received from the outside world: design § 15, `ekr.graph.Observation`.
///
/// Immutable. `kind` and `content_hash`, which the domain declares as fields, are answered from
/// [`content`](Observation::content) rather than stored beside it: a field that can disagree with
/// the content next to it is a field that eventually does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// Its stable id.
    pub id: ObservationId,
    /// The source it came from, by the name the runtime knows that source as.
    pub source: String,
    /// The identifier the source itself uses, where it has one.
    pub source_native_id: Option<String>,
    /// What was received.
    pub content: ObservationContent,
    /// When.
    pub captured_at: Timestamp,
}

impl Canonical for Observation {
    /// The five fields in declaration order, structural and untagged.
    ///
    /// `kind` and `content_hash` are not written: they are answered *from* `content`, which is
    /// written whole, so encoding them too would put the same bytes in twice and make a derived
    /// field part of an address.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.source.encode(out);
        out.option(self.source_native_id.as_ref());
        self.content.encode(out);
        self.captured_at.encode(out);
    }
}

impl Observation {
    /// The kind, as `ekr.graph.Observation.kind` carries it.
    #[must_use]
    pub const fn kind(&self) -> ObservationKind {
        self.content.kind()
    }

    /// The content address of what was received.
    #[must_use]
    pub const fn content_hash(&self) -> &ContentHash {
        self.content.content_hash()
    }
}

/// One link from an assertion to one piece of evidence: `ekr.graph.Support`.
///
/// Design § 13 writes the relation as `evidence: BTreeSet<EvidenceId>` on the assertion.
/// `graph.yaml` gives the link its own identity so that one piece of support can be addressed,
/// audited and withdrawn without touching the rest.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Support {
    /// The link's stable id.
    pub id: SupportId,
    /// The assertion that rests on the evidence.
    pub assertion_id: AssertionId,
    /// The evidence it rests on.
    pub evidence_id: EvidenceId,
}

impl Canonical for Support {
    /// The three fields in declaration order, structural and untagged.
    ///
    /// All three are id newtypes, which rule 5 of `ekr_core::canonical` makes encode identically —
    /// so what distinguishes a support link from another with the ids permuted is their positions,
    /// which is exactly what rule 5 says a composite value's field structure is for.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.assertion_id.encode(out);
        self.evidence_id.encode(out);
    }
}
