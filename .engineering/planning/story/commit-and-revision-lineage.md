---
format: aep.planning-md/1
id: story:commit-and-revision-lineage
kind: story
status: draft
title: Commit, revision roots, optimistic concurrency, retraction
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:transaction-and-validators
- depends_on: story:eventlog-store
- implements: executable-system-specification:ekr-v1
- depends_on: story:kernel-validated-seed
- depends_on: story:version-persisted-contracts
scope:
- confidence: inferred
  path: crates/ekr-core/src/decode.rs
- confidence: cited
  path: crates/ekr-core/src/lib.rs
- confidence: inferred
  path: crates/ekr-kernel/src/apply.rs
- confidence: inferred
  path: crates/ekr-kernel/src/commit.rs
- confidence: inferred
  path: crates/ekr-kernel/src/document.rs
- confidence: inferred
  path: crates/ekr-kernel/src/document/
- confidence: cited
  path: crates/ekr-kernel/src/lib.rs
- confidence: inferred
  path: crates/ekr-kernel/src/revision.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/adversary_transaction_document.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/replay.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/transaction_document.rs
- confidence: cited
  path: crates/ekr-ontology/src/types.rs
revision: 16
---
## Context

Design § 34: each successful commit produces a new immutable root with a parent, four sub-roots
and the transaction hash. Design § 71–72: validation records the revision it ran against, and a
commit that finds canonical state moved must not apply. Design § 36 and § 65: retraction and
supersession end an assertion's active life without deleting history.

## Acceptance

Through both SQLite and file providers and the real kernel authority: seed, validate, apply, persist, close the process, reopen, query the changed graph, retract, and reconstruct an earlier revision. The graph hash changes for the state-changing commit and reproduces on restart. The persisted ontology is content-bound and reopening does not substitute a caller-supplied different ontology.

## Tests the story ships

- Two commits validated against the same head cannot both publish: the loser returns Stale and changes no canonical graph.
- A crash before the conditional revision append exposes no partial state; retry after a lost successful response returns the original committed result.
- Retraction retains its original acceptance evidence and remains readable at an earlier revision. Latest-state queries exclude the retracted belief.
- Supersession records its replacement and supplied valid-time boundary. Queries before and after that boundary return the correct belief, including the design section 65 CLI example.
- Replay verifies persisted payload addresses, schema content, validation basis and revision ancestry. Missing or corrupt required payloads produce named errors, not revision-zero fallback.
- Property multiplicity and list-valued properties remain distinct; accepted multivalued input is not silently collapsed by application.

## Scope

- `crates/ekr-kernel/src/commit.rs` — `commit(ValidatedTransaction) -> Result<Root, CommitError>`;
  applies each `GraphOperation` to the canonical graph; computes the four sub-roots and the new
  `Root`; `Stale` when `validated_against != head`; fills the `RevisionEvent` and appends it to
  the `RevisionLog`
- `crates/ekr-kernel/src/apply.rs` — one function per `GraphOperation` variant, including
  `RetractAssertion` (sets `AssertionStatus::Retracted`), supersession (`Superseded { by }`,
  closes `valid_to`), `MergeEntity` (alias preserved, lineage recorded)
- `crates/ekr-kernel/src/revision.rs` — `Root` hashing, lineage walk, replay
- `crates/ekr-kernel/tests/replay.rs`

## Notes

Depends on `story:transaction-and-validators` and `story:eventlog-store` (the `RevisionLog`
trait the commit appends to). Crate dependencies are the skeleton's: `ekr-kernel` → `ekr-core`,
`ekr-ontology`, `ekr-graph`, `ekr-store`. `Root` and `RevisionEvent` are defined in `ekr-graph`
(`story:graph-model-and-assertions`); this story computes the one and fills the other. Commits
are serialised: one writer, the kernel. Adds no dependency beyond the skeleton's.

## Scope correction, 2026-09-22

The earlier scope sentence requiring MergeEntity application is superseded by the approved phase boundary: explicit merge/split semantics arrive in P3; schema evolution arrives in P5. P1 refuses these operations instead of certifying them. Ordinary graph changes, declared lifecycle operations, retraction and supersession are in scope.

The earlier reference to active() is obsolete. Acceptance is expressed through snapshot revision and valid time. Populated ontology state must contribute its canonical content to ontology_root. An actually absent agent registry may use a specified empty root; populated agent/validator state must be bound once introduced. All persisted shape changes require an explicit version and a preserving migration path, which refuses unreconstructable history.

## Retained submission and result records

The concrete conformance scope in
`.engineering/waves/p1-conformance-document-contract.md` requires a retained
Transactions read surface across Proposed, Validated, Committed, Rejected and
Stale. The writer retains the submitted operation payload, authenticated proposer,
derived operation count and hashes, validation basis and resulting decision; a
target-local map cannot supply those facts after restart. CLI and conformance
must use the same real document handler and kernel reads. The report records
synthesis only; runtime execution remains unexecuted.

