---
format: aep.planning-md/1
id: story:version-persisted-contracts
kind: story
status: active
title: Version assertion history and revision occurrences before durable application
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:kernel-validated-seed
- serves: vision:o2
- implements: executable-system-specification:ekr-v1
scope:
- confidence: cited
  path: .github/workflows/correctness.yml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: inferred
  path: crates/ekr-core/src/identity.rs
- confidence: inferred
  path: crates/ekr-core/src/lib.rs
- confidence: cited
  path: crates/ekr-graph/src/assertion.rs
- confidence: cited
  path: crates/ekr-graph/src/edge.rs
- confidence: cited
  path: crates/ekr-graph/src/events.rs
- confidence: inferred
  path: crates/ekr-graph/src/legacy.rs
- confidence: cited
  path: crates/ekr-graph/src/lib.rs
- confidence: cited
  path: crates/ekr-graph/src/node.rs
- confidence: cited
  path: crates/ekr-graph/src/snapshot.rs
- confidence: cited
  path: crates/ekr-graph/tests
- confidence: cited
  path: crates/ekr-graph/tests/canonical_value_and_assertion.rs
- confidence: cited
  path: crates/ekr-graph/tests/compile_fail/canonical_ref_cannot_target_a_transient_type.stderr
- confidence: cited
  path: crates/ekr-graph/tests/compile_fail/transient_state_has_no_content_address.stderr
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
- confidence: inferred
  path: crates/ekr-kernel/src/legacy.rs
- confidence: cited
  path: crates/ekr-kernel/src/lib.rs
- confidence: inferred
  path: crates/ekr-kernel/src/migration.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/candidate.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/provenance.rs
- confidence: cited
  path: crates/ekr-kernel/tests
- confidence: cited
  path: crates/ekr-kernel/tests/adversary_legacy_freeze.rs
- confidence: cited
  path: crates/ekr-kernel/tests/legacy.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/migration_inventory.rs
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/src/legacy.rs
- confidence: cited
  path: crates/ekr-store/src/lib.rs
- confidence: cited
  path: crates/ekr-store/src/log.rs
- confidence: cited
  path: crates/ekr-store/src/snapshot.rs
- confidence: cited
  path: crates/ekr-store/tests
- confidence: inferred
  path: crates/ekr-store/tests/fixtures/legacy
- confidence: cited
  path: crates/ekr-store/tests/fixtures/legacy/README.md
- confidence: cited
  path: crates/ekr-store/tests/fixtures/legacy/vectors.json
- confidence: cited
  path: crates/ekr-store/tests/legacy.rs
- confidence: inferred
  path: crates/ekr-store/tests/read_only_inventory.rs
- confidence: cited
  path: crates/ekr/tests/graph_assertion_serde.rs
- confidence: cited
  path: crates/ekr/tests/graph_events_serde.rs
- confidence: cited
  path: crates/ekr/tests/story_contract.rs
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: systems/ekr/domains/graph.yaml
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/ontology.yaml
- confidence: cited
  path: systems/ekr/domains/store.yaml
- confidence: cited
  path: systems/ekr/system.yaml
revision: 39
---
## Context

The approved completion plan places persisted-contract corrections after seed admission and before durable application. The read-only scope report in .engineering/waves/p1-persisted-contract-scope.md cites the current shapes and preservation constraints. No format change is implemented by this record.

## Acceptance

Retraction and supersession retain exact validation attribution and evidence. Independent lifecycle state and validation assessment are encoded explicitly, with proposal admission refusing caller verdicts or withdrawal state. A dated design amendment settles superseded-valid-time behavior before tests are changed.

Each revision fact has an occurrence identity: separate equal-content occurrences persist separately; retries of the same record do not duplicate it; reusing the same identity with different content refuses. This covers Proposed and Validated as well as Rejected and Stale. Preserve recorded backend identity, schema version and stream position for replay and migration.

Validation receipts use the canonical value domain with equality and sensitivity regressions. Format dispatch distinguishes old/new encoders without changing global hash labels. Unknown versions and unsupported semantic fields refuse.

Legacy verification is read-only, using frozen original vectors and their original encoders. Preserve source bytes and identities. Missing operation payloads, governing ontology, validation receipts or prior acceptance history produce a named migration refusal. Do not invent metadata or convert a seed snapshot into missing history. Complete successful history conversion remains coupled to the durable writer and its acceptance; this story alone cannot close that obligation.

## Scope

Cited by the story-scoper: graph assertion and revision-event types, canonical encoders and snapshot queries; store event metadata, append identity and fold; kernel transaction encoding, proposal admission and event publication; serialization, projection and provider regressions. Coordinator owns the additive design amendment, ESS graph/kernel/core changes and planning. New occurrence identity in core is inferred. Seed conversion paths must be re-read after the seed story lands.

## Decisions before dispatch

Adopt separate validation/lifecycle fields and an occurrence envelope covering every repeatable fact, with a versioned graph/seed envelope coordinated with the seed implementor. Record exact shapes in the additive amendment and ESS contracts before source work. The suggested format strings in the scope report are recommendations until those documents land.

## Verification

Test before implementation. Run both provider lanes for duplicate occurrence, retry and changed-payload identity conflict, including repeated same-content proposal/validation/rejection. Preserve legacy vectors and test corrupt/unknown format and missing-history refusals. Full gate and independent adversary on the integrated format precede writer dispatch.

