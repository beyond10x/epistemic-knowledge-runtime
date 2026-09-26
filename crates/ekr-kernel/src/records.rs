//! Strict retained records. Payload addresses and canonical value addresses are separate domains.
use crate::{GraphTransaction, ValidatorName};
use ekr_core::{
    AgentId, Canonical, ContentHash, Encoder, EventId, GraphRootId, IssueId, RevisionId, Timestamp,
    TransactionId,
};
use ekr_graph::{CanonicalValue, Root};
use ekr_store::StoreError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The durable issue identity, allocated only when recording a pure validation finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedValidationIssue {
    /// Immutable identity of this recorded finding.
    pub id: IssueId,
    /// Transaction refused.
    pub transaction_id: TransactionId,
    /// Actual deterministic check.
    pub validator: ValidatorName,
    /// Machine-readable refusal.
    pub code: String,
    /// Complete retained diagnostic.
    pub message: String,
}

/// Complete retained `ekr.proposal-record/1` or `/2` fields, in normative declaration order.
///
/// The two formats hold the same fields and differ only in how `document_bytes` is written:
/// `/1` as serde's default number array, `/2` as one standard padded base64 string
/// ([`ekr_core::bytes`]). A record keeps the format it was retained in, so re-encoding a `/1`
/// record reproduces its original bytes and its payload address; new records are `/2`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalRecordV1 {
    /// `ekr.proposal-record/2`, or `ekr.proposal-record/1` for a record retained before it.
    pub format: String,
    /// Retained `event_id`.
    pub event_id: EventId,
    /// Retained `submitted_at`.
    pub submitted_at: Timestamp,
    /// Retained `submitter`.
    pub submitter: AgentId,
    /// Retained `document_hash`.
    pub document_hash: ContentHash,
    /// Retained `document_bytes`.
    pub document_bytes: Vec<u8>,
    /// Retained `transaction_id`.
    pub transaction_id: TransactionId,
    /// Retained `operation_count`.
    pub operation_count: u64,
    /// Retained `evidence_hash`.
    pub evidence_hash: ContentHash,
    /// Retained `canonical_transaction_hash`.
    pub canonical_transaction_hash: Option<ContentHash>,
    /// Retained `canonical_operations_hash`.
    pub canonical_operations_hash: Option<ContentHash>,
}
impl ProposalRecordV1 {
    /// The record format new proposals are retained in.
    pub const FORMAT: &'static str = "ekr.proposal-record/2";
    /// The original format, still read and re-encoded exactly.
    pub const FORMAT_V1: &'static str = "ekr.proposal-record/1";
    /// Encodes current record bytes. This is a codec, not admission authority.
    /// # Errors
    /// Refuses a substituted format or an encoding failure.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        self.check_format()?;
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }
    /// Strictly decodes retained bytes without a generic-value round trip.
    /// # Errors
    /// Refuses unknown, duplicate or missing fields and unsupported formats.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        record.check_format()?;
        Ok(record)
    }
    pub(crate) fn check_format(&self) -> Result<(), StoreError> {
        if self.format != Self::FORMAT && self.format != Self::FORMAT_V1 {
            return Err(StoreError::Document("unsupported-record-format".into()));
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct ProposalWrite<'a> {
    format: &'a str,
    event_id: EventId,
    submitted_at: Timestamp,
    submitter: AgentId,
    document_hash: ContentHash,
    document_bytes: ekr_core::bytes::Spell<'a>,
    transaction_id: TransactionId,
    operation_count: u64,
    evidence_hash: ContentHash,
    canonical_transaction_hash: Option<ContentHash>,
    canonical_operations_hash: Option<ContentHash>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalRead {
    format: String,
    event_id: EventId,
    submitted_at: Timestamp,
    submitter: AgentId,
    document_hash: ContentHash,
    document_bytes: ekr_core::bytes::Spelled,
    transaction_id: TransactionId,
    operation_count: u64,
    evidence_hash: ContentHash,
    canonical_transaction_hash: Option<ContentHash>,
    canonical_operations_hash: Option<ContentHash>,
}
impl Serialize for ProposalRecordV1 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ProposalWrite {
            format: &self.format,
            event_id: self.event_id,
            submitted_at: self.submitted_at,
            submitter: self.submitter,
            document_hash: self.document_hash,
            document_bytes: ekr_core::bytes::Spell {
                bytes: &self.document_bytes,
                numbers: self.format == Self::FORMAT_V1,
            },
            transaction_id: self.transaction_id,
            operation_count: self.operation_count,
            evidence_hash: self.evidence_hash,
            canonical_transaction_hash: self.canonical_transaction_hash,
            canonical_operations_hash: self.canonical_operations_hash,
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ProposalRecordV1 {
    /// Each format admits only its own spelling of `document_bytes`: a number array in `/1`, a
    /// base64 string in `/2`. A record of any other format is read and refused by its codec.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let read = ProposalRead::deserialize(deserializer)?;
        let numbers = read.format == Self::FORMAT_V1;
        if read.document_bytes.numbers != numbers {
            return Err(serde::de::Error::custom(
                "document_bytes is not spelled as its record format writes it",
            ));
        }
        Ok(Self {
            format: read.format,
            event_id: read.event_id,
            submitted_at: read.submitted_at,
            submitter: read.submitter,
            document_hash: read.document_hash,
            document_bytes: read.document_bytes.bytes,
            transaction_id: read.transaction_id,
            operation_count: read.operation_count,
            evidence_hash: read.evidence_hash,
            canonical_transaction_hash: read.canonical_transaction_hash,
            canonical_operations_hash: read.canonical_operations_hash,
        })
    }
}

/// Complete retained ekr.validation-basis/1 fields, in normative declaration order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationBasisV1 {
    /// Exactly `ekr.validation-basis/1`.
    pub format: String,
    /// Retained `graph_root_id`.
    pub graph_root_id: GraphRootId,
    /// Retained `previous_revision_id`.
    pub previous_revision_id: RevisionId,
    /// Retained `previous_event_id`.
    pub previous_event_id: EventId,
    /// Retained `previous_record_hash`.
    pub previous_record_hash: ContentHash,
    /// Retained `previous_root`.
    pub previous_root: Root,
    /// Retained `previous_root_hash`.
    pub previous_root_hash: ContentHash,
    /// Retained `seed_hash`.
    pub seed_hash: ContentHash,
    /// Retained `ontology_root`.
    pub ontology_root: ContentHash,
    /// Retained `authority_root`.
    pub authority_root: ContentHash,
    /// Retained `validation_profile_hash`.
    pub validation_profile_hash: ContentHash,
}
impl ValidationBasisV1 {
    /// Exact supported record format.
    pub const FORMAT: &'static str = "ekr.validation-basis/1";
    /// Encodes current record bytes. This is a codec, not admission authority.
    /// # Errors
    /// Refuses a substituted format or an encoding failure.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        self.check_format()?;
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }
    /// Strictly decodes retained bytes without a generic-value round trip.
    /// # Errors
    /// Refuses unknown, duplicate or missing fields and unsupported formats.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        record.check_format()?;
        Ok(record)
    }
    pub(crate) fn check_format(&self) -> Result<(), StoreError> {
        if self.format != Self::FORMAT {
            return Err(StoreError::Document("unsupported-record-format".into()));
        }
        Ok(())
    }
}
impl Canonical for ValidationBasisV1 {
    fn encode(&self, out: &mut Encoder) {
        self.format.encode(out);
        self.graph_root_id.encode(out);
        self.previous_revision_id.encode(out);
        self.previous_event_id.encode(out);
        self.previous_record_hash.encode(out);
        self.previous_root.encode(out);
        self.previous_root_hash.encode(out);
        self.seed_hash.encode(out);
        self.ontology_root.encode(out);
        self.authority_root.encode(out);
        self.validation_profile_hash.encode(out);
    }
}