Initialization and every commit must return the result of their own occurrence,
including after a lost response, rather than a later head published by another
writer. The seed implementation's current initialize-then-head sequence is a
specific surface to reconsider when concurrent ordinary publication becomes real;
this is a static future-writer concern, not a reproduced seed failure.


## Durable implementation contract

The authoritative preparation below is synchronized with
`.engineering/waves/p1-durable-record-decisions.md`. Its required behavior remains
unexecuted until persisted-contract activation and the writer implementation.
This revision consolidates previously duplicated decision sections and adds the
frozen parser profile from the read-only parser readiness review.


These decisions complete the preparation in p1-writer-preparation.md. They are
implementation requirements, currently unexecuted. The coordinator incorporates
them into the dated design amendment and ESS before production activation. Pure
historical decoder preparation does not depend on implementing these new shapes.

### Authority and roots

Use the preparation report's explicit AuthorityStateV1 and fixed P1 profile.
Trusted bootstrap configuration supplies actual registered agent IDs, names and
capability metadata, including distinct operator and validator. Validate filing
and the exact supported profile; retain that full state. Capabilities remain
metadata until their later permission semantics exist. Do not claim P5 trust
policy from this P1 registry.

agent_root hashes the complete authority state and profile. ontology_root hashes
the full loaded ontology, including unused declarations and all semantic fields,
with declarations ordered by stable IDs. Existing zero placeholders disappear
only when these real retained inputs exist and their sensitivity tests pass.
Add complete Ontology export, using one encoding for declaration semantics.

Seed-envelope/2 contains the full Seed/2 input, actual BootstrapContext, retained
AuthorityStateV1 and the seed's committed_at. Authority is host configuration,
not a permissive policy accepted from the untrusted seed document. Reopen checks
the trusted bootstrap anchor and uses the retained state thereafter. P1 cannot
replace the registry/policy through another startup argument. New authority
mutation remains later explicit transaction work.

### One event version and immutable occurrence inputs

Use the still-unpublished ekr.revision-event/2, backend schema 2, with mandatory
event_id, record_hash and the corresponding named payload. Incorporate linkage
now; do not introduce the preparation report's proposed event/3. Every fact
resolves a strict addressed record selected by its kind. Check all duplicate
identities, hashes and counts. Event identity, provider identity and atomic-group
attempt identity remain different coordinates.

Initialize consumes the complete Seeded occurrence, SeedResultV1 and required
retained payloads as one prepared publication. Verify their event kind, seed hash,
revision ID, EventId, receipt address and computed root. Return its own recorded
result, including on retry after later head advancement. Object-contention retry
must retain the domain occurrence while respecting group fingerprint equality.

### Trusted time and complete validation basis

Trusted execution context supplies actor and timestamp once for each occurrence.
Require submitted_at <= validated_at <= committed_at and nondecreasing commit
time; equal timestamps are legal and revisions order them. Also enforce affected
assertion recorded_from bounds. Persist timestamps and reuse them on retry/replay.

ValidationBasisV1 binds the full previous Root plus its hash, previous revision
and event IDs, previous seed/commit receipt hash, seed hash, graph identity,
ontology/authority roots and exact profile hash. The previous receipt is required
because Root alone does not bind the previous time/context.

Root.transaction is the full accepted canonical transaction hash. Validation
material is the report's versioned full canonical transaction, complete basis
and exact actual validator set, hashed in the canonical value domain. The
receipt's separate payload address binds occurrence IDs and validated_at.
No self-containing hash or process-local receipt allowlist grants authority.

### Preserve the exact submitted document

ProposalRecordV1 retains exact versioned transaction document bytes and their
payload-domain document_hash, actual submitter, submitted_at, transaction ID,
derived operation_count/evidence_hash and optional canonical transaction and
operations hashes. Reparse the full retained document with its exact strict
versioned parser for validation/replay. The complete original operation payload
is therefore retained even before canonical narrowing.

This replaces the report's additional JSON-serialized GraphTransaction<Value>
field: nonfinite Float values would not survive ordinary JSON serialization
faithfully. Do not manufacture a canonical address for refused Float input.
Well-formed semantically invalid documents can be proposed and then rejected.
Malformed or structurally empty submissions refuse without a successful
Proposed record. Bind transaction.proposer and AddAssertion.proposed_by to the
trusted submitter. Caller paths/hashes/counts do not supply these facts.

A typed submission convenience, if added, must encode a documented lossless
transaction document and use this same submission path. It cannot invent an
original caller document that was never provided. CLI and conformance use the
actual bytes/document handler directly.

