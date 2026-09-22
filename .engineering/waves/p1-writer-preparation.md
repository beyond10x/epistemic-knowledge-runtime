# Durable writer implementation preparation — 2026-09-22

This scopes `story:commit-and-revision-lineage`, revision 8, against coordinator commit
`a5485c0b120359876f398fca136d8d48579d6a8f` as printed during inspection. The seed implementation was merged
at `73ab8b0`. This is an implementation preparation report, not a new plan, approved schema,
implementation result or evidence of a successful migration. No repository/planning edits, builds,
provider opens, live-store access, commits or upstream changes were made. Only this report was written.

## Minimum useful first slice

The first vertical acceptance should create actual knowledge, persist the full submitted
transaction and validation basis, stop the process, and reconstruct that changed knowledge in a
new process using the real kernel. Use exactly the same test program against SQLite and File.
Start from a valid seed with a nontrivial ontology, retained HumanStatement evidence, and the
coordinator-approved populated authority state described below. Submit a transaction that creates
a node and an evidence-backed assertion referring to that node; include a Many property and a
single List-valued property. Put AddAssertion before CreateNode in one permutation so application
cannot accidentally implement operation-vector order as temporal order.

Propose must retain a real record; Validate must produce a real private capability and a durable
accepted receipt; Commit must apply to a copy, compute roots, and atomically publish that result
against the exact prior head. After a fresh process opens using the trusted bootstrap identity
context, a latest snapshot must contain the new node/assertion and every property member. The
recomputed Root must equal the original committed result, and its knowledge hash must differ from
the seed's. Transactions readback must retain the proposal, validator attribution, basis and final
result. Supplying another ontology at reopen must not replace retained ontology.

This first witness is small; the implementation is not allowed to silently skip another admitted
operation. Its applier must exhaustively handle the seven currently admissible operation variants,
or the pipeline must explicitly refuse an as-yet-unimplemented variant before sealing. Schema
evolution and entity merge keep their existing named P1 refusals. A temporary narrower applier is
an intermediate implementation state, not completion of the story. Retraction, supersession,
historical snapshots, stale decisions and interruption acceptance remain mandatory before closing
the existing story; the first changed-and-reopened graph is its first decisive milestone.

## What exists and what does not

| Surface | Implemented state | Writer consequence |
|---|---|---|
| `ekr-kernel/src/seed.rs` | Complete seed input, ontology/evidence retention, real deterministic bootstrap validation and replay admission. | Use this authority path; do not replace it with a permissive test authority. |
| `ekr-store/src/eventlog.rs:566` | Atomic seed object metadata + Seeded publication, with NoStream retained across contention retries. Object bytes still live in event bodies. | Preserve the seed no-losing-object contract while the coordinated blob migration repairs placement. |
| `ekr-kernel/src/commit.rs:277–330` | Reads head, compares revision number, emits Proposed/Validated/Committed separately; does not apply operations. | Replace the stub's publication and result behavior, not just its returned revision. |
| `ekr-kernel/src/commit.rs:61–117` | Accepted validations are an in-memory set. New process has none. | Persist inputs sufficient for independent kernel revalidation; a deserialized receipt is not a capability. |
| `ekr-kernel/src/transaction.rs:206–228` | Sealed type retains canonical transaction, revision number and hash only. Current hash uses the wrong payload domain. | Seal a complete immutable validation basis and actual validator attribution; pending format unit corrects the domain. |
| `ekr-kernel/src/validate/mod.rs:93–187` | Seven fixed deterministic validators; actor is supplied to Authorization; sealing discards that actor. | Retain exact suite/policy and actor, and use them on replay. |
| `ekr-kernel/src/validate/authorization.rs:32–50` | Named proposer/validator separation; proposer is not authenticated here. No capability/trust policy is implemented. | Trusted submission boundary binds caller identity; do not trust the proposal's claim or invent a P5 capability model. |
| `ekr-store/src/log.rs:16–33,318,483` | ontology_root and agent_root are zero placeholders. | Both are explicitly replaced where state exists; zero is not an empty-state hash. |
| `ekr-ontology/src/schema.rs:55–75,298–314` | Full OntologyDocument exists; loaded Ontology has private maps and lookup methods, no complete export iterator/document method. | Add a small complete export/canonical surface; hashing only used types or the schema ID is insufficient. |
| `ekr-graph/src/root.rs:50–64` | Root contains four sub-roots, revision, parent hash and transaction hash; no timestamp or occurrence identity. | Receipt/occurrence metadata must retain and bind the time and identity not present in Root. |
| `ekr-graph/src/canonical.rs:313–327` | CanonicalGraph has ontology, graph records and evidence; no Agent registry. | A populated agent root needs a real retained representation, not hashing currently nonexistent state. |
| `systems/ekr/domains/kernel.yaml:69–88` | Agent is specified as id/name/capabilities, but no Rust Agent exists. TrustProfile is not implemented. | Minimal authority state needs a coordinator-owned contract; do not present it as existing Agent support. |
| `ekr-store/src/eventlog.rs:296–319` | Only replay from seed is supported; it folds to latest, not to an earlier selected revision. | Add an explicit snapshot-at/reconstruct-to API. Renaming `replay(from)` does not change its semantics. |

