//! The typed inventory shared by the serde and mint contracts and checked against every ESS domain.

macro_rules! identity_types {
    ($cases:ident) => {
        $cases! {
            agent_id => AgentId,
            transaction_id => TransactionId,
            revision_id => RevisionId,
            issue_id => IssueId,
            type_id => TypeId,
            property_id => PropertyId,
            schema_version_id => SchemaVersionId,
            graph_root_id => GraphRootId,
            node_id => NodeId,
            edge_id => EdgeId,
            assertion_id => AssertionId,
            support_id => SupportId,
            attachment_id => AttachmentId,
            evidence_id => EvidenceId,
            observation_id => ObservationId,
            event_id => EventId,
            merge_id => MergeId,
            split_id => SplitId,
            source_unit_id => SourceUnitId,
            source_checkpoint_id => SourceCheckpointId,
        }
    };
}

pub(crate) use identity_types;
