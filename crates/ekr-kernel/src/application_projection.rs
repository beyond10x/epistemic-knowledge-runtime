//! Lossless presentation projection between the two generated contract libraries.
//!
//! This is not admission: optional payloads, list order and empty/absent distinctions remain
//! inspectable even when the records would not authorize a new transaction. Numeric range and
//! byte decoding are checked. Every record destructures all source fields and constructs all
//! destination fields, so generated shape changes cannot silently discard application history.
use ekr_core::contract_data as w;
use ekr_core::contracts::{
    graph as g, integrate as m, kernel as k, ontology as o, primitives as p, store as s,
};
use ekr_store::StoreError;
use std::collections::BTreeMap;

pub(super) fn read(
    value: &w::EkrIntegrateApplicationRead,
) -> Result<m::ApplicationRead, StoreError> {
    value.project()
}
pub(super) fn receipt(
    value: &w::EkrIntegrateApplicationReceiptSnapshot,
) -> Result<m::ApplicationReceiptSnapshot, StoreError> {
    value.project()
}

trait Project<T> {
    fn project(&self) -> Result<T, StoreError>;
}
impl<T, U> Project<U> for Box<T>
where
    T: Project<U>,
{
    fn project(&self) -> Result<U, StoreError> {
        self.as_ref().project()
    }
}
impl<T, U> Project<Vec<U>> for Vec<T>
where
    T: Project<U>,
{
    fn project(&self) -> Result<Vec<U>, StoreError> {
        self.iter().map(Project::project).collect()
    }
}
impl<T, U> Project<Option<U>> for w::EssPresence<T>
where
    T: Project<U>,
{
    fn project(&self) -> Result<Option<U>, StoreError> {
        match self {
            Self::Absent => Ok(None),
            Self::Present(value) => value.project().map(Some),
        }
    }
}
impl<T, U> Project<BTreeMap<String, U>> for BTreeMap<String, T>
where
    T: Project<U>,
{
    fn project(&self) -> Result<BTreeMap<String, U>, StoreError> {
        self.iter()
            .map(|(key, value)| Ok((key.clone(), value.project()?)))
            .collect()
    }
}
impl Project<String> for String {
    fn project(&self) -> Result<String, StoreError> {
        Ok(self.clone())
    }
}
impl Project<p::Uuid> for String {
    fn project(&self) -> Result<p::Uuid, StoreError> {
        Ok(p::Uuid(self.clone()))
    }
}
impl Project<bool> for bool {
    fn project(&self) -> Result<bool, StoreError> {
        Ok(*self)
    }
}
impl Project<i64> for serde_json::Number {
    fn project(&self) -> Result<i64, StoreError> {
        self.as_i64().ok_or_else(|| {
            StoreError::Document(
                "application-projection: integer exceeds generated semantic range".into(),
            )
        })
    }
}
impl Project<Vec<u8>> for String {
    fn project(&self) -> Result<Vec<u8>, StoreError> {
        crate::incubation_projection::bytes(self)
    }
}
impl Project<p::Timestamp> for w::EssTimestamp {
    fn project(&self) -> Result<p::Timestamp, StoreError> {
        crate::incubation_document::timestamp_text(self).map(p::Timestamp)
    }
}

macro_rules! wrapper {
    ($source:ident => $module:ident::$target:ident) => {
        impl Project<$module::$target> for w::$source {
            fn project(&self) -> Result<$module::$target, StoreError> {
                Ok($module::$target(self.0.project()?))
            }
        }
    };
}
macro_rules! record {
    ($source:ident => $module:ident::$target:ident { $($field:ident),* $(,)? }) => {
        impl Project<$module::$target> for w::$source {
            fn project(&self) -> Result<$module::$target, StoreError> {
                let w::$source { $($field),* } = self;
                Ok($module::$target { $($field: $field.project()?),* })
            }
        }
    };
}
macro_rules! enumeration {
    ($source:ident => $module:ident::$target:ident { $($from:ident => $to:ident),* $(,)? }) => {
        impl Project<$module::$target> for w::$source {
            fn project(&self) -> Result<$module::$target, StoreError> {
                Ok(match self { $(w::$source::$from => $module::$target::$to),* })
            }
        }
    };
}
macro_rules! dictionary {
    ($source:ident => $target:ty) => {
        impl Project<BTreeMap<String, $target>> for w::$source {
            fn project(&self) -> Result<BTreeMap<String, $target>, StoreError> {
                let w::$source { ess_extra } = self;
                ess_extra.project()
            }
        }
    };
}