Normative basis read: design §§18–21, 34, 56–58, 70–72; story's explicit P1 operation boundary;
the pending format amendment §§88–90; and the retained Transactions/document-handler contract in
`.engineering/waves/p1-conformance-document-contract.md`. The latter was synthesis evidence, not
runtime execution evidence.

## Decisions that must be written before first durable publication

1. **Agent root and deterministic policy.** Recommendation: retain a small `AuthorityStateV1`
   containing real registered actor records and the exact P1 validation profile, and define
   `agent_root = ContentHash::of(AuthorityStateV1)`. This extends Root's current "agent registry"
   description to include its operative validation profile; it needs an additive amendment/ESS
   declaration. A registry hash plus an unbound host-selected policy would not bind the rule used
   to admit the same transaction. The alternative permitted by the existing story is a defined
   empty registry root, but that does not fulfill a requirement for populated agent/validator state.
   The implementation must not silently choose that alternative here.
2. **Authority initialization.** Recommendation: trusted bootstrap configuration supplies the actual
   operator and validator records and profile; seed/2 retains the resulting authority document.
   Seed input cannot declare a permissive profile the host blindly adopts. Reopen checks the
   configured bootstrap trust anchor against retained state, then uses retained state for replay.
   P1 has no authority-registry mutation operation; changing it requires an explicit later version/
   transaction, not replacing a startup argument. Settle its seed-envelope/2 placement before that
   pending format is frozen; otherwise bump the seed/envelope version again.
3. **Timestamp authority.** Recommendation: trusted execution context supplies submitted_at,
   validated_at and committed_at once per occurrence. Application uses committed_at for assertion
   transaction time and the receipt retains it. Require committed_at >= previous committed_at
   and >= affected records' recorded_from; allow equal instants, with revision ordering breaking
   ties. Require submitted_at <= validated_at <= committed_at for accepted records, or explicitly
   relax that ordering if the host model permits skew. Replay never reads its clock. A failed/lost
   response retry reuses the frozen timestamp rather than choosing a new one.
4. **Binding time and context.** A full prior Root does not contain committed_at. Therefore the
   validation basis also names the previous seed/commit receipt hash and occurrence metadata, not
   just its Root hash. This prevents a caller from changing time context while claiming the same
   graph root. Define Root.transaction as the full accepted canonical transaction hash, not only
   the operations hash; proposer/evidence/transaction identity are part of the transaction.
5. **Durable record linking/version.** Revision event format 2 in the current draft has no receipt
   reference. The exact proposal below uses format/schema 3 with a mandatory record_hash. If the
   coordinator incorporates this into still-unpublished format 2 before freezing it, use that one
   chosen format consistently. Never add optional ignored metadata to published format 2 and call
   it the same strict schema.
6. **Proposal hashes before acceptance.** A structurally valid GraphTransaction<Value> can contain
   Float; there is deliberately no Canonical implementation for it. ESS presently requires an
   operations_hash even for Proposed/Rejected. Recommendation: retain an always-present submission
   payload hash and explicitly optional canonical transaction/operations hashes until narrowing
   succeeds. Update the view's meaning/types accordingly. Alternatively define an explicitly named
   payload-domain operations address for every submission, distinct from the canonical address.
   Do not manufacture a canonical hash or silently reject all noncanonical values at Propose just
   to satisfy the existing field. That would move a validation outcome to another command.
