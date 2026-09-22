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
- confidence: inferred
  path: crates/ekr-store/tests/read_only_inventory.rs
- confidence: cited
  path: crates/ekr/tests/graph_assertion_serde.rs
- confidence: cited
  path: crates/ekr/tests/graph_events_serde.rs
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: systems/ekr/domains/graph.yaml
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 30
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