wrapper!(EkrIntegrateSchemaApplicationId => m::SchemaApplicationId);
wrapper!(EkrIntegrateSchemaProposalId => m::SchemaProposalId);
wrapper!(EkrIntegrateProposalReviewId => m::ProposalReviewId);
wrapper!(EkrIntegrateApplicationStepId => m::ApplicationStepId);
wrapper!(EkrIntegrateApplicationReceiptId => m::ApplicationReceiptId);
wrapper!(EkrIntegrateProcessingReceiptId => m::ProcessingReceiptId);
wrapper!(EkrIntegrateInterpretationId => m::InterpretationId);
wrapper!(EkrIntegrateMappingRecordId => m::MappingRecordId);
wrapper!(EkrKernelTransactionId => k::TransactionId);
wrapper!(EkrKernelAgentId => k::AgentId);
wrapper!(EkrKernelIssueId => k::IssueId);
wrapper!(EkrKernelEventId => k::EventId);
wrapper!(EkrKernelContentHash => k::ContentHash);
wrapper!(EkrKernelRevisionNumber => k::RevisionNumber);
wrapper!(EkrGraphNodeId => g::NodeId);
wrapper!(EkrGraphEdgeId => g::EdgeId);
wrapper!(EkrGraphGraphRootId => g::GraphRootId);
wrapper!(EkrGraphAssertionId => g::AssertionId);
wrapper!(EkrGraphEvidenceId => g::EvidenceId);
wrapper!(EkrGraphObservationId => g::ObservationId);
wrapper!(EkrOntologySchemaVersionId => o::SchemaVersionId);
wrapper!(EkrOntologyTypeId => o::TypeId);
wrapper!(EkrOntologyPropertyId => o::PropertyId);

record!(EkrIntegrateApplicationRead => m::ApplicationRead { election, steps, attempts, publications, receipts, remaining_items, corrections_pending });
record!(EkrIntegrateRetainedApplicationElection => m::RetainedApplicationElection { application_id, proposal_id, proposal_digest, initial_review_id, initial_proof_digest, base_schema, elected_at, schema_transaction, selected_items });
record!(EkrIntegrateRetainedApplicationStep => m::RetainedApplicationStep { step_election_id, application_id, step, correction_review_id, transaction, replacements, mappings, derivations, elected_at });
record!(EkrIntegrateRetainedApplicationAttempt => m::RetainedApplicationAttempt { transaction_id, step_election_id, predecessor_transaction, predecessor_record_hash, transaction, elected_at });
record!(EkrIntegrateApplicationPublicationRecord => m::ApplicationPublicationRecord { guard, transaction_id, event_id, record_hash, command });
record!(EkrIntegrateApplicationPublicationGuard => m::ApplicationPublicationGuard { application_id, step_election_id, attempt_transaction, proposal_id, proposal_digest, review_id, human_proof_digest, review_stream_version, step });
record!(EkrIntegrateApplicationReceiptSnapshot => m::ApplicationReceiptSnapshot { application_id, receipt_id, proposal_id, review_id, progress, schema_transaction, schema_revision, processing_receipts, remaining_items, corrections_pending, stop_reason });
record!(EkrIntegrateApplicationItemKey => m::ApplicationItemKey { source, item, mapping_digest });
record!(EkrIntegrateApplicationStep => m::ApplicationStep { kind, item });
record!(EkrIntegrateCanonicalDerivationRecord => m::CanonicalDerivationRecord { derivation_id, assertion_id, mapping_id, observation_id, evidence_id });
record!(EkrIntegrateRetainedMappingRecord => m::RetainedMappingRecord { mapping_id, proposal_digest, mapping_digest, source_document_digest, mapping, evidence, payload });
record!(EkrIntegrateKnowledgeMapping => m::KnowledgeMapping { source, source_item, source_type, target_type, target_member, value });
record!(EkrIntegrateInterpretationVersion => m::InterpretationVersion { interpretation_id, version, document_digest });
record!(EkrIntegrateDeclaredSourceField => m::DeclaredSourceField { declaration, field });
record!(EkrIntegrateDeclaredSourceRelation => m::DeclaredSourceRelation { declaration, relation });