7. **Lifecycle operation wire shape.** RetractAssertion currently carries only AssertionId, while
   the pending lifecycle shape requires a reason; no Supersede operation exists. Specify a reason-
   bearing retraction and explicit replacement/effective-time supersession before encoding these
   operations. A fabricated default reason or hidden inference from AddAssertion does not satisfy
   the approved contract. This is a scoped transaction-format addition, not schema evolution.

## Proposed exact retained records

The following is an implementation-ready schema proposal, not the claim that these types exist.
Every record is strict and version-dispatched. All payload records live in retained addressed blobs;
events carry identities, hashes, counts and status only. Canonical value hashes use ContentHash::of;
blob addresses use ContentHash::of_bytes over exact stored bytes. Keep those labels unchanged.

### AuthorityStateV1

```text
format: "ekr.authority-state/1"
agents: BTreeMap<AgentId, {
    id: AgentId,
    name: String,
    capabilities: BTreeSet<String>
}>
validation_profile: {
    format: "ekr.p1-validation-profile/1",
    ruleset: "ekr.p1-deterministic/1",
    checks: [Structural, Reference, Type, Cardinality,
             OntologyConstraint, Provenance, Authorization],
    validator: AgentId,
    proposer_separation: "distinct-authenticated-actor/1",
    provenance: "retained-admissible-evidence/1",
    application: "ekr.p1-apply/1"
}
```

The agent map must contain actual bootstrap actors and every actor this P1 host admits. Each key
must equal record.id. Names/capabilities are host-supplied facts, not invented fixture-like defaults.
The capability strings are retained metadata; this slice does not claim their permission semantics
or P5 trust profiles. The fixed profile names exactly implemented semantics; unknown versions,
omitted/reordered checks, another validator, or an unsupported profile refuse. At minimum bootstrap
operator and validator differ and are present. The profile's validator must match actual execution
context. A later multiple-validator policy is a new explicit profile, not an arbitrary caller list.

The policy must state the real provenance rule. Current ordinary validation checks retained evidence
identity/nonempty citations, while seed validates HumanStatement bytes. If the writer promises
retained admissible bytes, replay also verifies that cited retained payloads remain available and
hash-correct. It must not advertise stronger confidence/source trust checks that do not yet exist.

Canonical encoding is format, agents map, profile fields in declaration order; map keys and sets
have existing canonical order, checks are an ordered fixed sequence. This yields populated agent
content and a policy commitment without adding unsupported registration/trust operations.

### ProposalRecordV1

```text
format: "ekr.proposal-record/1"
event_id: EventId                       # Proposed occurrence
submitted_at: Timestamp
submitter: AgentId                      # trusted host identity
document_hash: ContentHash              # exact original submitted document bytes
transaction: GraphTransaction<Value>    # full typed proposal; no dropped operations
operation_count: u64                    # derived, nonzero after structural submission admission
evidence_hash: ContentHash              # canonical ID manifest
canonical_transaction_hash: Option<ContentHash>
canonical_operations_hash: Option<ContentHash>
```

The document handler reads an explicitly versioned transaction document. Paths stay outside the
validator/receipt domain. Bind transaction.proposer to submitter by rejecting a mismatch; also
reject conflicting AddAssertion.proposed_by values rather than retaining false attribution. The
caller does not select the validator. Malformed/empty submissions do not create an accepted
Proposed transaction merely to populate the view. A well-formed but semantically invalid proposal
is retained so Validate can produce its real refusal. The optional canonical hashes are recomputed
when possible, never caller supplied; the document hash always identifies its retained input bytes.

### ValidationBasisV1 and ValidationReceiptV1

```text
ValidationBasisV1 {
    format: "ekr.validation-basis/1",
    graph_root_id: GraphRootId,
    previous_revision_id: RevisionId,
    previous_event_id: EventId,
    previous_record_hash: ContentHash,   # seed or commit receipt, including its time/context
    previous_root: Root,                # all fields, not revision alone
    previous_root_hash: ContentHash,
    seed_hash: ContentHash,
    ontology_root: ContentHash,
    authority_root: ContentHash,
    validation_profile_hash: ContentHash
}
ValidationReceiptV1 {
    format: "ekr.validation-receipt/1",
    event_id: EventId,                  # Validated occurrence
    proposed_event_id: EventId,
    proposal_record_hash: ContentHash,
    transaction_hash: ContentHash,
    operations_hash: ContentHash,
    evidence_hash: ContentHash,
    operation_count: u64,
    basis: ValidationBasisV1,
    validators: BTreeSet<AgentId>,       # exactly the fixed profile's actual validator in P1
    validated_at: Timestamp,
    validation_hash: ContentHash
}
```

