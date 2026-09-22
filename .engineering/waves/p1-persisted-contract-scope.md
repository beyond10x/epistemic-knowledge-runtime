# Persisted-contract scope before the durable writer

Read-only scope preparation, 2026-09-22. The integration tree reached `bc46dc8e850d68a54bdede640ecc8ee1a10e8576` during inspection. No repository files were changed and no builds ran. Proposed formats below are recommendations for the coordinator to record before dispatch, not already approved compatibility mappings.

## Artifact inventory

- `task:assertion-retraction-erases-its-acceptance`: draft; blocks `story:commit-and-revision-lineage`.
- `task:two-revision-events-have-no-discriminator`: draft; blocks the same story.
- Validation-hash domain repair has **no dedicated task found**. It is recorded in `review-result:independent-review-p1-core` at `transaction.rs:224` and preserved as `.engineering/reviews/p1-baseline-209dd5e/cases/review_p1_validation_hash_domain.rs`. Create a scoped task before dispatch; do not repurpose implemented `task:canonical-newtype-discriminant`, which decides sum-type tags and structural newtypes.

The related projection tasks remain live: `graph-domain-carries-validation-state-payloads`, `typed-value-canonical-field-is-undecided`, `store-snapshot-and-its-id-are-declared-not-implemented`, and `two-of-the-five-revision-sub-roots-are-placeholders`. They must not be closed merely because the three changes below land.

## Current contracts that already exist

1. `GraphDocument` is unversioned JSON containing root, revision, nodes, edges, assertions and evidence. It omits ontology. `ObjectStore::put` addresses its exact bytes under `ekr.payload.v1`.
2. `Assertion.validation` is Serde's externally tagged enum. Accepted carries validators; Retracted carries revision/reason; Superseded carries replacement. Retraction or supersession therefore removes the only acceptance field. Canonical assertion encoding writes its ten fields in declaration order.
3. `RevisionEvent` uses the internal JSON tag `event`, six variant names and canonical indices 0–5. Rejected and Stale have no occurrence identity. Proposed/Validated also lack occurrence identity; their content can repeat.
4. The adapter writes every revision event as eventlog `schema_version = 1`. Append idempotency is the request hash of the JSON body. The revision stream is `ekr.revision/canonical`, scoped by tenant.
5. `read_all` discards eventlog's event ID, stream position, name and schema version, keeping only JSON data. Migration and explicit version dispatch need those recorded fields retained.
6. Validation hashes concatenate canonical transaction bytes and revision bytes, then incorrectly call `ContentHash::of_bytes`. The correct public hash boundary is `ContentHash::of` for canonical values. Do not change either global hash-domain label to repair one caller.
7. Knowledge roots hash nodes, edges and assertions; changed assertion encoding changes knowledge roots and their descendant parent hashes. Existing log events do not retain operation payloads or durable kernel validation receipts. A missing payload cannot be recovered from its digest.

Normative basis: original design §§17, 34, 36 and 72; ADR 0007 on the kernel publication boundary; implemented `canonical-newtype-discriminant` on encoding tags. The assertion task explicitly requires a dated design amendment.

## Retraction unit

### Scope

Derived 2026-09-22 by story-scoper.

- **Primary:** `crates/ekr-graph/src/assertion.rs` — cited; ValidationState, AssertionStatus, status(), is_current() and canonical encoding.
- **Queries:** `crates/ekr-graph/src/snapshot.rs` — cited; valid_at currently excludes all superseded/retracted assertions through is_current.
- **Wire/encoding regressions:** `crates/ekr/tests/graph_assertion_serde.rs`, `crates/ekr-graph/tests/canonical_value_and_assertion.rs`, `crates/ekr-graph/tests/snapshot_reads.rs`, `crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` — cited.
- **Projection:** `systems/ekr/domains/graph.yaml` and `crates/ekr-graph/tests/domain_projection.rs` — cited.
- **Proposal conversion:** `crates/ekr-kernel/src/transaction.rs`, `crates/ekr-kernel/src/validate/provenance.rs` — cited; copying and admitting assertion fields must preserve/restrict new lifecycle data.
- **Stored conversion:** `crates/ekr-store/src/snapshot.rs` — cited; current widen/narrow helpers copy assertion fields, but seed work may move this responsibility into kernel before dispatch.
- **Design:** `docs/epistemic-knowledge-runtime-design.md` — cited; append an amendment, retain original sections.
- **Fixture fallout:** assertion literals in graph, kernel and store tests — inferred; mechanical new-field initialization, with no relaxation of existing behavioral assertions.
- **Confidence:** high — cited; the task and current encoders name the exact conflict.
- **Would collide with:** seed document conversion, graph assertions, kernel assertion admission, serialization fixtures, and persisted-format migration — cited.

