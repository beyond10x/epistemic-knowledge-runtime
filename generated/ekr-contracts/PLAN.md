<!--
  generated from ekr v1
  model digest 76cac94520a78179089e10b0871c7979eacd35197c8e7b584616fac60d49e705
  contract digest c0793b41517a2b54756a5ae9c6d0c85aa53885defe0f88328a27a2c46c880d14
  do not edit: regenerate with `ess synthesize`
-->
# Synthesis plan — ekr v1

Scope: `component-skeletons`, planned by `ess-synth`. Regenerate with `ess synthesize`.

703 capabilities: **653 generated**, **39 obligations**, **11 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `ekr.graph.Assertion.State` |
| domain type | `ekr.graph.AssertionId` |
| domain type | `ekr.graph.AssertionLifecycleKind` |
| domain type | `ekr.graph.AssertionLifecycleProjection` |
| domain type | `ekr.graph.AssertionRecord` |
| domain type | `ekr.graph.AssessmentKind` |
| domain type | `ekr.graph.AssessmentProjection` |
| domain type | `ekr.graph.AttachedEvidenceRecord` |
| domain type | `ekr.graph.AttachmentId` |
| domain type | `ekr.graph.CanonicalValueKind` |
| domain type | `ekr.graph.Edge.State` |
| domain type | `ekr.graph.EdgeId` |
| domain type | `ekr.graph.EdgeRecord` |
| domain type | `ekr.graph.Evidence.State` |
| domain type | `ekr.graph.EvidenceAttachment.State` |
| domain type | `ekr.graph.EvidenceId` |
| domain type | `ekr.graph.EvidenceKind` |
| domain type | `ekr.graph.EvidenceRecord` |
| domain type | `ekr.graph.EvidenceSourceProjection` |
| domain type | `ekr.graph.GraphDocumentBodyProjection` |
| domain type | `ekr.graph.GraphDocumentFormatV2` |
| domain type | `ekr.graph.GraphDocumentV2Projection` |
| domain type | `ekr.graph.GraphRoot.State` |
| domain type | `ekr.graph.GraphRootId` |
| domain type | `ekr.graph.GraphRootRecord` |
| domain type | `ekr.graph.Node.State` |
| domain type | `ekr.graph.NodeId` |
| domain type | `ekr.graph.NodeRecord` |
| domain type | `ekr.graph.ObjectKind` |
| domain type | `ekr.graph.ObjectProjection` |
| domain type | `ekr.graph.Observation.State` |
| domain type | `ekr.graph.ObservationId` |
| domain type | `ekr.graph.ObservationKind` |
| domain type | `ekr.graph.ObservationRecord` |
| domain type | `ekr.graph.PredicateKind` |
| domain type | `ekr.graph.PredicateProjection` |
| domain type | `ekr.graph.RevisionRoot` |
| domain type | `ekr.graph.Space` |
| domain type | `ekr.graph.SubjectKind` |
| domain type | `ekr.graph.SubjectProjection` |
| domain type | `ekr.graph.Support.State` |
| domain type | `ekr.graph.SupportId` |
| domain type | `ekr.graph.TemporalRange` |
| domain type | `ekr.graph.TransactionTime` |
| domain type | `ekr.graph.TypedValue` |
| domain type | `ekr.integrate.AdditiveSchemaOperation` |
| domain type | `ekr.integrate.AmbiguousExtraction` |
| domain type | `ekr.integrate.AmbiguousReference` |
| domain type | `ekr.integrate.ApplicationProgress` |
| domain type | `ekr.integrate.ApplicationReceipt.State` |
| domain type | `ekr.integrate.ApplicationReceiptId` |
| domain type | `ekr.integrate.ApplicationReceiptSnapshot` |
| domain type | `ekr.integrate.ApplicationReport` |
| domain type | `ekr.integrate.CanonicalDerivation.State` |
| domain type | `ekr.integrate.CommittedExtraction` |
| domain type | `ekr.integrate.DeclaredSourceField` |
| domain type | `ekr.integrate.DeclaredSourceRelation` |
| domain type | `ekr.integrate.EdgeTypeSpec` |
| domain type | `ekr.integrate.ExtractedFact` |
| domain type | `ekr.integrate.ExtractedReference` |
| domain type | `ekr.integrate.ExtractionDocument` |
| domain type | `ekr.integrate.ExtractionDocumentPath` |
| domain type | `ekr.integrate.ExtractionFormat` |
| domain type | `ekr.integrate.ExtractionIssue` |
| domain type | `ekr.integrate.ExtractionRefusal` |
| domain type | `ekr.integrate.ExtractionRefusalCode` |
| domain type | `ekr.integrate.ExtractionReport` |
| domain type | `ekr.integrate.GapGroup` |
| domain type | `ekr.integrate.GapGroupId` |
| domain type | `ekr.integrate.HeldExtraction` |
| domain type | `ekr.integrate.HeldReason` |
| domain type | `ekr.integrate.IncubationImportReceipt` |
| domain type | `ekr.integrate.IncubationOutcome` |
| domain type | `ekr.integrate.IntegrationBlocker.State` |
| domain type | `ekr.integrate.IntegrationBlockerId` |
| domain type | `ekr.integrate.IntegrationBlockerKind` |
| domain type | `ekr.integrate.IntegrationBlockerSnapshot` |
| domain type | `ekr.integrate.IntegrationItemReceipt` |
| domain type | `ekr.integrate.Interpretation.State` |
| domain type | `ekr.integrate.InterpretationCoordinate` |
| domain type | `ekr.integrate.InterpretationDocument` |
| domain type | `ekr.integrate.InterpretationId` |
| domain type | `ekr.integrate.InterpretationImport` |
| domain type | `ekr.integrate.InterpretationObservation.State` |
| domain type | `ekr.integrate.InterpretationRead` |
| domain type | `ekr.integrate.InterpretationVersion` |
| domain type | `ekr.integrate.KnowledgeMapping` |
| domain type | `ekr.integrate.MappingPreview` |
| domain type | `ekr.integrate.MappingRecord.State` |
| domain type | `ekr.integrate.MappingRecordId` |
| domain type | `ekr.integrate.MappingValue` |
| domain type | `ekr.integrate.Merge.State` |
| domain type | `ekr.integrate.MergeId` |
| domain type | `ekr.integrate.NodeTypeSpec` |
| domain type | `ekr.integrate.OntologySpec` |
| domain type | `ekr.integrate.OptionalPropertyAddition` |
| domain type | `ekr.integrate.ProcessingDisposition` |
| domain type | `ekr.integrate.ProcessingReceipt.State` |
| domain type | `ekr.integrate.ProcessingReceiptId` |
| domain type | `ekr.integrate.ProcessingReceiptSnapshot` |
| domain type | `ekr.integrate.PropertyFact` |
| domain type | `ekr.integrate.PropertySpec` |
| domain type | `ekr.integrate.ProposalEvidence.State` |
| domain type | `ekr.integrate.ProposalObservation.State` |
| domain type | `ekr.integrate.ProposalReview.State` |
| domain type | `ekr.integrate.ProposalReviewId` |
| domain type | `ekr.integrate.ProposalReviewSnapshot` |
| domain type | `ekr.integrate.ProposalSource` |
| domain type | `ekr.integrate.ProposalSourceBinding.State` |
| domain type | `ekr.integrate.RejectedExtraction` |
| domain type | `ekr.integrate.RelationFact` |
| domain type | `ekr.integrate.ResolutionOutcome` |
| domain type | `ekr.integrate.ResolutionRefusal` |
| domain type | `ekr.integrate.ResolutionRefusalCode` |
| domain type | `ekr.integrate.ResolvedReference` |
| domain type | `ekr.integrate.RetainedInterpretation` |
| domain type | `ekr.integrate.ReviewDecision` |
| domain type | `ekr.integrate.SchemaLearningRequest` |
| domain type | `ekr.integrate.SchemaLearningResult` |
| domain type | `ekr.integrate.SchemaProposal.State` |
| domain type | `ekr.integrate.SchemaProposalDocument` |
| domain type | `ekr.integrate.SchemaProposalId` |
| domain type | `ekr.integrate.Split.State` |
| domain type | `ekr.integrate.SplitId` |
| domain type | `ekr.integrate.TypedReference` |
| domain type | `ekr.integrate.ValueSpec` |
| domain type | `ekr.kernel.Agent.State` |
| domain type | `ekr.kernel.AgentId` |
| domain type | `ekr.kernel.AliasAdditionProjection` |
| domain type | `ekr.kernel.AnswerOutcome` |
| domain type | `ekr.kernel.AnswerReceipt` |
| domain type | `ekr.kernel.AnswerRecordFormat` |
| domain type | `ekr.kernel.AnswerReplacement.State` |
| domain type | `ekr.kernel.ApplicationProfileV1` |
| domain type | `ekr.kernel.AttentionAnswerApplication` |
| domain type | `ekr.kernel.AttentionAnswerTarget` |
| domain type | `ekr.kernel.AttentionAnsweredPayload` |
| domain type | `ekr.kernel.AttentionItem` |
| domain type | `ekr.kernel.AttentionKind` |
| domain type | `ekr.kernel.AttentionSubject` |
| domain type | `ekr.kernel.AuthorityStateFormatV1` |
| domain type | `ekr.kernel.AuthorityStateV1` |
| domain type | `ekr.kernel.AuthorityTransition.State` |
| domain type | `ekr.kernel.AuthorityTransitionFormat` |
| domain type | `ekr.kernel.AuthorityTransitionId` |
| domain type | `ekr.kernel.AuthorityTransitionRecord` |
| domain type | `ekr.kernel.AuthorityUpgradeApplication` |
| domain type | `ekr.kernel.AuthorityUpgradeTarget` |
| domain type | `ekr.kernel.AuthorityUpgradedPayload` |
| domain type | `ekr.kernel.AuthorityVersion` |
| domain type | `ekr.kernel.BootstrapContext` |
| domain type | `ekr.kernel.CanonicalOperationProjection` |
| domain type | `ekr.kernel.CanonicalTransactionProjection` |
| domain type | `ekr.kernel.ClaimCorrection` |
| domain type | `ekr.kernel.ClaimCorrectionKind` |
| domain type | `ekr.kernel.ClaimReplacement` |
| domain type | `ekr.kernel.CliHostConfigurationV1` |
| domain type | `ekr.kernel.CliHostFormatV1` |
| domain type | `ekr.kernel.CommitCommandResult` |
| domain type | `ekr.kernel.CommitReceiptFormatV1` |
| domain type | `ekr.kernel.CommitReceiptV1` |
| domain type | `ekr.kernel.CommittedPayload` |
| domain type | `ekr.kernel.ContentHash` |
| domain type | `ekr.kernel.CreatedIdentitiesV1` |
| domain type | `ekr.kernel.Dispute.State` |
| domain type | `ekr.kernel.DisputeClaim.State` |
| domain type | `ekr.kernel.DisputeId` |
| domain type | `ekr.kernel.EdgeDraftProjection` |
| domain type | `ekr.kernel.EdgeWideningProjection` |
| domain type | `ekr.kernel.EntityMerge` |
| domain type | `ekr.kernel.EventId` |
| domain type | `ekr.kernel.EvidenceAdditionProjection` |
| domain type | `ekr.kernel.EvidenceAttachmentProjection` |
| domain type | `ekr.kernel.ExplainedAnswer` |
| domain type | `ekr.kernel.ExplainedAttachment` |
| domain type | `ekr.kernel.ExplainedCommit` |
| domain type | `ekr.kernel.ExplainedLifecycle` |
| domain type | `ekr.kernel.ExplainedProposal` |
| domain type | `ekr.kernel.ExplainedSeed` |
| domain type | `ekr.kernel.ExplainedValidation` |
| domain type | `ekr.kernel.ExplanationFormatV2` |
| domain type | `ekr.kernel.ExplanationLink` |
| domain type | `ekr.kernel.ExplanationResult` |
| domain type | `ekr.kernel.GraphTransaction.State` |
| domain type | `ekr.kernel.HumanAnswer.State` |
| domain type | `ekr.kernel.HumanAnswerId` |
| domain type | `ekr.kernel.HumanAnswerRecord` |
| domain type | `ekr.kernel.HumanDecisionAudience` |
| domain type | `ekr.kernel.HumanDecisionFormat` |
| domain type | `ekr.kernel.HumanDecisionIntent` |
| domain type | `ekr.kernel.HumanDecisionRecord` |
| domain type | `ekr.kernel.HumanDecisionScope` |
| domain type | `ekr.kernel.HumanDecisionTarget` |
| domain type | `ekr.kernel.InvocationProjection` |
| domain type | `ekr.kernel.IssueId` |
| domain type | `ekr.kernel.MigratedOccurrence` |
| domain type | `ekr.kernel.NodeDraftProjection` |
| domain type | `ekr.kernel.OperationKind` |
| domain type | `ekr.kernel.P1ValidationProfileV1` |
| domain type | `ekr.kernel.PropertyModificationProjection` |
| domain type | `ekr.kernel.PropertyMutationProjection` |
| domain type | `ekr.kernel.ProposalRecordFormatV1` |
| domain type | `ekr.kernel.ProposalRecordV1` |
| domain type | `ekr.kernel.ProposedPayload` |
| domain type | `ekr.kernel.ProposerSeparationV1` |
| domain type | `ekr.kernel.ProvenanceProfileV1` |
| domain type | `ekr.kernel.RegisteredAgent` |
| domain type | `ekr.kernel.RejectedPayload` |
| domain type | `ekr.kernel.RejectedTransactionV1` |
| domain type | `ekr.kernel.RejectionRecordFormatV1` |
| domain type | `ekr.kernel.RejectionRecordV1` |
| domain type | `ekr.kernel.RejectionsFormatV1` |
| domain type | `ekr.kernel.RejectionsV1` |
| domain type | `ekr.kernel.RetainedHumanDecision.State` |
| domain type | `ekr.kernel.Retraction` |
| domain type | `ekr.kernel.ReviewBasis` |
| domain type | `ekr.kernel.ReviewSignatureAlgorithm` |
| domain type | `ekr.kernel.ReviewerTrustEnrollment` |
| domain type | `ekr.kernel.ReviewerTrustFormat` |
| domain type | `ekr.kernel.ReviewerTrustPolicy` |
| domain type | `ekr.kernel.ReviewerVerificationKey` |
| domain type | `ekr.kernel.Revision.State` |
| domain type | `ekr.kernel.RevisionEventFormatV2` |
| domain type | `ekr.kernel.RevisionEventFormatV3` |
| domain type | `ekr.kernel.RevisionEventFormatV4` |
| domain type | `ekr.kernel.RevisionEventV2` |
| domain type | `ekr.kernel.RevisionEventV3` |
| domain type | `ekr.kernel.RevisionEventV4` |
| domain type | `ekr.kernel.RevisionId` |
| domain type | `ekr.kernel.RevisionNumber` |
| domain type | `ekr.kernel.RevisionPayload` |
| domain type | `ekr.kernel.RevisionPayloadV3` |
| domain type | `ekr.kernel.RevisionPayloadV4` |
| domain type | `ekr.kernel.RulesetV1` |
| domain type | `ekr.kernel.SchemaReviewTarget` |
| domain type | `ekr.kernel.SchemaTransactionEvidence.State` |
| domain type | `ekr.kernel.SeedDocumentFormatV2` |
| domain type | `ekr.kernel.SeedDocumentPath` |
| domain type | `ekr.kernel.SeedEnvelopeFormatV2` |
| domain type | `ekr.kernel.SeedEnvelopeFormatV3` |
| domain type | `ekr.kernel.SeedEnvelopeV2Projection` |
| domain type | `ekr.kernel.SeedEnvelopeV3Projection` |
| domain type | `ekr.kernel.SeedInputV2Projection` |
| domain type | `ekr.kernel.SeedInputV3Projection` |
| domain type | `ekr.kernel.SeedResultFormatV1` |
| domain type | `ekr.kernel.SeedResultV1` |
| domain type | `ekr.kernel.SeededPayload` |
| domain type | `ekr.kernel.SignedHumanDecision` |
| domain type | `ekr.kernel.SnapshotResult` |
| domain type | `ekr.kernel.StalePayload` |
| domain type | `ekr.kernel.StaleRecordFormatV1` |
| domain type | `ekr.kernel.StaleRecordV1` |
| domain type | `ekr.kernel.StoreMigrationV1` |
| domain type | `ekr.kernel.Supersession` |
| domain type | `ekr.kernel.TransactionDocumentFormatV1` |
| domain type | `ekr.kernel.TransactionDocumentPath` |
| domain type | `ekr.kernel.TransactionId` |
| domain type | `ekr.kernel.TrustedOperatorIdentity` |
| domain type | `ekr.kernel.TrustedReviewHostBinding` |
| domain type | `ekr.kernel.UpgradeContradiction` |
| domain type | `ekr.kernel.UpgradePreview` |
| domain type | `ekr.kernel.ValidatedPayload` |
| domain type | `ekr.kernel.ValidationBasisFormatV1` |
| domain type | `ekr.kernel.ValidationBasisV1` |
| domain type | `ekr.kernel.ValidationCommandResult` |
| domain type | `ekr.kernel.ValidationIssue.State` |
| domain type | `ekr.kernel.ValidationIssueRecord` |
| domain type | `ekr.kernel.ValidationMaterialFormatV1` |
| domain type | `ekr.kernel.ValidationMaterialV1Projection` |
| domain type | `ekr.kernel.ValidationProfileFormatV1` |
| domain type | `ekr.kernel.ValidationReceiptFormatV1` |
| domain type | `ekr.kernel.ValidationReceiptV1` |
| domain type | `ekr.kernel.ValidatorName` |
| domain type | `ekr.observe.ObservationIdempotencyKey` |
| domain type | `ekr.observe.ObservationImport` |
| domain type | `ekr.observe.ObservationImportReceipt` |
| domain type | `ekr.observe.ObservationRetentionOutcome` |
| domain type | `ekr.observe.PollHealth` |
| domain type | `ekr.observe.PollStatus` |
| domain type | `ekr.observe.RetainedObservation.State` |
| domain type | `ekr.observe.RetainedObservationRead` |
| domain type | `ekr.observe.SourceCheckpoint.State` |
| domain type | `ekr.observe.SourceCheckpointId` |
| domain type | `ekr.observe.SourceRecordObservation` |
| domain type | `ekr.observe.SourceUnit.State` |
| domain type | `ekr.observe.SourceUnitId` |
| domain type | `ekr.ontology.Cardinality` |
| domain type | `ekr.ontology.EdgeType.State` |
| domain type | `ekr.ontology.EdgeTypeDeclaration` |
| domain type | `ekr.ontology.EdgeWidening` |
| domain type | `ekr.ontology.EvolveRefusalCode` |
| domain type | `ekr.ontology.IncompatibilityCode` |
| domain type | `ekr.ontology.LifecycleDeclaration` |
| domain type | `ekr.ontology.NodeType.State` |
| domain type | `ekr.ontology.NodeTypeDeclaration` |
| domain type | `ekr.ontology.OntologyDocumentProjection` |
| domain type | `ekr.ontology.OperationDefinition` |
| domain type | `ekr.ontology.PropertyDeclaration` |
| domain type | `ekr.ontology.PropertyDefinition.State` |
| domain type | `ekr.ontology.PropertyId` |
| domain type | `ekr.ontology.PropertyModification` |
| domain type | `ekr.ontology.SchemaChange` |
| domain type | `ekr.ontology.SchemaVersion.State` |
| domain type | `ekr.ontology.SchemaVersionId` |
| domain type | `ekr.ontology.SchemaVersionRecord` |
| domain type | `ekr.ontology.Transition` |
| domain type | `ekr.ontology.TypeId` |
| domain type | `ekr.ontology.ValueKind` |
| domain type | `ekr.ontology.ValueTypeProjection` |
| domain type | `ekr.store.NativeBlobWrite` |
| domain type | `ekr.store.NativeClaim` |
| domain type | `ekr.store.NativeCommandMeta` |
| domain type | `ekr.store.NativeExpected` |
| domain type | `ekr.store.NativeExpectedKind` |
| domain type | `ekr.store.NativeNewEvent` |
| domain type | `ekr.store.NativePublicationRequest` |
| domain type | `ekr.store.NativeStreamAppend` |
| domain type | `ekr.store.NativeStreamId` |
| domain type | `ekr.store.Publication` |
| domain type | `ekr.store.PublicationCommandKey` |
| domain type | `ekr.store.PublicationCommandKind` |
| domain type | `ekr.store.PublicationObject` |
| domain type | `ekr.store.PublicationPreparationFormatV1` |
| domain type | `ekr.store.PublicationPreparationV1` |
| domain type | `ekr.store.PublicationResolution` |
| domain type | `ekr.store.StagedPublicationObject` |
| domain type | `ekr.store.StorageClass` |
| domain type | `ekr.store.StoredObject.State` |
| domain type | `ekr.views.AssertionQuality` |
| domain type | `ekr.views.BasisPoints` |
| domain type | `ekr.views.BooleanValue` |
| domain type | `ekr.views.ChangeCursor` |
| domain type | `ekr.views.ChangeKind` |
| domain type | `ekr.views.ChangesMeta` |
| domain type | `ekr.views.CodeNameFinding` |
| domain type | `ekr.views.CodeNameKind` |
| domain type | `ekr.views.CodeNameMatch` |
| domain type | `ekr.views.CodeNameMode` |
| domain type | `ekr.views.CodeNamesFormatV1` |
| domain type | `ekr.views.CodeNamesMeta` |
| domain type | `ekr.views.CodeNamesV1` |
| domain type | `ekr.views.DetailMeta` |
| domain type | `ekr.views.DetailNode` |
| domain type | `ekr.views.EdgeSide` |
| domain type | `ekr.views.EpochMillis` |
| domain type | `ekr.views.FactJudgement` |
| domain type | `ekr.views.FactJudgementsFormatV1` |
| domain type | `ekr.views.FactJudgementsV1` |
| domain type | `ekr.views.FactQualityFormatV1` |
| domain type | `ekr.views.FactQualityMeta` |
| domain type | `ekr.views.FactQualityV1` |
| domain type | `ekr.views.FactSampleFormatV1` |
| domain type | `ekr.views.FactSampleMeta` |
| domain type | `ekr.views.FactSampleV1` |
| domain type | `ekr.views.FactVerdict` |
| domain type | `ekr.views.GraphChange` |
| domain type | `ekr.views.GraphChangesFormatV1` |
| domain type | `ekr.views.GraphChangesV1` |
| domain type | `ekr.views.GraphOverviewFormatV1` |
| domain type | `ekr.views.GraphOverviewV1` |
| domain type | `ekr.views.GraphProjectionFormatV1` |
| domain type | `ekr.views.GraphProjectionV1` |
| domain type | `ekr.views.GraphSliceFormatV1` |
| domain type | `ekr.views.GraphSliceV1` |
| domain type | `ekr.views.GraphTimelineFormatV1` |
| domain type | `ekr.views.GraphTimelineV1` |
| domain type | `ekr.views.IntegerValue` |
| domain type | `ekr.views.ListValue` |
| domain type | `ekr.views.MatchField` |
| domain type | `ekr.views.MatchTier` |
| domain type | `ekr.views.MatchesMeta` |
| domain type | `ekr.views.ModifiedProperty` |
| domain type | `ekr.views.NodeDetailFormatV1` |
| domain type | `ekr.views.NodeDetailV1` |
| domain type | `ekr.views.NodeMatch` |
| domain type | `ekr.views.NodeMatchesFormatV1` |
| domain type | `ekr.views.NodeMatchesV1` |
| domain type | `ekr.views.NodeSummary` |
| domain type | `ekr.views.Ocel20Log` |
| domain type | `ekr.views.OcelAttributeType` |
| domain type | `ekr.views.OcelEvent` |
| domain type | `ekr.views.OcelEventAttribute` |
| domain type | `ekr.views.OcelFormatV1` |
| domain type | `ekr.views.OcelLogV1` |
| domain type | `ekr.views.OcelMeta` |
| domain type | `ekr.views.OcelName` |
| domain type | `ekr.views.OcelNames` |
| domain type | `ekr.views.OcelObject` |
| domain type | `ekr.views.OcelObjectAttribute` |
| domain type | `ekr.views.OcelRelationship` |
| domain type | `ekr.views.OcelTime` |
| domain type | `ekr.views.OcelType` |
| domain type | `ekr.views.OcelTypeAttribute` |
| domain type | `ekr.views.OverviewMeta` |
| domain type | `ekr.views.OverviewRevision` |
| domain type | `ekr.views.OverviewRoles` |
| domain type | `ekr.views.OverviewSchema` |
| domain type | `ekr.views.OverviewSchemaVersion` |
| domain type | `ekr.views.OverviewTimeline` |
| domain type | `ekr.views.ProjectedAssertion` |
| domain type | `ekr.views.ProjectedEdge` |
| domain type | `ekr.views.ProjectedEdgeType` |
| domain type | `ekr.views.ProjectedEvidence` |
| domain type | `ekr.views.ProjectedLifecycle` |
| domain type | `ekr.views.ProjectedNode` |
| domain type | `ekr.views.ProjectedNodeType` |
| domain type | `ekr.views.ProjectedOntology` |
| domain type | `ekr.views.ProjectedProperty` |
| domain type | `ekr.views.ProjectedRevision` |
| domain type | `ekr.views.ProjectedSchema` |
| domain type | `ekr.views.ProjectedSchemaVersion` |
| domain type | `ekr.views.ProjectedValue` |
| domain type | `ekr.views.ProjectionMeta` |
| domain type | `ekr.views.PropertyAspect` |
| domain type | `ekr.views.PropertyQuality` |
| domain type | `ekr.views.QualityMeta` |
| domain type | `ekr.views.RecordValue` |
| domain type | `ekr.views.ReferencingAssertion` |
| domain type | `ekr.views.SampleOrigin` |
| domain type | `ekr.views.SampledEvidence` |
| domain type | `ekr.views.SampledFact` |
| domain type | `ekr.views.SchemaEvidenceEntry` |
| domain type | `ekr.views.SchemaMember` |
| domain type | `ekr.views.SchemaMemberKind` |
| domain type | `ekr.views.SharedName` |
| domain type | `ekr.views.SinceKind` |
| domain type | `ekr.views.SliceCursor` |
| domain type | `ekr.views.SliceEdge` |
| domain type | `ekr.views.SliceMeta` |
| domain type | `ekr.views.SliceNode` |
| domain type | `ekr.views.SourceText` |
| domain type | `ekr.views.StoreLocation` |
| domain type | `ekr.views.StoreQualityFormatV1` |
| domain type | `ekr.views.StoreQualityV1` |
| domain type | `ekr.views.TextValue` |
| domain type | `ekr.views.TimelineBucket` |
| domain type | `ekr.views.TimelineBucketWidth` |
| domain type | `ekr.views.TimelineCell` |
| domain type | `ekr.views.TimelineEvent` |
| domain type | `ekr.views.TimelineMeta` |
| domain type | `ekr.views.TimelineRow` |
| domain type | `ekr.views.TimelineRowType` |
| domain type | `ekr.views.TimelineStep` |
| domain type | `ekr.views.TypeCount` |
| domain type | `ekr.views.TypeTiming` |
| domain type | `ekr.views.WidenedEnd` |
| entity lifecycle | `ekr.graph.Assertion` |
| entity lifecycle | `ekr.graph.Edge` |
| entity lifecycle | `ekr.graph.Evidence` |
| entity lifecycle | `ekr.graph.EvidenceAttachment` |
| entity lifecycle | `ekr.graph.GraphRoot` |
| entity lifecycle | `ekr.graph.Node` |
| entity lifecycle | `ekr.graph.Observation` |
| entity lifecycle | `ekr.graph.Support` |
| entity lifecycle | `ekr.integrate.ApplicationReceipt` |
| entity lifecycle | `ekr.integrate.CanonicalDerivation` |
| entity lifecycle | `ekr.integrate.IntegrationBlocker` |
| entity lifecycle | `ekr.integrate.Interpretation` |
| entity lifecycle | `ekr.integrate.InterpretationObservation` |
| entity lifecycle | `ekr.integrate.MappingRecord` |
| entity lifecycle | `ekr.integrate.Merge` |
| entity lifecycle | `ekr.integrate.ProcessingReceipt` |
| entity lifecycle | `ekr.integrate.ProposalEvidence` |
| entity lifecycle | `ekr.integrate.ProposalObservation` |
| entity lifecycle | `ekr.integrate.ProposalReview` |
| entity lifecycle | `ekr.integrate.ProposalSourceBinding` |
| entity lifecycle | `ekr.integrate.SchemaProposal` |
| entity lifecycle | `ekr.integrate.Split` |
| entity lifecycle | `ekr.kernel.Agent` |
| entity lifecycle | `ekr.kernel.AnswerReplacement` |
| entity lifecycle | `ekr.kernel.AuthorityTransition` |
| entity lifecycle | `ekr.kernel.Dispute` |
| entity lifecycle | `ekr.kernel.DisputeClaim` |
| entity lifecycle | `ekr.kernel.GraphTransaction` |
| entity lifecycle | `ekr.kernel.HumanAnswer` |
| entity lifecycle | `ekr.kernel.RetainedHumanDecision` |
| entity lifecycle | `ekr.kernel.Revision` |
| entity lifecycle | `ekr.kernel.SchemaTransactionEvidence` |
| entity lifecycle | `ekr.kernel.ValidationIssue` |
| entity lifecycle | `ekr.observe.RetainedObservation` |
| entity lifecycle | `ekr.observe.SourceCheckpoint` |
| entity lifecycle | `ekr.observe.SourceUnit` |
| entity lifecycle | `ekr.ontology.EdgeType` |
| entity lifecycle | `ekr.ontology.NodeType` |
| entity lifecycle | `ekr.ontology.PropertyDefinition` |
| entity lifecycle | `ekr.ontology.SchemaVersion` |
| entity lifecycle | `ekr.store.StoredObject` |
| command contract | `ekr.integrate.ApplyExtraction` |
| command contract | `ekr.integrate.ApplySchemaProposal` |
| command contract | `ekr.integrate.ApproveSchemaProposal` |
| command contract | `ekr.integrate.DiscoverSchemaGaps` |
| command contract | `ekr.integrate.ImportInterpretation` |
| command contract | `ekr.integrate.ListInterpretations` |
| command contract | `ekr.integrate.RejectSchemaProposal` |
| command contract | `ekr.integrate.ShowInterpretation` |
| command contract | `ekr.integrate.ShowSchemaProposal` |
| command contract | `ekr.integrate.SubmitSchemaProposal` |
| command contract | `ekr.kernel.AnswerAttention` |
| command contract | `ekr.kernel.ApplyUpgrade` |
| command contract | `ekr.kernel.Commit` |
| command contract | `ekr.kernel.Explain` |
| command contract | `ekr.kernel.ListAnswers` |
| command contract | `ekr.kernel.ListAttention` |
| command contract | `ekr.kernel.PreviewUpgrade` |
| command contract | `ekr.kernel.Propose` |
| command contract | `ekr.kernel.Seed` |
| command contract | `ekr.kernel.ShowAttention` |
| command contract | `ekr.kernel.Snapshot` |
| command contract | `ekr.kernel.Validate` |
| command contract | `ekr.observe.ImportObservation` |
| command contract | `ekr.observe.ListObservations` |
| command contract | `ekr.observe.ShowObservation` |
| command contract | `ekr.views.ChangesSince` |
| command contract | `ekr.views.DescribeNode` |
| command contract | `ekr.views.DrawFactSample` |
| command contract | `ekr.views.ExpandNeighbourhood` |
| command contract | `ekr.views.ExportOcel` |
| command contract | `ekr.views.FindCodeNames` |
| command contract | `ekr.views.ProjectGraph` |
| command contract | `ekr.views.ProjectOverview` |
| command contract | `ekr.views.ProjectTimeline` |
| command contract | `ekr.views.ReportFactQuality` |
| command contract | `ekr.views.ReportStoreQuality` |
| command contract | `ekr.views.SearchNodes` |
| event type | `ekr.integrate.ApplySchemaProposalResult` |
| event type | `ekr.integrate.ApproveSchemaProposalResult` |
| event type | `ekr.integrate.DiscoverSchemaGapsResult` |
| event type | `ekr.integrate.ExtractionApplied` |
| event type | `ekr.integrate.ImportInterpretationResult` |
| event type | `ekr.integrate.ListInterpretationsResult` |
| event type | `ekr.integrate.RejectSchemaProposalResult` |
| event type | `ekr.integrate.ShowInterpretationResult` |
| event type | `ekr.integrate.ShowSchemaProposalResult` |
| event type | `ekr.integrate.SubmitSchemaProposalResult` |
| event type | `ekr.kernel.AnswerAttentionResult` |
| event type | `ekr.kernel.ApplyUpgradeResult` |
| event type | `ekr.kernel.AttentionAnswered` |
| event type | `ekr.kernel.AuthorityUpgraded` |
| event type | `ekr.kernel.Explained` |
| event type | `ekr.kernel.ListAnswersResult` |
| event type | `ekr.kernel.ListAttentionResult` |
| event type | `ekr.kernel.PreviewUpgradeResult` |
| event type | `ekr.kernel.RevisionCommitted` |
| event type | `ekr.kernel.Seeded` |
| event type | `ekr.kernel.ShowAttentionResult` |
| event type | `ekr.kernel.SnapshotTaken` |
| event type | `ekr.kernel.TransactionProposed` |
| event type | `ekr.kernel.TransactionRejected` |
| event type | `ekr.kernel.TransactionStale` |
| event type | `ekr.kernel.TransactionValidated` |
| event type | `ekr.observe.ImportObservationResult` |
| event type | `ekr.observe.ObservationShown` |
| event type | `ekr.observe.ObservationsListed` |
| event type | `ekr.store.CheckpointWritten` |
| event type | `ekr.store.ObjectRetentionRaised` |
| event type | `ekr.store.ObjectStored` |
| event type | `ekr.store.PublicationPrepared` |
| event type | `ekr.views.ChangesListed` |
| event type | `ekr.views.CodeNamesFound` |
| event type | `ekr.views.FactQualityReported` |
| event type | `ekr.views.FactSampleDrawn` |
| event type | `ekr.views.GraphOverviewed` |
| event type | `ekr.views.GraphProjected` |
| event type | `ekr.views.NeighbourhoodExpanded` |
| event type | `ekr.views.NodeDescribed` |
| event type | `ekr.views.NodesSearched` |
| event type | `ekr.views.OcelExported` |
| event type | `ekr.views.StoreQualityReported` |
| event type | `ekr.views.SubjectsTimelined` |
| error type | `ekr.integrate.ExtractionRefused` |
| error type | `ekr.integrate.KnowledgeRefused` |
| error type | `ekr.kernel.AlreadySeeded` |
| error type | `ekr.kernel.AssertionNotFound` |
| error type | `ekr.kernel.InvalidSeed` |
| error type | `ekr.kernel.KnowledgeRefused` |
| error type | `ekr.kernel.ProposalAttribution` |
| error type | `ekr.kernel.RevisionNotFound` |
| error type | `ekr.kernel.StructurallyInvalid` |
| error type | `ekr.kernel.TransactionNotFound` |
| error type | `ekr.kernel.TransactionStateConflict` |
| error type | `ekr.observe.KnowledgeRefused` |
| error type | `ekr.store.StoreReplaced` |
| error type | `ekr.views.EventTimeInvalid` |
| error type | `ekr.views.EventTypeNotFound` |
| error type | `ekr.views.JudgedTwice` |
| error type | `ekr.views.LimitExceeded` |
| error type | `ekr.views.NodeNotFound` |
| error type | `ekr.views.NotSeeded` |
| error type | `ekr.views.RevisionNotFound` |
| error type | `ekr.views.SinceMalformed` |
| view type | `ekr.graph.Assertions` |
| view type | `ekr.graph.SettledAssertions` |
| view query | `ekr.graph.SettledAssertions` |
| view type | `ekr.integrate.ApplicationReceiptRecords` |
| view query | `ekr.integrate.ApplicationReceiptRecords` |
| view type | `ekr.integrate.CanonicalDerivationRecords` |
| view query | `ekr.integrate.CanonicalDerivationRecords` |
| view type | `ekr.integrate.IntegrationBlockerRecords` |
| view query | `ekr.integrate.IntegrationBlockerRecords` |
| view type | `ekr.integrate.InterpretationObservationRecords` |
| view query | `ekr.integrate.InterpretationObservationRecords` |
| view type | `ekr.integrate.InterpretationRecords` |
| view query | `ekr.integrate.InterpretationRecords` |
| view type | `ekr.integrate.MappingRecordRecords` |
| view query | `ekr.integrate.MappingRecordRecords` |
| view type | `ekr.integrate.ProcessingReceiptRecords` |
| view query | `ekr.integrate.ProcessingReceiptRecords` |
| view type | `ekr.integrate.ProposalEvidenceRecords` |
| view query | `ekr.integrate.ProposalEvidenceRecords` |
| view type | `ekr.integrate.ProposalObservationRecords` |
| view query | `ekr.integrate.ProposalObservationRecords` |
| view type | `ekr.integrate.ProposalReviewRecords` |
| view query | `ekr.integrate.ProposalReviewRecords` |
| view type | `ekr.integrate.ProposalSourceBindingRecords` |
| view query | `ekr.integrate.ProposalSourceBindingRecords` |
| view type | `ekr.integrate.SchemaProposalRecords` |
| view query | `ekr.integrate.SchemaProposalRecords` |
| view type | `ekr.kernel.AuthorityTransitionRecords` |
| view query | `ekr.kernel.AuthorityTransitionRecords` |
| view type | `ekr.kernel.CurrentRevision` |
| view query | `ekr.kernel.CurrentRevision` |
| view type | `ekr.kernel.DisputeClaimRecords` |
| view query | `ekr.kernel.DisputeClaimRecords` |
| view type | `ekr.kernel.DisputeRecords` |
| view query | `ekr.kernel.DisputeRecords` |
| view type | `ekr.kernel.HumanAnswerRecords` |
| view query | `ekr.kernel.HumanAnswerRecords` |
| view type | `ekr.kernel.HumanDecisionRecords` |
| view query | `ekr.kernel.HumanDecisionRecords` |
| view type | `ekr.kernel.PendingTransactions` |
| view query | `ekr.kernel.PendingTransactions` |
| view type | `ekr.kernel.Rejections` |
| view type | `ekr.kernel.RetainedEvidence` |
| view query | `ekr.kernel.RetainedEvidence` |
| view type | `ekr.kernel.Revisions` |
| view query | `ekr.kernel.Revisions` |
| view type | `ekr.kernel.SchemaTransactionEvidenceRecords` |
| view query | `ekr.kernel.SchemaTransactionEvidenceRecords` |
| view type | `ekr.kernel.Transactions` |
| view query | `ekr.kernel.Transactions` |
| view type | `ekr.kernel.ValidationIssues` |
| view query | `ekr.kernel.ValidationIssues` |
| view type | `ekr.observe.RetainedObservationRecords` |
| view query | `ekr.observe.RetainedObservationRecords` |
| view type | `ekr.ontology.NodeTypesByVersion` |
| view query | `ekr.ontology.NodeTypesByVersion` |
| component port | `ekr-graph` |
| component port | `ekr-integrate` |
| component port | `ekr-kernel` |
| component port | `ekr-observe` |
| component port | `ekr-ontology` |
| component port | `ekr-store` |
| component port | `ekr-views` |

