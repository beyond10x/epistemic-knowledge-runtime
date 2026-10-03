# F contract proposal — design only, not validated or implemented

## Atomic protocol (no upstream Eventlog change)

Use one per-proposal review/publication stream whose events are a closed union of
`ProposalReviewRecorded` and `ApplicationPublicationRecorded`. Every review append and every
application canonical publication compare/exchange the physical stream sequence. An application
publication adds a real marker event in the SAME native BlobAppendGroup as the ordinary revision
event, object metadata and blobs. There is no empty guard entry and no reservation granting a
future write. The marker is evidence of that exact publication, not another human decision.

The stream fold tracks physical sequence separately from effective human proof. Marker events
advance sequence but do not change the latest human decision or the expected_previous_decision
proof digest. E retries a physical conflict, rechecks the latest actual human decision, and can
retain the same signed decision when intervening events are merely application markers.

A stale prepared native request contains its original review-stream Expected::Exact. Rejection
therefore defeats its publication. If that exact request already committed before rejection,
provider idempotency returns its old result; this is receipt recovery, not a new post-rejection
write. Resolve uncertain results before minting or electing any replacement attempt.

## Minimal named types / entity additions

All fields below belong in ESS first; generated wire and semantic models are required. Existing
retained semantic concepts are projected, not copied into handcrafted Rust models.

1. `ApplicationItemKey` struct:
   - source: InterpretationVersion
   - item: String (facts[index])
   - mapping_digest: kernel.ContentHash
   Use it for every remaining_items field in ApplicationReport, ApplicationReceiptSnapshot,
   ApplicationReceipt and ApplicationReceiptRecords. The digest includes mapping.source and the
   exact typed mapping; define its canonical bytes once and pin them. No live root reference.

2. `ApplicationStep` tagged union: Schema (unit), Mapping(ApplicationItemKey), Corrections(unit).
   The correction step covers the ordered proposal.corrections list, including replacements.
   An empty correction list elects no correction step. Do not silently fold corrections into
   arbitrary per-fact iterations. Ordering is schema, all elected selected mappings successfully
   committed, then one atomic corrections transaction. Unresolved mappings cannot be dropped to
   reach corrections. After that final commit no canonical work remains, only receipt recovery.

3. `SchemaApplicationId` UUID newtype; `ApplicationElection` entity, identity application_id:
   proposal_id, proposal_digest, initial_review_id, initial_proof_digest, base_schema,
   elected_at, schema_transaction: CanonicalTransactionProjection,
   selected_items: List<ApplicationItemKey>.
   Unique per store/proposal_id/exact proposal_digest; a newer approval of the SAME immutable
   proposal resumes this election rather than minting a second schema transaction. Initial review
   documents election; it is never permanent authority for later writes. The retained ordinary
   schema transaction fixes transaction/schema/type/property/relation/evidence IDs and operation
   bytes before any publication. Relations: proposal, initial_review; schema transaction is a
   planned document at election, NOT yet a mandatory reference to a retained GraphTransaction.
   Named `RetainedApplicationElection` carries the same fields for storage and replay.

4. `ApplicationStepElection` entity, identity step_election_id: Uuid:
   application_id, step: ApplicationStep, transaction: CanonicalTransactionProjection,
   replacements: List<kernel.ClaimReplacement>, mapping_ids: List<MappingRecordId>,
   derivation_ids: List<Uuid>, elected_at.
   Unique by application_id + exact step key. The transaction is the initial attempt and immutable
   operation/allocation template, not a promise that a terminal stale transaction can be revived.
   Frozen operation bytes bind allocated assertions and evidence; replacements bind prior to new assertion IDs. Every step is elected
   BEFORE Propose/Validate/Commit, atomically with its pinned material. The schema step must equal
   ApplicationElection.schema_transaction. Other steps elect once when applicable. Unresolved
   mappings remain qualified pending items and have no fabricated transaction. Named retained
   projection required. Relation: application; no reference to a transaction before it exists.

   `ApplicationStepAttempt` entity and `RetainedApplicationAttempt` project separately elected
   transaction attempts, identified by transaction_id: step_election_id, optional predecessor
   transaction and predecessor record hash, full transaction projection, and elected_at. Initial
   attempts have no predecessor. A successor requires a verified terminal Stale predecessor and
   resolves every uncertain predecessor publication first. Only the transaction identity changes;
   schema, assertion, evidence, mapping, replacement and derivation identities and operation bytes
   remain the elected template. Preserve the ordinary Proposed/Validated/Stale lifecycle and
   immutable validation input binding. Receipts reference the actual committed attempt.

5. `ApplicationPublicationGuard` struct:
   application_id, step_election_id, proposal_id, proposal_digest, review_id, human_proof_digest,
   review_stream_version: Integer, step: ApplicationStep, attempt_transaction: TransactionId.
   `ApplicationPublicationRecord` struct adds transaction_id, event_id, record_hash and
   publication kind (reuse store.PublicationCommandKind where suitable). Its exact record is the
   nonempty shared-stream marker. It binds a real ordinary canonical occurrence, not a promise.
   Effective review is reconstructed from retained, verified stream prefix; client JSON carrying
   a guard has no authority. Eventlog stream coordinates are derived from store/tenant/proposal,
   never accepted as an arbitrary target stream supplied by a client.