record!(EkrKernelCanonicalTransactionProjection => k::CanonicalTransactionProjection { id, proposer, operations, evidence, schema_version });
record!(EkrKernelClaimReplacement => k::ClaimReplacement { previous, replacement });
record!(EkrKernelAliasAdditionProjection => k::AliasAdditionProjection { node, alias });
record!(EkrKernelEdgeDraftProjection => k::EdgeDraftProjection { id, root_id, type_id, source, target, properties });
record!(EkrKernelEdgeWideningProjection => k::EdgeWideningProjection { edge_type, source_types, target_types });
record!(EkrKernelEntityMerge => k::EntityMerge { absorbed, into });
record!(EkrKernelEvidenceAdditionProjection => k::EvidenceAdditionProjection { evidence, payload });
record!(EkrKernelEvidenceAttachmentProjection => k::EvidenceAttachmentProjection { assertion, evidence });
record!(EkrKernelInvocationProjection => k::InvocationProjection { node, operation, arguments });
record!(EkrKernelNodeDraftProjection => k::NodeDraftProjection { id, root_id, type_id, canonical_name, properties, aliases });
record!(EkrKernelPropertyModificationProjection => k::PropertyModificationProjection { owner, property });
record!(EkrKernelPropertyMutationProjection => k::PropertyMutationProjection { node, property, values });
record!(EkrKernelRetraction => k::Retraction { assertion, reason });
record!(EkrKernelSupersession => k::Supersession { assertion, by, effective_from });

record!(EkrGraphAssertionRecord => g::AssertionRecord { id, root_id, subject, predicate, object, evidence, proposed_by, assessment, lifecycle, valid_time, transaction_time });
record!(EkrGraphSubjectProjection => g::SubjectProjection { kind, id });
record!(EkrGraphPredicateProjection => g::PredicateProjection { kind, id });
record!(EkrGraphObjectProjection => g::ObjectProjection { kind, value, reference });
record!(EkrGraphAssessmentProjection => g::AssessmentProjection { kind, completed, required, validators, issues, competing_assertions });
record!(EkrGraphAssertionLifecycleProjection => g::AssertionLifecycleProjection { kind, at_revision, reason, by, effective_from });
record!(EkrGraphTemporalRange => g::TemporalRange { from, to });
record!(EkrGraphTransactionTime => g::TransactionTime { recorded_from, recorded_to });
record!(EkrGraphTypedValue => g::TypedValue { kind, canonical_bytes });
record!(EkrGraphEvidenceRecord => g::EvidenceRecord { id, source, content_hash, extracted_by, observed_at, confidence_bp });
record!(EkrGraphEvidenceSourceProjection => g::EvidenceSourceProjection { kind, url, document_id, section, database, table, key, assertion, observation, identity });