/// Complete retained ekr.validation-receipt/1 fields, in normative declaration order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReceiptV1 {
    /// Exactly `ekr.validation-receipt/1`.
    pub format: String,
    /// Retained `event_id`.
    pub event_id: EventId,
    /// Retained `proposed_event_id`.
    pub proposed_event_id: EventId,
    /// Retained `proposal_record_hash`.
    pub proposal_record_hash: ContentHash,
    /// Retained `transaction_hash`.
    pub transaction_hash: ContentHash,
    /// Retained `operations_hash`.
    pub operations_hash: ContentHash,
    /// Retained `evidence_hash`.
    pub evidence_hash: ContentHash,
    /// Retained `operation_count`.
    pub operation_count: u64,
    /// Retained `basis`.
    pub basis: ValidationBasisV1,
    /// Retained `validators`.
    #[serde(deserialize_with = "ekr_core::decode::unique_set")]
    pub validators: BTreeSet<AgentId>,
    /// Retained `validated_at`.
    pub validated_at: Timestamp,
    /// Retained `validation_hash`.
    pub validation_hash: ContentHash,
}
impl ValidationReceiptV1 {
    /// Exact supported record format.
    pub const FORMAT: &'static str = "ekr.validation-receipt/1";
    /// Encodes current record bytes. This is a codec, not admission authority.
    /// # Errors
    /// Refuses a substituted format or an encoding failure.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        self.check_format()?;
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }
    /// Strictly decodes retained bytes without a generic-value round trip.
    /// # Errors
    /// Refuses unknown, duplicate or missing fields and unsupported formats.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        record.check_format()?;
        Ok(record)
    }
    pub(crate) fn check_format(&self) -> Result<(), StoreError> {
        if self.format != Self::FORMAT {
            return Err(StoreError::Document("unsupported-record-format".into()));
        }
        self.basis.check_format()?;
        Ok(())
    }
}