Define validation_hash exactly as the canonical value-domain hash of:

```text
ValidationMaterialV1 {
    format: "ekr.validation-material/1",
    transaction: GraphTransaction<CanonicalValue>,
    basis: ValidationBasisV1,
    validators: BTreeSet<AgentId>
}
```

No hash is computed over a field containing itself. The receipt's blob hash separately binds its
occurrence IDs and validated_at. An equal transaction against equal complete basis and validator
set has the same validation hash; two validation occurrences still have different EventIds. A new
kernel ruleset must have a new recognized profile and dispatch; running today's changed validators
while claiming to reproduce an old ruleset is not replay verification. Unsupported old profiles
produce a named refusal until a supported preserving migration exists.

### CommitReceiptV1

```text
format: "ekr.commit-receipt/1"
event_id: EventId                       # RevisionCommitted occurrence
revision_id: RevisionId
proposal: ProposalRecordV1              # full input retained in the receipt for bounded replay
validation: ValidationReceiptV1
validation_record_hash: ContentHash
committer: AgentId                      # host execution identity, retained rather than inferred
committed_at: Timestamp
result: Root
result_hash: ContentHash
```

The repeated full proposal is intentional for the minimum slice: the store can hand the complete
receipt bytes to kernel replay without learning GraphOperation or recursively fetching kernel-
specific record dependencies during a provider transaction. Its embedded proposal must have
exactly the separately recorded proposal hash; validation must match the separately retained
validation record. Store the separately addressed records as well because Proposed and Validated
must survive a restart before Commit. Do not reconstruct their earlier occurrences retrospectively
from a later commit receipt. Future deduplicated bundles may remove duplication with a new version.

The validation record is metadata over the full retained proposal, not a substitute for it. For
canonical hashing, re-narrow the proposal after rerunning the supported kernel validators. The
result is recomputed and compared field-for-field with result and result_hash. The accepted receipt
cannot be deserialized directly into ValidatedTransaction.

### Seed result and refusal records

Seed publication also needs its own retained result record, containing its EventId, RevisionId,
seed envelope address, authority root, committed_at, full Root0 and Root0 hash. Root0 uses the
retained ontology and authority state; parent is None; Root0.transaction is the seed envelope hash
as today. Initialize returns this record's result, not a later store.head value. Its exact receipt
hash supplies previous_record_hash for the first transaction's basis.

Rejected and Stale decisions have retained strict records too. Rejected binds its EventId,
proposal hash/ID, requested complete basis, actual validator, timestamp, and the full ordered
validation issues; stable IssueIds are assigned once at recording, outside deterministic checks.
Stale binds its EventId, accepted receipt hash, expected basis, observed committed root/receipt and
timestamp. Neither creates a canonical revision. A terminal Transactions row is reconstructed
from these records; it is not a map held by the CLI/conformance target.

### Revision fact linkage

```text
{ format: "ekr.revision-event/3",
  event_id: EventId,
  record_hash: ContentHash,
  payload: <the corresponding six named revision-event payloads> }
```

Backend schema_version is 3. record_hash selects SeedResult, ProposalRecord, ValidationReceipt,
RejectionRecord, CommitReceipt or StaleRecord according to the payload variant. The decoder checks
all duplicated identities, hashes and counts. Preserve backend event ID, name, schema, stream
position and tenant/stream coordinates; an EventId is not silently substituted for a backend ID.
Canonical record encoding includes format, EventId, record_hash and payload. If folded into the
still-pending event/2 contract before publication, the coordinator must update this exact linkage
and its golden vectors together. There is one active contract, not two guessed readers.

## Computing real roots

- **Ontology:** add `Ontology::to_document()` or deterministic complete iterators. A kernel-owned
  canonical wrapper can use the existing declaration encoders currently private in
  `transaction.rs:371–516`, factored to one shared module. Encode schema id/number/parent/created_at,
  every node declaration keyed by TypeId, and every edge declaration keyed by TypeId. Include names,
  parents, properties, required/cardinality/type/constraints, lifecycle, operations, endpoint sets,
  inverse/symmetric/transitive fields. Sort document declaration vectors by stable IDs after load;
  input declaration order is not different governing ontology. Tests vary every semantic field,
  including an unused type/operation, and show a changed ontology_root. No schema evolution is
  needed to hash the complete immutable P1 seed ontology.