Recommended amendment: store validation and lifecycle independently. Keep the five assessment states Proposed/Validating/Accepted/Rejected/Disputed; move Superseded and Retracted solely into a stored AssertionStatus. Acceptance validators remain unchanged when lifecycle changes. New proposals carry Proposed plus Active only. Existing Active means “not withdrawn”, not “accepted”; keep read eligibility separately explicit.

Keep retained evidence IDs and acceptance validators in the current record as well as in historical revisions. Retraction records reason and revision; supersession records replacement and its supplied valid-time boundary. This unit defines/serializes the state and changes read eligibility; the writer unit creates the actual lifecycle transactions.

Acceptance:
- Accepted → Retracted and Accepted → Superseded retain the exact validator/evidence sets through JSON round-trip and canonical hashing.
- Modifying either lifecycle payload or acceptance provenance changes the assertion hash.
- Proposed/Rejected records remain unreadable as canonical beliefs.
- A proposal cannot supply Accepted or a non-Active lifecycle to evade kernel admission.
- An earlier revision still returns the earlier assertion; a latest revision excludes retracted beliefs.
- Preserve legacy test vectors separately; do not regenerate them using the new types and call that migration evidence.

**Query contract to resolve explicitly:** `snapshot_reads.rs::valid_at_never_returns_a_retracted_or_superseded_assertion` and snapshot.rs say superseded records never answer even for an earlier valid time. The approved writer/CLI acceptance instead asks the latest snapshot before/after a supplied handover boundary to return the old/new belief. Recommended amendment: latest-revision valid-time reads include still-supported superseded facts inside their closed valid interval; exclude retracted beliefs. Reconstruction at an earlier revision remains a separate axis. Update the old negative test deliberately, not by weakening it accidentally.

## Event-occurrence unit

### Scope

Derived 2026-09-22 by story-scoper.

- **Vocabulary:** `crates/ekr-graph/src/events.rs`, `systems/ekr/domains/kernel.yaml` — cited.
- **Persistence:** `crates/ekr-store/src/eventlog.rs`, `crates/ekr-store/src/log.rs` — cited; append identity, metadata preservation, version routing and replay.
- **Publisher:** `crates/ekr-kernel/src/commit.rs` — cited; construct event occurrence once, preserve it over retries.
- **Identity:** `crates/ekr-core/src/identity.rs` and `crates/ekr-core/src/lib.rs` — inferred; a dedicated event-occurrence ID avoids calling a non-committing refusal a RevisionId.
- **Contract tests:** `crates/ekr-graph/tests/revision_events.rs`, `crates/ekr/tests/graph_events_serde.rs`, `crates/ekr-store/tests/adversary2_event_vocabulary.rs`, `crates/ekr-store/tests/providers.rs`, `crates/ekr-store/tests/fold_rules.rs`, `crates/ekr-store/tests/lineage/mod.rs` — cited.
- **Confidence:** high — cited; adapter hashing and repeated-proposal fold semantics are directly visible.
- **Would collide with:** seed initialization, revision append/fold ports, commit publication, core identity projection and migration — cited.

**Narrowing correction:** adding a discriminator only to Rejected/Stale is insufficient. After rejection removes pending state, re-proposing the same transaction and operations produces the same Proposed bytes, which append deduplicates. Revalidating unchanged content against the same revision likewise repeats Validated. The same-occurrence retry and a later occurrence must differ for every repeatable event.

Recommended format: a versioned revision record wrapping the existing six event payloads, with a dedicated EventId allocated by the kernel per occurrence. Keep event payload variant indices 0–5 stable. Serialize the envelope as `{format: "ekr.revision-event/2", event_id, payload}` and write eventlog schema version 2. The payload keeps its existing event discriminator. Include the occurrence ID in canonical event-record hashing.

A retry reuses the exact record and occurrence ID. A new rejection/revalidation gets a new one. Use event identity as the idempotency key and the complete record as the request hash: same ID plus changed payload must refuse, not append another fact. Never derive occurrence identity from stream position or generate it afresh inside every append retry.

Acceptance over both providers:
- Two separate equal-content rejection/stale occurrences are both Written and survive reopen.
- Repeating the same record returns AlreadyRecorded and adds nothing.
- Reusing one occurrence ID with changed content refuses.
- Propose → validate → reject → same-content propose → validate → reject leaves no committable pending transaction.
- Lost-acknowledgement retry preserves one physical event.
- Unsupported record versions refuse before fold; declared version and payload shape must agree.

## Validation-address unit

Minimal source scope is `crates/ekr-kernel/src/transaction.rs:217` (cited), with `crates/ekr-kernel/tests/validation.rs` and the archived hash-domain case (cited). A private canonical wrapper over transaction plus validation basis is inferred; implement Canonical for it and call ContentHash::of. No public constructor or Deserialize for ValidatedTransaction is added.

Keep deterministic same-input hashes and transaction/revision sensitivity. Add a positive equality assertion against the canonical wrapper's value-domain hash, as well as the archived inequality against the payload-domain hash. Merely changing the hash arbitrarily would satisfy the archived inequality.