## Ports — yours to provide

What the specification fully determines is generated; what it cannot determine is an obligation. A generated command behaviour or view query reads and writes through the ports below, and they are yours to provide: synthesis generates each port's contract and never an implementation of one, so where instances live stays your decision.

| port | what it answers |
| --- | --- |
| storage | one per entity a generated behaviour or query reads or writes: the instance stored under an identity; storing, replacing and removing one; and every stored instance, in the order the store keeps them |
| context | where a generated behaviour asks it: the caller's attributes, every identity and value the specification says the implementation assigns, and whether each `external:` branch is taken |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |
| command behaviour | `ekr.integrate.ApplyExtraction` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.ApplyExtraction` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `applied` otherwise, emits `ekr.integrate.ExtractionApplied`; `refused` externally decided (the reader refuses the document against the ontology of the store's head), error `ekr.integrate.ExtractionRefused` |
| command behaviour | `ekr.integrate.ApplySchemaProposal` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.ApplySchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ApplySchemaProposalResult`; `refused` externally decided (Before mutation require the referenced review to match proposal_id and exact proposal digest, to be Approved, and to be the latest trusted human decision for that proposal. Refuse changed digest/evidence/options/effects or incompatible base schema. A later failure after any commit produces a partial receipt, not a refusal.), error `ekr.integrate.KnowledgeRefused` |
| command behaviour | `ekr.integrate.ApproveSchemaProposal` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.ApproveSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ApproveSchemaProposalResult`; `refused` externally decided (Verify human_proof under the independently enrolled reviewer policy, with a target matching this approve/reject operation, proposal id, proposal digest, statement and expected predecessor decision. Derive the operator from the verified key registry; a host UUID, agent statement or caller-supplied key cannot authenticate a human. The exact proposal, reviewed evidence and intended effects must match. Agent-supplied content cannot grant approval.), error `ekr.integrate.KnowledgeRefused` |
| command behaviour | `ekr.integrate.DiscoverSchemaGaps` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.DiscoverSchemaGaps` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.DiscoverSchemaGapsResult` |
| command behaviour | `ekr.integrate.ImportInterpretation` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.ImportInterpretation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ImportInterpretationResult`; `refused` externally decided (The typed document must match its bytes and digest; its root must be Transient, every observation retained, every evidence source admissible, and an existing version immutable. Local declaration/reference integrity is checked independently of canonical ontology; a canonical vocabulary mismatch is a durable blocker, not an import refusal.), error `ekr.integrate.KnowledgeRefused` |
| command behaviour | `ekr.integrate.ListInterpretations` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.ListInterpretations` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ListInterpretationsResult` |
| command behaviour | `ekr.integrate.RejectSchemaProposal` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.RejectSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.RejectSchemaProposalResult`; `refused` externally decided (Verify human_proof under the independently enrolled reviewer policy, with a target matching this approve/reject operation, proposal id, proposal digest, statement and expected predecessor decision. Derive the operator from the verified key registry; a host UUID, agent statement or caller-supplied key cannot authenticate a human. The exact proposal, reviewed evidence and intended effects must match. Agent-supplied content cannot grant approval.), error `ekr.integrate.KnowledgeRefused` |
| command behaviour | `ekr.integrate.ShowInterpretation` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.ShowInterpretation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ShowInterpretationResult`; `refused` externally decided (An unknown version or changed document digest is refused.), error `ekr.integrate.KnowledgeRefused` |
| command behaviour | `ekr.integrate.ShowSchemaProposal` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.ShowSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.ShowSchemaProposalResult`; `refused` externally decided (An unknown proposal is refused.), error `ekr.integrate.KnowledgeRefused` |
| command behaviour | `ekr.integrate.SubmitSchemaProposal` | kept an obligation by a typed response (`response:`) | given `ekr.integrate.SubmitSchemaProposal` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.integrate.SubmitSchemaProposalResult`; `refused` externally decided (Refuse mutable or missing sources/evidence, nonadditive changes, required property additions, undeclared selectors, mismatched typed constants and enum exhaustion inferred from observed values.), error `ekr.integrate.KnowledgeRefused` |
| command behaviour | `ekr.kernel.AnswerAttention` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.AnswerAttention` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.AnswerAttentionResult`; `refused` externally decided (Verify human_proof under the independently enrolled reviewer policy and the exact AnswerAttention target, statement, corrections and predecessor decision. A host operator UUID or supplied key establishes no human authority. Revalidate against current state. Unrelated revisions alone do not invalidate review; changed evidence, choices or intended effects require renewed human review. An agent-provided identity or unsupported claim cannot authorize resolution.), error `ekr.kernel.KnowledgeRefused` |
| command behaviour | `ekr.kernel.ApplyUpgrade` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.ApplyUpgrade` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.ApplyUpgradeResult`, emits `ekr.kernel.AuthorityUpgraded`; `refused` externally decided (Verify an UpgradeAuthority human_proof and the independently provisioned host/store reviewer binding before enrollment or mutation. Never trust a policy or verification key solely because request content supplies it. Before any mutation refuse a stale head, altered preview digest, unsupported target, unauthenticated operator or incomplete historical verification.), error `ekr.kernel.KnowledgeRefused` |
| command behaviour | `ekr.kernel.Commit` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.Commit` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `committed` when the existing subject is in Validated, takes `commit` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.RevisionCommitted`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`, emits `ekr.store.CheckpointWritten`; `stale` externally decided (the canonical revision moved since the transaction was validated), takes `stale` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionStale`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `retained-commit` when the existing subject is in Committed, returns the exact retained result of `committed` without errors, events, or subject changes; `transaction-not-found` externally decided (no retained transaction carries input.transaction_id), error `ekr.kernel.TransactionNotFound`; `wrong-state` otherwise, error `ekr.kernel.TransactionStateConflict` |
| command behaviour | `ekr.kernel.Explain` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.Explain` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `explained` otherwise, emits `ekr.kernel.Explained`; `not-found` externally decided (the canonical core holds no such assertion), error `ekr.kernel.AssertionNotFound` |
| command behaviour | `ekr.kernel.ListAnswers` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.ListAnswers` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.ListAnswersResult` |
| command behaviour | `ekr.kernel.ListAttention` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.ListAttention` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.ListAttentionResult` |
| command behaviour | `ekr.kernel.PreviewUpgrade` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.PreviewUpgrade` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.PreviewUpgradeResult`; `refused` externally decided (Unknown rule versions, unverified history and unsupported transitions are refused.), error `ekr.kernel.KnowledgeRefused` |
| command behaviour | `ekr.kernel.Propose` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.Propose` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `proposed` otherwise, creates `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionProposed`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `malformed` externally decided (strict document parsing or structural admission refuses before recording), error `ekr.kernel.StructurallyInvalid`; `misattributed` externally decided (the document names a proposer other than the trusted submitter, or the submitter is unregistered), error `ekr.kernel.ProposalAttribution` |
| command behaviour | `ekr.kernel.Seed` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.Seed` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `seeded` otherwise, creates `ekr.kernel.Revision`, emits `ekr.kernel.Seeded`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`, emits `ekr.store.CheckpointWritten`; `already-seeded` externally decided (a retained seed exists and its parsed full Seed2 input or actual BootstrapContext or trusted host AuthorityStateV1 anchor differs), error `ekr.kernel.AlreadySeeded`; `retained-seed` externally decided (a retained seed has the same full parsed Seed2 input, actual BootstrapContext and trusted host AuthorityStateV1 anchor), returns the exact retained result of `seeded` without errors, events, or subject changes; `invalid-seed` externally decided (deterministic bootstrap validation refuses the seed), error `ekr.kernel.InvalidSeed` |
| command behaviour | `ekr.kernel.ShowAttention` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.ShowAttention` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.kernel.ShowAttentionResult`; `refused` externally decided (Refuse an attention subject whose kind and supplied identity disagree or whose source does not exist.), error `ekr.kernel.KnowledgeRefused` |
| command behaviour | `ekr.kernel.Snapshot` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.Snapshot` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `taken` otherwise, emits `ekr.kernel.SnapshotTaken`; `not-found` externally decided (no revision carries the requested number), error `ekr.kernel.RevisionNotFound` |
| command behaviour | `ekr.kernel.Validate` | kept an obligation by a typed response (`response:`) | given `ekr.kernel.Validate` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `validated` otherwise, takes `validate` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionValidated`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `rejected` externally decided (at least one deterministic validator raised an issue against the named revision), takes `reject` of `ekr.kernel.GraphTransaction`, emits `ekr.kernel.TransactionRejected`, emits `ekr.store.PublicationPrepared`, emits `ekr.store.ObjectStored`; `revision-not-found` externally decided (the transaction is Proposed and no committed revision carries input.against), error `ekr.kernel.RevisionNotFound`; `transaction-not-found` externally decided (no retained transaction carries input.transaction_id), error `ekr.kernel.TransactionNotFound`; `wrong-state` from a state no declared move starts in, error `ekr.kernel.TransactionStateConflict`, and for an instance no record carries, without the error's fields |
| command behaviour | `ekr.observe.ImportObservation` | kept an obligation by a typed response (`response:`) | given `ekr.observe.ImportObservation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `answered` otherwise, emits `ekr.observe.ImportObservationResult`; `refused` externally decided (Payload digest, source key coherence, immutable identity and idempotency are checked against retained records; a reused identity with other bytes is refused.), error `ekr.observe.KnowledgeRefused` |
| command behaviour | `ekr.observe.ListObservations` | kept an obligation by a typed response (`response:`) | given `ekr.observe.ListObservations` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `listed` otherwise, emits `ekr.observe.ObservationsListed` |
| command behaviour | `ekr.observe.ShowObservation` | kept an obligation by a typed response (`response:`) | given `ekr.observe.ShowObservation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `shown` otherwise, emits `ekr.observe.ObservationShown`; `refused` externally decided (An unknown observation or missing/mismatched retained bytes is refused before answering.), error `ekr.observe.KnowledgeRefused` |
| command behaviour | `ekr.views.ChangesSince` | kept an obligation by a typed response (`response:`) | given `ekr.views.ChangesSince` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `listed` otherwise, emits `ekr.views.ChangesListed`; `since-malformed` when `(since_kind == Revision and since < 0)`, error `ekr.views.SinceMalformed`; `limit-exceeded` when `(limit < 1 or limit > 2000 or after < 0)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number, as at or as since_revision), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.DescribeNode` | kept an obligation by a typed response (`response:`) | given `ekr.views.DescribeNode` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `described` otherwise, emits `ekr.views.NodeDescribed`; `node-not-found` externally decided (the requested revision holds no node with the requested id), error `ekr.views.NodeNotFound`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.DrawFactSample` | kept an obligation by a typed response (`response:`) | given `ekr.views.DrawFactSample` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `drawn` otherwise, emits `ekr.views.FactSampleDrawn`; `limit-exceeded` when `(size < 1 or size > 1000)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.ExpandNeighbourhood` | kept an obligation by a typed response (`response:`) | given `ekr.views.ExpandNeighbourhood` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `expanded` otherwise, emits `ekr.views.NeighbourhoodExpanded`; `limit-exceeded` when `(depth < 0 or depth > 2 or limit < 1 or limit > 2000 or edge_limit < 1 or edge_limit > 5000 or after < 0)`, error `ekr.views.LimitExceeded`; `node-not-found` externally decided (a seed names a node the requested revision does not hold), error `ekr.views.NodeNotFound`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.ExportOcel` | kept an obligation by a typed response (`response:`) | given `ekr.views.ExportOcel` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `exported` otherwise, emits `ekr.views.OcelExported`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded`; `event-type-not-found` externally decided (the revision's ontology holds no node type with a name the request lists in events), error `ekr.views.EventTypeNotFound`; `event-time-invalid` externally decided (an event_time selector is malformed, absent, ambiguous or conflicting, or a selected node holds multiple distinct timestamps), error `ekr.views.EventTimeInvalid` |
| command behaviour | `ekr.views.FindCodeNames` | kept an obligation by a typed response (`response:`) | given `ekr.views.FindCodeNames` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `found` otherwise, emits `ekr.views.CodeNamesFound`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.ProjectGraph` | kept an obligation by a typed response (`response:`) | given `ekr.views.ProjectGraph` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `projected` otherwise, emits `ekr.views.GraphProjected`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.ProjectOverview` | kept an obligation by a typed response (`response:`) | given `ekr.views.ProjectOverview` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `overviewed` otherwise, emits `ekr.views.GraphOverviewed`; `limit-exceeded` when `(limit < 1 or limit > 500)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.ProjectTimeline` | kept an obligation by a typed response (`response:`) | given `ekr.views.ProjectTimeline` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `timelined` otherwise, emits `ekr.views.SubjectsTimelined`; `limit-exceeded` when `(hops < 1 or hops > 3 or limit < 1 or limit > 500)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.ReportFactQuality` | kept an obligation by a typed response (`response:`) | given `ekr.views.ReportFactQuality` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `reported` otherwise, emits `ekr.views.FactQualityReported`; `limit-exceeded` when `(confidence < 1 or confidence > 9999)`, error `ekr.views.LimitExceeded`; `judged-twice` externally decided (the judgements judge one assertion more than once), error `ekr.views.JudgedTwice` |
| command behaviour | `ekr.views.ReportStoreQuality` | kept an obligation by a typed response (`response:`) | given `ekr.views.ReportStoreQuality` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `reported` otherwise, emits `ekr.views.StoreQualityReported`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| command behaviour | `ekr.views.SearchNodes` | kept an obligation by a typed response (`response:`) | given `ekr.views.SearchNodes` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `searched` otherwise, emits `ekr.views.NodesSearched`; `limit-exceeded` when `(limit < 1 or limit > 100)`, error `ekr.views.LimitExceeded`; `not-found` externally decided (the store holds no committed revision with the requested number), error `ekr.views.RevisionNotFound`; `not-seeded` externally decided (the store has never been seeded), error `ekr.views.NotSeeded` |
| view query | `ekr.graph.Assertions` | kept an obligation by an order over the `Timestamp` field `recorded_from`, which the suite ranks by its text and not by its instant | a query answering `ekr.graph.Assertions` with rows projected from `ekr.graph.Assertion` at `read_your_writes` consistency |
| view query | `ekr.kernel.Rejections` | kept an obligation by a view parameter (`params:`), which a generated query does not apply | a query answering `ekr.kernel.Rejections` with rows projected from `ekr.kernel.ValidationIssue` at `read_your_writes` consistency, containing instances where `((not (defined(param.from)) or against >= param.from) and (not (defined(param.to)) or against <= param.to))` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
| actor grants | `ekr.integrate.Applier` | planning | may invoke `ekr.integrate.ApplyExtraction`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.integrate.HumanReviewer` | planning | may invoke `ekr.integrate.ApproveSchemaProposal`, `ekr.integrate.RejectSchemaProposal`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.integrate.KnowledgeProposer` | planning | may invoke `ekr.integrate.ApplySchemaProposal`, `ekr.integrate.DiscoverSchemaGaps`, `ekr.integrate.ImportInterpretation`, `ekr.integrate.ListInterpretations`, `ekr.integrate.ShowInterpretation`, `ekr.integrate.ShowSchemaProposal`, `ekr.integrate.SubmitSchemaProposal`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.kernel.Committer` | planning | may invoke `ekr.kernel.Commit`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.kernel.HumanOperator` | planning | may invoke `ekr.kernel.AnswerAttention`, `ekr.kernel.ApplyUpgrade`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.kernel.KnowledgeReader` | planning | may invoke `ekr.kernel.ListAnswers`, `ekr.kernel.ListAttention`, `ekr.kernel.PreviewUpgrade`, `ekr.kernel.ShowAttention`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.kernel.Operator` | planning | may invoke `ekr.kernel.Explain`, `ekr.kernel.Seed`, `ekr.kernel.Snapshot`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.kernel.Proposer` | planning | may invoke `ekr.kernel.Propose`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.kernel.Validator` | planning | may invoke `ekr.kernel.Validate`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.observe.KnowledgeSupplier` | planning | may invoke `ekr.observe.ImportObservation`, `ekr.observe.ListObservations`, `ekr.observe.ShowObservation`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `ekr.views.Reader` | planning | may invoke `ekr.views.ChangesSince`, `ekr.views.DescribeNode`, `ekr.views.DrawFactSample`, `ekr.views.ExpandNeighbourhood`, `ekr.views.ExportOcel`, `ekr.views.FindCodeNames`, `ekr.views.ProjectGraph`, `ekr.views.ProjectOverview`, `ekr.views.ProjectTimeline`, `ekr.views.ReportFactQuality`, `ekr.views.ReportStoreQuality`, `ekr.views.SearchNodes`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