record!(EkrOntologyNodeTypeDeclaration => o::NodeTypeDeclaration { id, name, parents, properties, abstract_type, lifecycle, operations });
record!(EkrOntologyEdgeTypeDeclaration => o::EdgeTypeDeclaration { id, name, source_types, target_types, cardinality, properties, inverse, symmetric, transitive });
record!(EkrOntologyPropertyDeclaration => o::PropertyDeclaration { id, name, value_type, cardinality, required, constraints });
record!(EkrOntologyValueTypeProjection => o::ValueTypeProjection { kind, allowed_types, variants, element, fields });
record!(EkrOntologyLifecycleDeclaration => o::LifecycleDeclaration { initial, states, transitions });
record!(EkrOntologyOperationDefinition => o::OperationDefinition { name, arguments, preconditions, transition, emits });
record!(EkrOntologyTransition => o::Transition { from, to });
impl Project<Box<o::ValueTypeProjection>> for w::EkrOntologyValueTypeProjection {
    fn project(&self) -> Result<Box<o::ValueTypeProjection>, StoreError> {
        <Self as Project<o::ValueTypeProjection>>::project(self).map(Box::new)
    }
}

dictionary!(EkrKernelNodeDraftProjectionProperties => Vec<g::TypedValue>);
dictionary!(EkrKernelEdgeDraftProjectionProperties => Vec<g::TypedValue>);
dictionary!(EkrKernelInvocationProjectionArguments => g::TypedValue);
dictionary!(EkrOntologyNodeTypeDeclarationProperties => o::PropertyDeclaration);
dictionary!(EkrOntologyNodeTypeDeclarationOperations => o::OperationDefinition);
dictionary!(EkrOntologyEdgeTypeDeclarationProperties => o::PropertyDeclaration);
dictionary!(EkrOntologyOperationDefinitionArguments => Box<o::ValueTypeProjection>);
dictionary!(EkrOntologyValueTypeProjectionFields => Box<o::ValueTypeProjection>);

enumeration!(EkrIntegrateApplicationProgress => m::ApplicationProgress { V0=>Complete,V1=>Elected,V2=>FactsInProgress,V3=>Partial,V4=>SchemaCommitted });
enumeration!(EkrIntegrateApplicationStepKind => m::ApplicationStepKind { V0=>Corrections,V1=>Mapping,V2=>Schema });
enumeration!(EkrStorePublicationCommandKind => s::PublicationCommandKind { V0=>AnswerAttention,V1=>Bootstrap,V2=>Commit,V3=>Propose,V4=>UpgradeAuthority,V5=>Validate });
enumeration!(EkrOntologyCardinality => o::Cardinality { V0=>Many,V1=>One });
enumeration!(EkrOntologyValueKind => o::ValueKind { V0=>Boolean,V1=>Decimal,V2=>Duration,V3=>Enum,V4=>Float,V5=>Integer,V6=>List,V7=>NodeRef,V8=>Record,V9=>String,V10=>Timestamp });
enumeration!(EkrGraphAssertionLifecycleKind => g::AssertionLifecycleKind { V0=>Active,V1=>Retracted,V2=>Superseded });
enumeration!(EkrGraphAssessmentKind => g::AssessmentKind { V0=>Accepted,V1=>Disputed,V2=>Proposed,V3=>Rejected,V4=>Validating });
enumeration!(EkrGraphCanonicalValueKind => g::CanonicalValueKind { V0=>Boolean,V1=>Decimal,V2=>Duration,V3=>Enum,V4=>Integer,V5=>List,V6=>NodeRef,V7=>Record,V8=>String,V9=>Timestamp });
enumeration!(EkrGraphEvidenceKind => g::EvidenceKind { V0=>DatabaseRecord,V1=>Document,V2=>GraphAssertion,V3=>HumanStatement,V4=>Observation,V5=>Url });
enumeration!(EkrGraphObjectKind => g::ObjectKind { V0=>Node,V1=>Type,V2=>Value });
enumeration!(EkrGraphPredicateKind => g::PredicateKind { V0=>Property,V1=>Relation });
enumeration!(EkrGraphSubjectKind => g::SubjectKind { V0=>Edge,V1=>Node,V2=>Type });