- **Knowledge:** preserve the stated nodes/edges/assertions map composition, using accepted v2
  multiplicity and assessment/lifecycle encoders. Populate initial node type_state from its declared
  lifecycle and apply validated Invoke transitions. Metadata not represented in this sub-root must
  be bound elsewhere rather than assumed covered.
- **Evidence:** retain the evidence-map canonical root, whose records name retained payload hashes.
  Verify needed payload bytes as well as IDs during replay. The seed stores every initial statement
  payload; no current GraphOperation ingests new evidence. The first writer slice deliberately uses
  retained seed evidence, without pretending transaction.evidence creates new evidence.
- **Agents/policy:** hash the populated AuthorityStateV1 above; use no zero placeholder. P1 cannot
  silently change this state at reopen. If the coordinator instead explicitly selects an absent
  registry profile, give the exact versioned empty value a real hash and state that populated
  authority-root acceptance remains unmet.
- **Revision:** number is prior+1, parent is the canonical hash of prior Root, transaction is the
  full canonical accepted transaction hash. Compute result_hash from this complete Root. Record
  occurrence identity/time in its receipt; include the previous receipt hash in validation basis.

The graph's root identity/schema association is checked on every application/replay. The seed's
complete retained ontology is loaded; a same-ID but content-different caller ontology refuses.
Prefer an opening API that derives ontology from retained seed and treats an optional caller
ontology only as a checked expectation, never the authoritative replacement.

## Kernel replay and ownership, step by step

Retain the existing dependency boundary: only ekr-kernel declares ekr-store or implements its
production authority port. The smallest port evolution is to replace the boolean `attests` query
with a fallible, deterministic replay/apply callback returning an admitted revision result (graph,
full Root, root hash, occurrence coordinates and committed_at). The kernel callback consumes
previous admitted state, verified retained seed bytes and the complete current receipt bytes.
Store remains responsible for byte integrity, recorded coordinates and conditional publication;
it does not implement GraphOperation semantics or decide that receipt syntax means acceptance.

Seed admission returns the corresponding admitted Root0/result as well as the graph so the store
does not independently insert placeholder roots. Passing full immutable inputs avoids rebuilding
a transient allowlist or making the authority re-enter its own store inside a provider transaction.
All callbacks and validation happen before conditional publication, outside provider transaction
callbacks. Any alternative kernel-owned replay loop must preserve the same ownership guarantees;
moving the loop is larger than extending this port and is not necessary for the first slice.

1. Read/verify the committed seed record, its exact retained envelope bytes and configured trust
   anchor. Strictly dispatch seed/graph/authority versions. Run real seed admission, derive complete
   ontology and authority state, recompute all Root0 fields, and compare the retained result.
2. Read ordered revision facts with backend metadata. Check recognized schema/name/envelope,
   occurrence uniqueness, exact retry equality and transition order. Resolve every record_hash
   through verified retained object/blob bytes; missing/redacted/corrupt content names the record
   and refuses. A malformed durable commit must not silently leave the fold at revision zero.
3. For each Proposed/Validated/refusal fact, verify record linkage and retain its transaction-view
   state. Verify the separately recorded proposal document and all derived counts/hashes. No
   proposed identity or state can replace another retained proposal under the same TransactionId.
4. For a commit, require its validation receipt to name the exact previous Root and receipt, graph
   identity, full ontology root and authority/profile root. Recompute hashes, check actor bindings,
   temporal ordering and accepted profile. Resolve evidence payloads; missing evidence is a named
   failure, not an empty lookup that a later validator forgets to inspect.
5. Reconstruct GraphTransaction<Value> from the retained proposal, rerun the matching real
   deterministic pipeline with the retained actual validator and admitted previous snapshot, and
   reseal a private capability containing this complete basis. Compare the independently computed
   validation material/hash. Do not deserialize a ValidatedTransaction or add the stored hash to
   Validations::transactions merely because the receipt contains it.
6. Apply the accepted unordered operation set to a copy with the same pure application function
   live Commit uses. Create identities first, apply property/lifecycle changes to the agreed
   candidate, delete canceled edges consistently, attribute acceptance from actual validation,
   and set transaction times from committed_at. Preserve multiplicity and inner Lists. Conflicting
   lifecycle/retraction/replacement actions must refuse rather than depend on vector order.