6. Named `RetainedMappingRecord` projects ALL current MappingRecord entity fields, including
   mapping_id, proposal_digest, mapping_digest, source_document_digest, mapping, evidence.
   Named `CanonicalDerivationRecord` projects ALL current CanonicalDerivation fields. Preserve
   existing relations to retained proposal/source/mapping bytes, observations and graph Evidence.
   Retain these plus actual admissible evidence independently at Provenance or stronger. Canonical
   replay must not load a live Interpretation entity or TransientGraph to justify an assertion.

7. ApplicationReceipt and its snapshot add application_id and optional step_election_id where
   describing a step. Add application relation. Qualified items as above; schema_revision becomes
   Optional<RevisionNumber>, present only after verified schema commit; add Elected progress.
   ApplicationReport likewise allows no schema revision yet. IMPORTANT: if Elected receipts are
   retained before Propose, the existing mandatory schema_transaction -> GraphTransaction relation
   is invalid; keep ApplicationReceipt post-schema-commit and expose election progress separately
   in Show/report, or explicitly model the planned-vs-retained transaction distinction. Prefer the
   former for the smallest patch: retain nonoptional committed receipt fields and put Elected plus
   optional schema revision only in the public election/application report. ProcessingReceipt's existing
   mapping_digest + document_digest + item remain the exact idempotency scope; a success for one
   mapping never resolves another mapping for the same source item. Progress records are append
   only, with distinct receipt IDs, and reference confirmed ordinary transactions.

## Store declaration delta

Add optional `review_guard: ApplicationPublicationGuard` to store.Publication, absent from ALL
historical encodings. Allocate publication-preparation /7 after the existing signed-publication /6;
only guarded publications use it. Its native_request carries revision + exact marker + object appends.
Historical /1–/6 validation and bytes remain unchanged. Extend preparation validation to permit
EXACTLY the typed marker stream/event under this guard, rather than arbitrary extra appends.
Authorize both the existing revision and marker's proposal/review/application/transaction links
through the existing kernel CommitAuthority. Existing Propose/Validate/Commit families can remain;
the per-step transaction ID already supplies their stable recovery key. New ordinary attempts
must retain/derive the guard; an unguarded revalidation/commit cannot consume a marked application
transaction and bypass rejection.

Expose retained proposal coordination history through the store's verified read interface so
KernelAuthority can verify the effective review at the marker's historical prefix. Extend
RetainedHistory/object requirements with the closed typed records actually needed, preserving
old history handling. The provider must also check one-to-one marker/canonical occurrence links
on full replay. A marker is not valid merely because it can deserialize.

## Existing source seams and required behavior

- ekr-store schema_proposals.rs: independent proposal retention. E review retention must fold the
  shared stream union and use native sequence, not number of review records, for Expected::Exact.
- ekr-store preparation.rs::native_for: add the typed nonempty marker to the elected atomic group.
- preparation.rs::authorize_preparation already authenticates the mandatory audience-wide human
  identity append for /6. Preserve that closed rule; /7 admits exactly its typed application marker
  alongside ordinary revision/object appends. Any composition with a human identity append must be
  explicitly declared and validated, never accepted as an arbitrary extra stream.
- preparation.rs::resume_preparation retries exact native bytes, which is correct for response
  loss provided the marker was included when elected. Never attach a guard only at resume time.
- ekr-kernel commands.rs::drive: conflicts reload both canonical state and proposal coordination
  state; renewed latest approval and material checks precede a successor preparation.
- Existing pending transaction recovery must require its original application linkage; ordinary
  generic commit must not provide an escape hatch around the guard.
- Kernel reviewed-correction admission must reuse the existing correction validator with the
  SchemaReviewTarget authorization context. Do not fabricate an AttentionAnswer signature or
  permit ordinary unsigned retirement of disputed claims.

## Two application-specific semantic traps to specify

Own progress changes the state: E candidate schema rejects already-declared additions, and its
review basis changes when schema/corrections are applied. F must recognize and verify the exact
already-committed prefix of THIS election, then compare remaining effects and evidence against
that approved plan. It must not re-run initial whole-proposal preview blindly and demand a new
review after its own first commit. Conversely, never attribute a coincidental external change
to this application merely because names/values match.

A signed new approval for the same proposal can permit continuation after rejection, but does
not duplicate the earlier schema or committed items. Each future marker binds the newly effective
review. Full replay evaluates historical authorization at each marker prefix; a rejection today
does not invalidate earlier approved commits.

E Show/Approve and historical review verification must share application-aware material for that
continuation. A new signature binds verified own-prefix references and the residual evidence and
effects, while preserving the immutable proposal and its selected corrections. The original
whole-plan applicability test is insufficient after committed schema/mapping steps. Corrections
remain pending until the final atomic step; once it committed, completion is derived from linked
canonical commits even if a reporting receipt was lost. A later rejection then permits only exact
receipt recovery/reporting, not new canonical work.

## Required failure probes

Approval read -> rejection append -> canonical commit must conflict/no new write. Canonical
publication winning before rejection remains recoverable. Crash before/after schema and item
atomic groups yields frozen IDs/no duplicates. Marker append and canonical append are all-or-none
on both providers. Marker insertion does not become a human predecessor. Generic commit cannot
bypass the application guard. An unrelated ordinary commit can revalidate without another human
answer by electing a successor after a verified stale attempt; changed reviewed effects require
review. Exercise validate-T, unrelated-commit-U, commit-T, including crashes around the stale result
and successor election. Preserve old /6 recovery as a compatibility control. Repeat after complete
returns original results.
