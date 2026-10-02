---
format: aep.planning-md/3
id: release-plan:knowledge-inbox-schema-learning
kind: release-plan
status: draft
title: Knowledge inbox and evidence-led schema learning
relations:
- derived_from: epic:p2-observation-layer
- derived_from: epic:p3-incubation-integration
- derived_from: epic:p4-operator-surface
- derived_from: epic:p5-frontier-schema-scheduler
revision: 2
---
## Intent and authorization

Operator-approved execution plan: knowledge inbox and evidence-led schema learning. The operator requested implementation and verification of the supplied plan on 2026-10-03. ESS is the contract authority; AEP records delivery, dependencies and evidence. This release plan is work in progress, not a claim of delivered behavior.

## Milestones and ordering

A: retain supplied observations independently of revisions and retain typed interpretations, including unmapped knowledge. B: explicitly upgrade existing stores, detect overlapping differing One claims, mark both disputed and project an evidence-backed inbox. C: validate human claim selection, retraction and temporal correction, preserving evidence/history and rechecking the answer basis. A then B then C delivers the inbox.

D reuses story:schema-transaction-cites-evidence after authority/evidence foundations. E uses A to group recurring integration gaps and accept typed consumer-agent proposals without canonical mutation. F depends on C, D and E: exact human approval, additive schema first, ordinary validated mapped-fact transactions second, resumable receipts without duplication.

Each implementation story must follow validated ESS entities/relations and have an exact scope before dispatch. Shared kernel, CLI, contracts and gate changes are serialized.

## Decisions

Observations have append-only retention through ekr-store independently of canonical revisions, using existing idempotency keys. Rejection of an interpretation retains its sources; cited bytes are pinned. Incubation reuses GraphRoot/TransientGraph and immutable interpretation versions. Roots reference observations; they do not own them. Canonical derivations depend on retained admissible evidence and immutable mapping records, never live incubation roots.

Attention is a projection over disputes, blocked integration and schema proposals, not another queue database. Both competing claims are disputed and excluded from settled projections. Equal values, disjoint times and Many declarations do not conflict. Human answers may choose, retract or correct effective time; uncertainty may remain. Unrelated store changes permit answer revalidation; changed evidence, options or effects require renewed review.

Trusted operator identity, exact proposal digest and reviewed evidence bind approval. Agent content cannot approve itself. Schema additions are new types/relations and optional properties; mappings copy declared typed source fields/relations or explicit constants. No arbitrary transformations, enum exhaustion inference or silent fact rewriting. Consumer-supplied agents interpret; the runtime groups and validates. No embedded provider or scheduler.

Existing stores require a versioned authority transition with a preview, stale refusal before mutation, unchanged seed anchors/historical bytes and original historical replay rules. Pending validations must be revalidated after transition.

## Public interfaces

CLI/typed SDK writes; read-only Rust-rendered viewer inbox and proposal pages. Command groups: observe import; incubate import/list/show; attention list/show/answer; schema-proposal discover/submit/show/approve/reject/apply; upgrade preview/apply. Existing document-schema command ekr schema remains compatible. Browser and MCP writes are excluded.

## Generation prerequisite

The baseline validates. The retained synthesis reports in .engineering/reviews/knowledge-inbox-prerequisite/ reproduce invalid-identifier wire labels and recursive-layout cycles with both the pinned and installed ESS releases. Resolve the upstream generator, preserve serialized labels, pin the verified release and gate regeneration before new runtime contracts are generated. Hand transcription is not authorized.

## Acceptance

Run these named cases on file and SQLite providers with reopen and full replay: observation_retry_is_idempotent; rejected_interpretation_retains_its_sources; unmapped_knowledge_survives_reopen; overlapping_single_value_claims_become_disputed; equal_disjoint_and_many_claims_do_not_conflict; human_resolution_preserves_evidence_and_history; changed_answer_basis_requires_review; upgrade_preserves_historical_rules_and_hashes; schema_proposal_requires_exact_human_approval; schema_change_exposes_supporting_evidence; interrupted_integration_resumes_without_duplicates; canonical_derivation_has_no_transient_dependency.

Demonstrate ownership clarification with approved operational/business relations and preserved earlier history. Demonstrate parked project health observations becoming reviewed vocabulary and mapped facts, with a repeat producing no changes. Require generated drift checks, real implementation conformance, upgrade/crash recovery, CLI/SDK examples, viewer inspection and task check; retain actual reports and specification digests. Validation alone does not prove conformance.

## Preserved scope and exclusions

Keep broader observation-retention/general migration stories open until their full acceptance holds. Record the retention decision against decision-blocker:observation-retention-path; unrelated polling, checkpoints, entity merge and constraint-language decisions remain open. Exclude live connectors, autonomous scheduling, destructive general migration, entity merge/split, retention/GC and separately planned hosted PostgreSQL. Persist through the existing store abstraction.

## Prior review provenance

The supplied plan says four critic perspectives were examined locally and spawning was refused: this was not an independent panel. No individual findings or verbatim reports were supplied to this execution session, so none are fabricated. The approved boundaries above preserve the dispositions carried by that plan; independent implementation review remains required.

## Upstream issue

Exact baseline synthesis failures are recorded in https://github.com/beyond10x/ess/issues/400, created and read back as b10x-bot[bot]. The retained JSON reports preserve the complete diagnostics and source/contract digests for both releases. Local assessment must verify the existing enum name/wire idiom before proposing new naming syntax.