/// Complete retained `ekr.commit-receipt/1` or `/2` fields, in normative declaration order.
///
/// The fields are the same in both. A `/1` receipt embeds an `ekr.proposal-record/1`; a `/2`
/// receipt embeds the transaction's retained proposal in whichever of the two formats it was
/// retained in, which is what lets it carry the compact `/2` proposal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitReceiptV1 {
    /// `ekr.commit-receipt/2`, or `ekr.commit-receipt/1` for a receipt retained before it.
    pub format: String,
    /// Retained `event_id`.
    pub event_id: EventId,
    /// Retained `revision_id`.
    pub revision_id: RevisionId,
    /// Retained `proposal`.
    pub proposal: ProposalRecordV1,
    /// Retained `validation`.
    pub validation: ValidationReceiptV1,
    /// Retained `validation_record_hash`.
    pub validation_record_hash: ContentHash,
    /// Retained `committer`.
    pub committer: AgentId,
    /// Retained `committed_at`.
    pub committed_at: Timestamp,
    /// Retained `result`.
    pub result: Root,
    /// Retained `result_hash`.
    pub result_hash: ContentHash,
}
impl CommitReceiptV1 {
    /// The record format new commit receipts are retained in.
    pub const FORMAT: &'static str = "ekr.commit-receipt/2";
    /// The original format, still read; it embeds only an `ekr.proposal-record/1`.
    pub const FORMAT_V1: &'static str = "ekr.commit-receipt/1";
    /// Encodes current record bytes. This is a codec, not admission authority.
    /// # Errors
    /// Refuses a substituted format or an encoding failure.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        self.check_format()?;
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }
    /// Strictly decodes retained bytes without a generic-value round trip.
    /// # Errors
    /// Refuses unknown, duplicate or missing fields and unsupported formats.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        record.check_format()?;
        Ok(record)
    }
    pub(crate) fn check_format(&self) -> Result<(), StoreError> {
        let embeds = match self.format.as_str() {
            Self::FORMAT => true,
            Self::FORMAT_V1 => self.proposal.format == ProposalRecordV1::FORMAT_V1,
            _ => false,
        };
        if !embeds {
            return Err(StoreError::Document("unsupported-record-format".into()));
        }
        self.proposal.check_format()?;
        self.validation.check_format()?;
        Ok(())
    }
}

/// Complete retained ekr.seed-result/1 fields, in normative declaration order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedResultV1 {
    /// Exactly `ekr.seed-result/1`.
    pub format: String,
    /// Retained `event_id`.
    pub event_id: EventId,
    /// Retained `revision_id`.
    pub revision_id: RevisionId,
    /// Retained `seed_hash`.
    pub seed_hash: ContentHash,
    /// Retained `authority_root`.
    pub authority_root: ContentHash,
    /// Retained `committed_at`.
    pub committed_at: Timestamp,
    /// Retained `result`.
    pub result: Root,
    /// Retained `result_hash`.
    pub result_hash: ContentHash,
}
impl SeedResultV1 {
    /// Exact supported record format.
    pub const FORMAT: &'static str = "ekr.seed-result/1";
    /// Encodes current record bytes. This is a codec, not admission authority.
    /// # Errors
    /// Refuses a substituted format or an encoding failure.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        self.check_format()?;
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }
    /// Strictly decodes retained bytes without a generic-value round trip.
    /// # Errors
    /// Refuses unknown, duplicate or missing fields and unsupported formats.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        record.check_format()?;
        Ok(record)
    }
    pub(crate) fn check_format(&self) -> Result<(), StoreError> {
        if self.format != Self::FORMAT {
            return Err(StoreError::Document("unsupported-record-format".into()));
        }
        Ok(())
    }
}