7. Compute four sub-roots and the resulting revision from that candidate. Require equality with
   every retained result field and its hash, then advance the fold once. Never trust a published
   knowledge_root or full result as the value to return without recomputation.
8. Historical snapshot selection stops at the requested committed revision and returns that
   admitted result. Missing revision refuses. Latest-state reads continue through all committed
   results. A noncommitting terminal decision remains visible in Transactions without moving the
   canonical graph. Repeated queries do not mutate history.

This is executable replay verification, not cryptographic proof of an external human's identity.
The host authenticates submission/execution context; hashes bind the retained account of it.
Authenticating an arbitrary untrusted external history requires a separate trust anchor/signature
policy. Do not promise resistance to an attacker who can replace the entire trusted source and
all its addresses merely because its self-consistent receipts rerun successfully.

## Conditional publication and retry result

Validate against a coherent immutable snapshot with full root/receipt coordinates. Build candidate,
roots, record bytes, occurrence IDs and time once. Publish new retained blobs/bindings, object
metadata and the successful decision/revision fact together through the coordinated atomic provider
port. Its lineage expectation uses the actual recorded backend stream version, not RevisionNumber:
noncommitting Proposed/Validated/Rejected/Stale facts also occupy stream positions.

On conflict, reread the committed head and first check whether this same occurrence already
committed. If exact record bytes match, return its retained result. If the same identity carries
different bytes, refuse identity conflict. If another canonical Root won, retain/report Stale and
leave the proposed graph unapplied. If only noncommitting records advanced the stream while the
same complete canonical basis remains, a bounded retry may update the backend expectation while
preserving the intended occurrence/receipt. The port's command-fingerprint and retry-key rules
must explicitly support that case; a new attempt identity must never create a new domain occurrence.
After unknown commit, resolve the original occurrence before attempting another publication.

This backend-position versus canonical-head distinction needs a regression. Treating every raw
stream conflict as proof the validated canonical revision changed can incorrectly mark a valid
transaction stale merely because another proposal was recorded.

Proposed and Validated records are legitimately durable before commit; atomic commit does not
erase them on a later stale decision. The no-partial-commit guarantee applies to the new canonical
result and its newly required commit artifacts. Losing candidate blobs must not leak into the
public blob namespace when their preexistence did not justify them. Previously retained submission
and validation bytes remain as history. No cleanup-after-error may delete shared or unknown-commit
content. Both seed and commit return their own retained result, never whatever head happens to be
latest after publication.

## Dependencies and truthful implementation boundaries

- **Pending persisted format:** assertion assessment/lifecycle, multiplicity, explicit graph/seed
  version dispatch, event occurrences and frozen old encoders must settle first. Receipt links,
  authority-in-seed placement and lifecycle operation shapes must join the coordinated contracts
  before their relevant encodings freeze. Old records are not silently decoded with new defaults.
- **Atomic blob publication:** pinned Eventlog 0.2.1 has independent blob puts and atomic event
  groups, but no public atomic blob+append capability. New durable proposal/validation/commit/seed
  payload publication cannot keep both metadata-only events and no-losing-object semantics until
  a coordinated upstream capability is released and pinned. Pure applier/root/receipt/replay tests
  can progress, but successful provider durability/no-write acceptance cannot be claimed by
  replacing that dependency with put-then-append or an inline payload fallback.
- **Live history inspection:** normal provider opens can perform DDL/housekeeping. Read-only
  inventory of existing sources needs a provider-supported stable read-only snapshot/export
  contract. This does not prevent fresh disposable provider writer tests once atomic publication
  exists; it does prevent claiming source inventory or preservation/cutover of actual old stores.
- **Preserving history migration:** missing operation payloads, prior acceptance, original policy,
  ontology or evidence refuse with exact affected record. The legacy in-memory authority cannot
  be recreated from a recorded validation hash. Do not migrate revision-1 no-op stubs as if they
  had durably applied their absent operations. Preserve their original bytes and original verified
  behavior; conversion eligibility requires independently verifiable history.
- **CLI/conformance:** share real Propose/Validate/Commit/document handlers and Transactions reads.
  No adapter-local decision map, caller hash substitution, revision-1-to-0 remapping or permissive
  replay authority may satisfy acceptance. This report adds no CLI verb or separate plan.

## Red-first acceptance and implementation order