impl Project<m::MappingValue> for w::EkrIntegrateMappingValue {
    fn project(&self) -> Result<m::MappingValue, StoreError> {
        // Constant tags are exhaustively matched too: generated wrapper drift is not ignored.
        Ok(match self {
            Self::V0(v) => {
                let w::EkrIntegrateMappingValueVariant0 { kind, value } = v;
                match kind {
                    w::EkrIntegrateMappingValueVariant0Kind::V0 => {}
                };
                m::MappingValue::Constant(value.project()?)
            }
            Self::V1(v) => {
                let w::EkrIntegrateMappingValueVariant1 { kind, value } = v;
                match kind {
                    w::EkrIntegrateMappingValueVariant1Kind::V0 => {}
                };
                m::MappingValue::CopyField(value.project()?)
            }
            Self::V2(v) => {
                let w::EkrIntegrateMappingValueVariant2 { kind, value } = v;
                match kind {
                    w::EkrIntegrateMappingValueVariant2Kind::V0 => {}
                };
                m::MappingValue::CopyRelation(value.project()?)
            }
        })
    }
}
macro_rules! operation {
    ($value:ident, $record:ident, $tag:ident, $variant:ident) => {{
        let w::$record { kind, value } = $value;
        match kind {
            w::$tag::V0 => {}
        }
        k::CanonicalOperationProjection::$variant(value.project()?)
    }};
}
impl Project<k::CanonicalOperationProjection> for w::EkrKernelCanonicalOperationProjection {
    fn project(&self) -> Result<k::CanonicalOperationProjection, StoreError> {
        Ok(match self {
            Self::V0(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant0,
                EkrKernelCanonicalOperationProjectionVariant0Kind,
                AddAlias
            ),
            Self::V1(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant1,
                EkrKernelCanonicalOperationProjectionVariant1Kind,
                AddAssertion
            ),
            Self::V2(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant2,
                EkrKernelCanonicalOperationProjectionVariant2Kind,
                AddEvidence
            ),
            Self::V3(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant3,
                EkrKernelCanonicalOperationProjectionVariant3Kind,
                AttachEvidence
            ),
            Self::V4(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant4,
                EkrKernelCanonicalOperationProjectionVariant4Kind,
                CreateEdge
            ),
            Self::V5(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant5,
                EkrKernelCanonicalOperationProjectionVariant5Kind,
                CreateNode
            ),
            Self::V6(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant6,
                EkrKernelCanonicalOperationProjectionVariant6Kind,
                DefineEdgeType
            ),
            Self::V7(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant7,
                EkrKernelCanonicalOperationProjectionVariant7Kind,
                DefineNodeType
            ),
            Self::V8(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant8,
                EkrKernelCanonicalOperationProjectionVariant8Kind,
                DeleteEdge
            ),
            Self::V9(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant9,
                EkrKernelCanonicalOperationProjectionVariant9Kind,
                Invoke
            ),
            Self::V10(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant10,
                EkrKernelCanonicalOperationProjectionVariant10Kind,
                MergeEntity
            ),
            Self::V11(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant11,
                EkrKernelCanonicalOperationProjectionVariant11Kind,
                ModifyProperty
            ),
            Self::V12(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant12,
                EkrKernelCanonicalOperationProjectionVariant12Kind,
                RetractAssertion
            ),
            Self::V13(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant13,
                EkrKernelCanonicalOperationProjectionVariant13Kind,
                SupersedeAssertion
            ),
            Self::V14(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant14,
                EkrKernelCanonicalOperationProjectionVariant14Kind,
                UpdateProperty
            ),
            Self::V15(v) => operation!(
                v,
                EkrKernelCanonicalOperationProjectionVariant15,
                EkrKernelCanonicalOperationProjectionVariant15Kind,
                WidenEdgeType
            ),
        })
    }
}

#[cfg(test)]
mod tests;
