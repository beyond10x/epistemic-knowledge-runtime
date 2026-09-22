# Durable record decisions for the approved P1 implementation

These decisions complete the preparation in p1-writer-preparation.md. They are
implementation requirements, currently unexecuted. The coordinator incorporates
them into the dated design amendment and ESS before production activation. Pure
historical decoder preparation does not depend on implementing these new shapes.

## Authority and roots

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

## One event version and immutable occurrence inputs

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

## Trusted time and complete validation basis

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

## Preserve the exact submitted document

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

## Retained decisions, replay and application

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

## Exit evidence

The first decisive case is seed, propose a new node and evidence-backed assertion,
validate, apply, atomically persist, stop the process, restart and query changed
knowledge through both real providers with real kernel authority. Include a
Many property and a single List property. The knowledge hash must change.

That case begins the writer acceptance; it does not replace stale/concurrent
writers, interrupted publication, own-result retry, retained terminal decisions,
retraction/supersession, historical reconstruction, complete roots and corruption
refusals. Full preserving migration remains downstream of the writer; missing
original evidence refuses with an unchanged source.