1. Freeze the proposed receipt/profile/root vectors after coordinator decisions; write a failing
   real-provider child-process test showing a CreateNode+AddAssertion commit changes graph state
   and remains after restart. Record both provider failures separately. Current behavior fails
   apply and real-authority reopen; do not stop after one backend's panic.
2. Implement complete ontology export/canonical hashing and explicit populated authority state.
   Verify declaration-order equality and per-field sensitivity, including unused ontology types;
   actor/profile changes affect their bound root. Same revision/schema IDs with different content
   refuse wrong-basis validation/replay. Zero placeholders no longer appear in new supported roots.
3. Implement the pure exhaustive applier and complete basis sealing. Test unordered permutations,
   Many versus one List value, optional clears versus required refusal, create+cancel edge agreement,
   lifecycle initialization/Invoke and reference/provenance rules. A late property member must be
   typed and reference-checked. Unsupported schema/merge operations still refuse before sealing.
4. Implement retained submission, accepted validation and terminal records plus versioned linking.
   After separate fresh-process restarts at Proposed, Validated, Committed, Rejected and Stale,
   Transactions reports the actual identity, derived count/hash fields, attribution and state.
   Forge caller proposer/assertion attribution and verify the trusted handler refuses it.
5. Implement atomic publication with both real providers and independent handles. Two validated
   candidates against the same full basis yield one changed canonical result; the loser is stale.
   An interleaved unrelated proposal alone does not make canonical state stale. Assert raw losing
   blobs, metadata and committed lineage as well as the high-level snapshot.
6. Exercise interruption before publication, after durable success but before response, and restart.
   No partial canonical commit appears; exact retry returns its original root/timestamp/IDs even
   after another transaction advances head; changed payload under the same identity refuses. Seed
   receives the corresponding own-result regression once ordinary commits exist.
7. Rerun live and replay through the same kernel validation/application rules; tamper independently
   with payload bytes, root/sub-root, profile/check list, actors, previous receipt/root, timestamps,
   ontology under the same schema ID, receipt address and evidence bytes. Each gives a named refusal
   rather than silently ignoring the commit or reverting to seed. The kernel-only writer and
   non-deserializable-capability guards stay effective.
8. Complete the existing story's lifecycle and historical acceptance: accepted evidence/validators
   survive retraction and supersession; latest valid-time reads answer the handover example; an
   earlier committed revision retains its prior view. Self/cyclic replacement, unreadable or
   retracted replacement, invalid boundary and conflicting lifecycle writes refuse. No fabricated
   retraction reason. Confirm reconstruction starts from seed and stops at the requested revision.
9. Run touched package suites, format/clippy/docs, integrated full gate and independent adversary.
   Retain exact per-provider outcomes. A passing first vertical witness is not closure of steps 4–8
   or of actual old-store migration. No tests were run by this read-only preparation.

## Scope

Derived 2026-09-22 by `story-scoper` from the story and inspected source. Every entry identifies
whether the path/change is cited or inferred; this section is a proposed refresh for the existing
story, not a new work item.

- **Primary:** `crates/ekr-kernel/src/commit.rs`, `crates/ekr-kernel/src/transaction.rs`,
  `crates/ekr-kernel/src/validate/mod.rs`, `crates/ekr-kernel/src/seed.rs` — cited; existing stub,
  sealed basis, validator context and initialize-then-head path.
- **New kernel implementation:** `crates/ekr-kernel/src/apply.rs`,
  `crates/ekr-kernel/src/revision.rs`, `crates/ekr-kernel/src/receipt.rs`,
  `crates/ekr-kernel/src/authority.rs`, `crates/ekr-kernel/src/lib.rs` — inferred; pure application,
  complete roots/replay, strict retained records, authority profile and exports.
- **Existing validation surfaces:** `crates/ekr-kernel/src/validate/structural.rs`,
  `crates/ekr-kernel/src/validate/ontology.rs`, `crates/ekr-kernel/src/validate/provenance.rs`,
  `crates/ekr-kernel/src/validate/authorization.rs`, `crates/ekr-kernel/src/validate/reference.rs`
  — cited; current supported-operation/conflicting-write, lifecycle, attribution and reference rules
  must agree with the candidate applier and explicit new lifecycle operations.
- **Ontology export:** `crates/ekr-ontology/src/schema.rs`,
  `crates/ekr-ontology/tests/ontology_load.rs` — cited; complete state is private today, so a total
  lossless export/iterator plus its roundtrip coherence test is necessary.