## Additional persisted shape to settle before dispatch

The coordinator compared `crates/ekr-graph/src/node.rs:68` and `edge.rs:45`
(`BTreeMap<PropertyId, V>`) with `crates/ekr-kernel/src/transaction.rs:72`,
`:100` (`BTreeMap<PropertyId, Vec<V>>`) and `ekr-ontology/src/check.rs:138`.
Validated property multiplicity has no lossless direct home in the current
canonical node/edge field shape. The writer story already requires retaining
multiple values distinctly from one list-valued property.

Settle the explicit multiplicity container in this same versioned graph change,
before durable application, rather than inventing an implicit Value::List
convention or truncating a validated vector. Review ordering, duplicate values,
empty-vector/absent semantics, list-valued properties and frozen legacy wrapping
before adopting the exact encoding. Extend the additive design amendment and
ESS projection accordingly. This is a measured representation gap and a pending
format decision; no multivalued canonical application is claimed implemented.

## Preparation and production activation boundary

The refreshed source assessment is .engineering/waves/p1-format-refresh.md.
Its independent static review is .engineering/reviews/p1-format-refresh-review.md.
The review found one coordinator dispatch-contract blocker; no runtime claim was
tested. Resolve it by keeping preparatory version-2 models and pure legacy
verification disconnected from the existing production seed publication path.
The current version-1 API remains available during preparation. There is no claim
that version-2 normal ingestion or source cutover has happened.

Switch the public current graph/seed model and production publication together
with verified provider atomic blob+event support. No new inline payload fallback,
put-blob-then-append sequence or substitute seed authority may bridge the gap.
The combined story remains active until integrated format/occurrence activation
passes its real-provider acceptance; preparatory source alone cannot close it.

The next bounded source slice freezes original graph, assertion, transaction/
operation, event and seed representations and exact byte/hash vectors from the
integrated seed source, before any current encoders change. Pure verification
takes immutable bytes and original metadata. It neither opens a provider nor
claims a complete export, complete migration or a live-store inventory.

Adopt the proposed outer property vectors for eventual graph/2: preserve order
and duplicates; one inner List remains one value; canonical absence represents
zero values; explicit empty stored collections refuse; optional empty draft
values may later apply to absence after full validation. This decision remains
unexecuted until the eventual version-2 cases run. Frozen scalar legacy values
wrap once only after original-address verification; omitted history refuses.

Initialize will consume a complete Seeded occurrence record, preserving both
EventId and RevisionId across retries, verifying event kind and retained seed
hash. Event occurrence identity stays distinct from a transport group identity
whose object-publication entries may change under a retry. Add frozen original
AddAssertion transaction encoding and validation-hash vectors as the reviewer
recommends; current encoders cannot certify old receipts.

Strict live source inspection and atomic content publication explicitly block
the writer, independently of any preparatory format work. Complete preserving
migration remains a downstream writer/cutover acceptance, not a cyclic writer
prerequisite. Exact published and verified dependency commits suffice; a new
Eventlog tag is not required.

## Record linkage before production activation

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

## Complete activation contract preparation

The unapplied complete declaration patch is .engineering/waves/p1-writer-activation.patch,
with preparation and future-case reports beside it. It appends dated design amendments
and updates graph, ontology, kernel and store projections together. The report records
actual compiler capability, validation and synthesis evidence; no runtime scenario ran.

Current production projection guards stay intact during original-format and source-guard
integration. Apply declarations to coordinator and the activation unit together before
dispatching new-format implementation. Atomic provider source verification and coordinated
dependency adoption precede production activation. Independent contract review is pending.
Frozen original vectors and encoders remain unchanged.

## Verified provider source adoption, 2026-09-22

Eventlog main4ee3dc23f0d02a5726a0e41d097477791f09efe2 implements strict
read-only File/SQLite inspection and native atomic blob publication across all
three providers. PR11 and its exact-source required production, comparative
and restart checks passed:
https://github.com/beyond10x/eventlog/pull/11
https://github.com/beyond10x/eventlog/actions/runs/35688825492

The coordinator advanced all three Cargo.toml selectors from tag0.2.1 to that
immutable revision, regenerated Cargo.lock, and updated the existing dependency
qualifier guard and workspace story together. The provider's SQLite inspection
dependency adds nix and cfg_aliases. Consumer tests and the final integrated gate
remain the acceptance of this dependency update.

This supplies the provider capability. EKR still needs metadata-only schema2
ObjectStored publication, versioned readers, actual durable application and
the preserving migration. No existing runtime event format has been changed
by this dependency selection alone.

## Released compiler adoption

The verified ESS release is adopted in the correctness workflow and README.
The exact source pin is the annotated release commit, not an unreleased branch.
The existing EKR specification validates with its downloaded binary.

The independent released-compiler preflight is retained verbatim in
.engineering/reviews/p1-writer-released-compiler-preflight.md. It executes
validation, compilation and kernel synthesis against the unchanged complete
ess/7 activation proposal and compares its artifacts with final development
outputs. The report establishes that the prior Seed/Commit retained-result
and complete-refusal compiler gaps are closed. Its preliminary invalid component
spelling is preserved separately from the successful synthesis.

This supersedes older preparation's compiler-capability blocker; it does not
claim an activated runtime, executed EKR target or finished persisted-contract
story. The complete declaration patch remains unapplied until the shared parser
correction is integrated and the activation unit is ready to implement it.