The first proposal document envelope is YAML with exactly format
"ekr.transaction-document/1" and transaction containing the full typed
GraphTransaction<Value>. Use the existing serde_yaml_ng parser family already
used by SeedDocument::from_yaml; JSON syntax is accepted only as its supported
subset. Admit one UTF-8 document under an explicit input bound, refusing duplicate
keys, unknown semantic fields, mismatched nesting and unsupported versions.
Arbitrary unique Record keys remain data. Preserve the exact supplied bytes,
including a well-formed nonfinite Float proposal that Validate must reject.
This first transaction document version is independent of graph/seed/event version
two. The implementation must bind parser shape and exact-byte rejection readback
to executable cases; those cases are unexecuted at this decision.

### Frozen parser profile for transaction-document/1

Adopt the bounded route and counting rules in
`.engineering/reviews/p1-transaction-parser-readiness.md`. These are conservative
policy choices, not measured capacity claims. Inclusive maxima are: 262,144 raw
input bytes; one document; container depth 32; 32,768 expanded nodes; 4,096 entries
per mapping or sequence; 65,536 bytes per decoded string; 4,096 bytes per decoded
key; 1,048,576 cumulative expanded string bytes; 256 operations; and 1,024 input
evidence-manifest elements. Operations must be nonempty. Alias occurrences charge
their expanded position and content; counters use checked arithmetic.

Check the raw byte limit before parser entry or unrestricted reads. A shared
budgeted representation pass checks nesting and expansion before application
allocation, then strict typed decoding uses the original bytes. It must not
silently change typed string semantics through generic Value conversion. Reject
duplicates at semantic fields and after actual map-key decoding, before decoding
a duplicate's value. Unique Record keys stay data, including reserved-looking
names and merge-key text; do not invoke YAML merge processing. Required containers
need actual map/sequence syntax, so an empty plain scalar cannot become an empty
collection by coercion. Preserve explicit empty inner Lists and Records.

This version uses the typed YAML operation tags supplied by the selected parser,
such as `!CreateNode`; an untagged JSON object around an operation name is not an
alternative encoding. JSON-compatible scalar and collection syntax remains usable
where that typed YAML grammar accepts it. A typed convenience must serialize the
same documented wire shape. Exact original bytes, including permitted nonfinite
Float spellings, remain the proposal's evidence.

The pinned loader buffers YAML events before visiting the representation. The raw
byte cap bounds that input and the visitor bounds expanded application values;
neither is an exact allocator-byte guarantee or a claim of pre-loader scalar/event
quotas. Do not add an ad hoc YAML lexer or another upstream dependency to claim one.
The format version selects this frozen profile in proposal, validation and replay.
A stricter host upload limit affects new ingress only, never historical validity.
Any changed frozen profile requires a new explicit version decision. All boundary,
duplicate/shape, alias and restart controls listed by the readiness report remain
required and unexecuted until the writer implements them.

### Retained decisions, replay and application

Adopt the preparation report's ValidationReceiptV1, CommitReceiptV1, SeedResultV1,
Rejected and Stale records, substituting the exact-document ProposalRecord above.
Commit retains the full proposal and validation linkage for deterministic replay;
separately retained Proposed/Validated decisions survive restart before commit.
Every payload lives behind atomic provider blob publication; event bodies retain
only identities, addresses, counts and status.

Replace boolean replay attestation with a fallible kernel replay/apply authority.
Only the kernel validates and constructs the admitted graph/root. Replay checks
payload integrity, version dispatch, linkages, basis, actual actors/profile,
evidence bytes and time; revalidates and applies with the same pure functions;
recomputes every root field; and refuses a corrupt commit instead of silently
leaving the fold at its seed. Historical reads stop at the chosen revision.

Retraction carries its reason. Explicit supersession carries replaced/replacing
assertion identities and the effective boundary. Preserve acceptance attribution
and evidence. Apply the admitted unordered set, not vector order, preserving
outer property multiplicity. Implement every admitted operation or issue a named
refusal before sealing. Unsupported schema evolution and merging stay explicit.

Canonical-head staleness differs from provider stream position: Proposed and
other noncommitting facts advance the stream but not Root. On conflict, resolve
an exact committed occurrence first, then compare complete canonical basis.
Bounded transport retry for unrelated noncommitting facts reuses the immutable
domain receipt/occurrence. Another canonical winner produces retained Stale.
Unknown commit never authorizes deletion or blind new publication.

### Exit evidence

The first decisive case is seed, propose a new node and evidence-backed assertion,
validate, apply, atomically persist, stop the process, restart and query changed
knowledge through both real providers with real kernel authority. Include a
Many property and a single List property. The knowledge hash must change.

That case begins the writer acceptance; it does not replace stale/concurrent
writers, interrupted publication, own-result retry, retained terminal decisions,
retraction/supersession, historical reconstruction, complete roots and corruption
refusals. Full preserving migration remains downstream of the writer; missing
original evidence refuses with an unchanged source.