- **Store ports and implementation:** `crates/ekr-store/src/log.rs`,
  `crates/ekr-store/src/eventlog.rs`, `crates/ekr-store/src/lib.rs` — cited; boolean authority,
  placeholder fold, historical selection, payload retrieval and atomic conditional publication.
- **Graph contracts:** `crates/ekr-graph/src/events.rs`, `crates/ekr-graph/src/root.rs`,
  `crates/ekr-graph/src/snapshot.rs` — cited; receipt linkage/occurrences, computed root contracts
  and historical/lifecycle read behavior. Pending format must be integrated first.
- **Kernel acceptance:** `crates/ekr-kernel/tests/replay.rs`,
  `crates/ekr-kernel/tests/receipts.rs`, `crates/ekr-kernel/tests/durable_transactions.rs` — inferred;
  both real providers, true new-process restart, retained state and corruption/concurrency cases.
- **Existing regression suites:** `crates/ekr-kernel/tests/commit_path.rs`,
  `crates/ekr-kernel/tests/seed.rs`, `crates/ekr-kernel/tests/validation.rs`,
  `crates/ekr-store/tests/fold_rules.rs`, `crates/ekr-store/tests/providers.rs`,
  `crates/ekr/tests/story_contract.rs` — cited; stub expectations and ownership/no-write guarantees
  that must change explicitly or remain held.
- **Coordinator-owned contracts:** `docs/epistemic-knowledge-runtime-design.md`,
  `systems/ekr/domains/kernel.yaml`, `systems/ekr/domains/graph.yaml`,
  `systems/ekr/domains/store.yaml` — cited; authority/root/receipt/context and operation semantics,
  retained Transactions and exact versioned event/document contracts.
- **Dependency pin:** `Cargo.toml`, `Cargo.lock` — inferred; only after the required upstream atomic
  blob-publication capability has a released, reviewed version. Its upstream source is separate work.
- **Confidence:** high — cited; current paths contain the measured stub, missing basis/context,
  placeholders and publication/replay ports. New module layout is inferred as marked.
- **Would collide with:** persisted graph/seed/event formats, atomic blob migration, kernel
  submission/CLI document handlers, authority/root encoders, lifecycle validators and store replay
  — cited; these share the named sources and require coordinated sequencing or one owner.

## Scope-entry commands for the coordinator (not run)

```sh
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/commit.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/transaction.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/validate/mod.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/seed.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/apply.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/revision.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/receipt.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/authority.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/lib.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/validate/structural.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/validate/ontology.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/validate/provenance.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/validate/authorization.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/src/validate/reference.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-ontology/src/schema.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-ontology/tests/ontology_load.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-store/src/log.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-store/src/eventlog.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-store/src/lib.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-graph/src/events.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-graph/src/root.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-graph/src/snapshot.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/tests/replay.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/tests/receipts.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/tests/durable_transactions.rs --inferred
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/tests/commit_path.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/tests/seed.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-kernel/tests/validation.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-store/tests/fold_rules.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr-store/tests/providers.rs
aep plan artifact scope story:commit-and-revision-lineage --add crates/ekr/tests/story_contract.rs
aep plan artifact scope story:commit-and-revision-lineage --add docs/epistemic-knowledge-runtime-design.md
aep plan artifact scope story:commit-and-revision-lineage --add systems/ekr/domains/kernel.yaml
aep plan artifact scope story:commit-and-revision-lineage --add systems/ekr/domains/graph.yaml
aep plan artifact scope story:commit-and-revision-lineage --add systems/ekr/domains/store.yaml
aep plan artifact scope story:commit-and-revision-lineage --add Cargo.toml --inferred
aep plan artifact scope story:commit-and-revision-lineage --add Cargo.lock --inferred
```

## Unestablished points

- Authority/profile schema, trusted host context source, exact timestamp ordering and receipt/event
  version linkage are recommendations awaiting coordinator decisions; none is already implemented.
- The proposed applier/root/receipt module layout is inferred. No performance or storage-amplification
  claim was measured; full proposal duplication deliberately favors a bounded first replay contract.
- No released atomic blob+append or true read-only provider inspection capability was verified.
- No live legacy store or auxiliary operation/receipt archive was inspected; successful historical
  conversion cannot be promised from the presently retained hash-only legacy events.
- No test/build was run. The recommended first vertical witness and all acceptance above remain
  unexecuted, including both provider lanes and real process restart.