The wire-format decoder must identify the old versus corrected validation-address scheme explicitly. Do not globally change ekr.payload.v1/ekr.value.v1 and do not silently recompute old recorded hashes under the new algorithm. Full root-hash validation basis belongs to the durable writer; if its shape is changed here too, record that additional format change explicitly.

## Preservation-first format and migration boundary

Recommended new graph document envelope: `{format: "ekr.graph-document/2", graph: ...}`, with the new assertion representation. Coordinate its placement with the seed unit; do not independently add a conflicting wrapper. The new version selects the matching canonical encoder and validation-address scheme. Reject unrecognized formats and extra semantic fields.

Keep exact legacy decoders/encoders for inventory and historical verification, distinct from normal new-format ingestion. Unversioned documents and eventlog schema-version-1 records are legacy, never interpreted with new defaults. Do not use serde defaults to invent Active status, validators, event identities or acceptance history.

The next bounded unit can deliver format detection, a read-only migration inventory, legacy hash verification, and named refusal of unsupported migration. **It must not promise successful whole-store conversion before operation replay exists.** The actual verified migration/cutover is owned jointly with the durable writer and must block that writer's acceptance.

Migration requirements:
- Read source stores without writes; use a distinct destination. Preserve original objects, events, recorded metadata and hashes.
- Verify every source digest/lineage claim using its original format before conversion.
- Produce an explicit old→new object/revision address map and source event-coordinate→new occurrence identity map; preserve stable domain object IDs.
- Freeze generated occurrence IDs in the migration manifest before writing, so retries produce the same destination. Existing eventlog event IDs can be cited as source identity; never invent events deduplicated out of the old log.
- Legacy Accepted validators can be carried exactly. Legacy Retracted/Superseded has no prior acceptance set: recover only from verifiable earlier revisions/receipts. Otherwise refuse migration with the exact assertion ID and missing evidence.
- Legacy transaction hashes cannot be expanded into operations. Missing operations, ontology history or kernel validation evidence causes explicit refusal. Reinterpreting a seed snapshot as the missing history is forbidden.
- Only switch to a destination after both backends' readers verify mapped history and restart equivalence. On refusal, leave source usable and destination unpublished; report what evidence is missing.

Tests: frozen legacy documents/events, valid acceptance metadata preservation, unrecoverable retraction refusal, missing-operation refusal, corrupt hash refusal, unknown version refusal, interrupted migration resume, unchanged source-byte inventory, and restart verification of a fully evidenced fixture.

## Outstanding decisions before dispatch

1. Record the assertion amendment and its superseded-valid-time query rule.
2. Adopt an occurrence envelope for all repeatable revision facts, rather than the task's incomplete two-field patch; add the EventId ESS projection and update task scope.
3. Create a dedicated validation-domain task and bind it to versioned receipts/migration.
4. Agree the seed/graph envelope boundary with the seed implementor.
5. Establish which legacy source stores and authoritative auxiliary payloads actually exist. None were inspected here. Do not claim a supported conversion for unavailable history.
6. Scope the Rust migration tool through kernel APIs without introducing an xtask→store dependency that violates ADR 0007. The existing xtask only implements Doctor; its dependency/command additions require coordinator-owned scope changes.

No decision above authorizes discarding data, rewriting source history, using a substitute validation authority, or restoring from a snapshot as if it were complete history.

## Scope-entry commands for the coordinator

These commands were not run. The hash-domain unit has no existing artifact ID, so no command is supplied for an invented ID.

```sh
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-graph/src/assertion.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-graph/src/snapshot.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr/tests/graph_assertion_serde.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-graph/tests/canonical_value_and_assertion.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-graph/tests/snapshot_reads.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add systems/ekr/domains/graph.yaml
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-graph/tests/domain_projection.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-kernel/src/transaction.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-kernel/src/validate/provenance.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add crates/ekr-store/src/snapshot.rs
aep plan artifact scope task:assertion-retraction-erases-its-acceptance --add docs/epistemic-knowledge-runtime-design.md
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-graph/src/events.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add systems/ekr/domains/kernel.yaml
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-store/src/eventlog.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-store/src/log.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-kernel/src/commit.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-core/src/identity.rs --inferred
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-core/src/lib.rs --inferred
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-graph/tests/revision_events.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr/tests/graph_events_serde.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-store/tests/adversary2_event_vocabulary.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-store/tests/providers.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-store/tests/fold_rules.rs
aep plan artifact scope task:two-revision-events-have-no-discriminator --add crates/ekr-store/tests/lineage/mod.rs
```

## Coordinator verification of scope commands

The installed AEP CLI refuses typed scope on tasks: scope belongs to story, and a task inherits
its owning story's surface. The commands above are the scoper's unexecuted recommendations.
Typed scope is recorded on story:version-persisted-contracts instead. The validation-domain
finding now has task:validation-hash-uses-payload-domain. No source format is changed yet.