/// Complete retained ekr.rejection-record/1 fields, in normative declaration order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectionRecordV1 {
    /// Exactly `ekr.rejection-record/1`.
    pub format: String,
    /// Retained `event_id`.
    pub event_id: EventId,
    /// Retained `proposed_event_id`.
    pub proposed_event_id: EventId,
    /// Retained `proposal_record_hash`.
    pub proposal_record_hash: ContentHash,
    /// Retained `requested_basis`.
    pub requested_basis: ValidationBasisV1,
    /// Retained `validator`.
    pub validator: AgentId,
    /// Retained `rejected_at`.
    pub rejected_at: Timestamp,
    /// Retained `issues`.
    pub issues: Vec<RecordedValidationIssue>,
}
impl RejectionRecordV1 {
    /// Exact supported record format.
    pub const FORMAT: &'static str = "ekr.rejection-record/1";
    /// Encodes current record bytes. This is a codec, not admission authority.
    /// # Errors
    /// Refuses a substituted format or an encoding failure.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        self.check_format()?;
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }
    /// Strictly decodes retained bytes without a generic-value round trip.
    /// # Errors
    /// Refuses unknown, duplicate or missing fields and unsupported formats.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        record.check_format()?;
        Ok(record)
    }
    pub(crate) fn check_format(&self) -> Result<(), StoreError> {
        if self.format != Self::FORMAT {
            return Err(StoreError::Document("unsupported-record-format".into()));
        }
        self.requested_basis.check_format()?;
        Ok(())
    }
}

/// Complete retained ekr.stale-record/1 fields, in normative declaration order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaleRecordV1 {
    /// Exactly `ekr.stale-record/1`.
    pub format: String,
    /// Retained `event_id`.
    pub event_id: EventId,
    /// Retained `validation_record_hash`.
    pub validation_record_hash: ContentHash,
    /// Retained `expected_basis`.
    pub expected_basis: ValidationBasisV1,
    /// Retained `observed_revision_id`.
    pub observed_revision_id: RevisionId,
    /// Retained `observed_event_id`.
    pub observed_event_id: EventId,
    /// Retained `observed_record_hash`.
    pub observed_record_hash: ContentHash,
    /// Retained `observed_root`.
    pub observed_root: Root,
    /// Retained `observed_root_hash`.
    pub observed_root_hash: ContentHash,
    /// Retained `stale_at`.
    pub stale_at: Timestamp,
}
impl StaleRecordV1 {
    /// Exact supported record format.
    pub const FORMAT: &'static str = "ekr.stale-record/1";
    /// Encodes current record bytes. This is a codec, not admission authority.
    /// # Errors
    /// Refuses a substituted format or an encoding failure.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        self.check_format()?;
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }
    /// Strictly decodes retained bytes without a generic-value round trip.
    /// # Errors
    /// Refuses unknown, duplicate or missing fields and unsupported formats.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let record: Self = serde_json::from_slice(bytes)
            .map_err(|error| StoreError::Document(error.to_string()))?;
        record.check_format()?;
        Ok(record)
    }
    pub(crate) fn check_format(&self) -> Result<(), StoreError> {
        if self.format != Self::FORMAT {
            return Err(StoreError::Document("unsupported-record-format".into()));
        }
        self.expected_basis.check_format()?;
        Ok(())
    }
}

/// Value-domain validation material, independent of receipt occurrence and recording time.
pub struct ValidationMaterialV1<'a> {
    /// Full accepted canonical transaction.
    pub transaction: &'a GraphTransaction<CanonicalValue>,
    /// Complete historical validation basis.
    pub basis: &'a ValidationBasisV1,
    /// Actual validators under the retained profile.
    pub validators: &'a BTreeSet<AgentId>,
}
impl Canonical for ValidationMaterialV1<'_> {
    fn encode(&self, out: &mut Encoder) {
        "ekr.validation-material/1".encode(out);
        self.transaction.encode(out);
        self.basis.encode(out);
        self.validators.encode(out);
    }
}
