# Persisted-format scope refresh — 2026-09-22

Recommendation for the coordinator: retain the draft's graph/seed/event version boundaries, add
an explicit outer property-value sequence to both Node and Edge, and dispatch the pure format and
frozen legacy verification work independently of live-store inspection and blob publication.
These are proposed amendment additions, not decisions or implemented behavior. The existing
story must retain any provider and migration acceptance it already owns until the coordinator
records a separate owner/dependency; splitting the work does not discharge those obligations.

Read-only assessment under the installed aep-drive story-scoper charter and aep-plan planning
skill. The parent explicitly requested this scratch output, overriding the charter's no-output-file
default for this one file. No repository/planning edits, builds, live-store access, leases, commits,
or upstream changes were made. Only this scratch report was written.

## Inputs and limits

- Coordinator story `story:version-persisted-contracts`, revision 18, read in full and through
  `aep plan artifact show` from `ekr-completion-20260922`.
- Coordinator draft `.engineering/waves/p1-persisted-contract-amendment-draft.md`, sections 88–90,
  and the preceding `.engineering/waves/p1-persisted-contract-scope.md`.
- Completed seed tree `ekr-p1-08-seed`, opening HEAD
  `4032d002ecd2b08519910398ae6ed3b9ec8ea0f6`, including its handed-back uncommitted implementation,
  unchanged independent seed adversary cases, and the shared ontology filing correction.
- Pinned Eventlog 0.2.1 source at commit prefix `77cda080`; no provider was opened.
- `object-blob-scope.md` in this report's directory retains the detailed atomicity assessment.

The completed seed code is the implementation basis, not the pre-seed tree at the same HEAD.
Line citations below refer to that handed-back working tree unless the provider is named.
No actual legacy source stores, auxiliary transaction archives, or migration manifests were
inventoried, and no format/source compatibility was executed in this assessment.

## What the source establishes

| Surface | Observation | Consequence |
|---|---|---|
| `ekr-graph/src/node.rs:69`, `edge.rs:46` | `BTreeMap<PropertyId, V>` | Each stored property has exactly one value slot. |
| `ekr-kernel/src/transaction.rs:72,83,100` | Drafts and updates carry `Vec<V>` per property; empty update clears. | A successful Many proposal can contain information the graph cannot retain. |
| `ekr-ontology/src/check.rs:160–187`, `value.rs:29–48` | Every vector member is typed; length determines cardinality; One permits zero/one, Many any count; required separately forbids zero. Absent and empty mean the same absence. | Do not reinterpret an outer vector as one List value or silently deduplicate it. |
| `ekr-kernel/src/transaction.rs:332–360` | Draft/update encoders already encode the vectors as ordered lists. | Preserving outer order is the smallest information-preserving decision. Sorting would change accepted proposal semantics. |
| `ekr-kernel/src/validate/candidate.rs:45–59` | Every existing graph property is counted as 1 by iterating keys. | This source must change alongside the stored shape, even if no writer exists yet. |
| `ekr-kernel/src/seed.rs:251–268` | Seed records are converted into drafts by `vec![value.clone()]`. | V2 must pass the full outer vector, not wrap it again or take one member. |
| `ekr-kernel/src/seed.rs:355–398`, `ekr-store/src/snapshot.rs:220–253` | Narrowing and widening convert one property value. | Convert every outer member, retain order, and report refusal at the entity/property (preferably outer index too). |
| `ekr-core/src/canonical.rs:176–183,204–229` | Lists preserve order/count; maps sort keys. | Existing primitives encode the new shape without a global hash-domain or newtype-tag change. |
| `ekr-graph/src/value.rs:70–73` | List and Record are recursive tagged values. | An outer sequence is distinct from each member's possible List or Record. |
| `ekr-store/src/eventlog.rs:252–287` | `read_all` discards backend ID, name, schema version and stream position. | Retaining recorded metadata is required for strict dispatch and pure exported-history verification. |
| `ekr-store/src/eventlog.rs:567–590` | Initialize currently creates Seeded internally, mints a RevisionId once, and appends schema 1. | An EventId envelope for all facts must reach this path, not just ordinary `append`. |

No ontology production change is needed merely to introduce vectors: its existing node checker and
kernel node/edge checks already operate on vectors. This assessment found no specified set/unique
property semantics. Such a policy would need a separate explicit constraint, not a storage shortcut.

## Concrete additions to the amendment draft

The following text is ready for the coordinator to adapt/add after section 89's compatibility
table. It deliberately chooses semantics rather than leaving the implementation to infer them.

### Proposed addition: property multiplicity in graph format 2

In graph format 2, Node and Edge both store properties as
`BTreeMap<PropertyId, Vec<V>>`. The outer vector is the sequence of values carried by that property;
each member independently has the property's declared ValueType. Its length is the property's
cardinality count. A value whose ValueType is List remains one value, regardless of the inner
list's length. An empty inner List is also one present value.

The outer sequence preserves order and duplicates. Neither admission, canonical encoding,
serialization, reconstruction nor migration sorts or deduplicates its members. Reordering distinct
members changes canonical state; adding a duplicate changes multiplicity and therefore the
canonical encoding. Cardinality One refuses two members even if they are equal. Cardinality Many
does not imply uniqueness. The operation set's unordered execution contract does not make each
operation's property-value sequence unordered.

Zero values are represented in canonical state by an absent property key. Every present canonical
property entry has a nonempty outer vector. A graph/2 document or seed/2 input that supplies an
empty outer vector refuses with `empty-property-values`, identifying the entity and PropertyId;
normal ingestion does not silently remove it and address different bytes. Proposal drafts and
PropertyMutation retain their existing ability to carry an empty vector: after all typing,
cardinality, required-property and constraint checks, application represents that zero-value
result by removing/omitting the property entry. The receipt retains the exact proposed operation;
normalizing resulting state does not rewrite its request or validation hash. Clearing an undeclared
property still refuses, and clearing a required property still refuses.

The canonical encoder writes the existing Node/Edge fields in their existing order, with the
properties field encoded as a map whose values are ordered lists of individually encoded values.
It does not omit empty entries during hashing: invalid shapes are refused by admission, not hidden
by the encoder. No global hash-domain label changes. Nodes or edges with no properties may retain
their old structural encoding; a format change is not a promise that every individual hash changes.

The graph/2 wire representation uses an actual JSON/YAML array as the outer container. The version
selects its meaning; there is no scalar fallback, overloaded Value::List convention, or untagged
scalar-or-vector decoder. Stable PropertyIds remain keys. For example:

```json
{"properties":{"<property-id>":[{"String":"alpha"},{"String":"beta"}]}}
```

holds two String values. The following holds one List<String> value:

```json
{"properties":{"<property-id>":[{"List":[{"String":"alpha"},{"String":"beta"}]}]}}
```

`{"properties":{"<property-id>":[{"List":[]}]}}` holds one empty list and is distinct
from `{"properties":{}}`. The property map above is a fragment, not a standalone graph envelope.

The versioned graph boundary remains exactly
`{format:"ekr.graph-document/2", graph:<graph fields>}`. Its graph contains the new assessment/
lifecycle assertion fields and the new Node/Edge property shape together. Seed/2 contains the
complete graph/2 envelope in its `graph` field. Seed-envelope/2 contains that entire Seed/2 input
in `input` and the actual operator/validator bootstrap context in `context`. All three layers
are strict. No extra property-container format tag is needed because the graph format selects it.

Before any legacy conversion, verify original bytes/addresses using frozen original types and
encoders. A legacy property entry `(id, old_value)` converts to `(id, vec![old_value])` after that
verification; a legacy List is wrapped once, never flattened. A legacy empty List converts to one
empty List, never to absence. An absent legacy property remains absent. Stable domain identities
are preserved and old/new object and root addresses are recorded. An old graph that lost multiple
proposal values cannot recover them from its one remaining slot: retained operation payloads are
required to establish complete history, and their absence produces a named migration refusal.

### Proposed addition: freeze representations before changing current types

Legacy verification has a separate version dispatch and frozen representation. Frozen GraphV1,
NodeV1, EdgeV1 and AssertionV1 retain the old single-value map and old combined assessment/lifecycle
enum. Frozen RevisionEventV1 retains the original six variants and their old canonical indices.
Frozen SeedV1 and SeedEnvelopeV1 retain their exact old nesting and bootstrap attribution. Their
verification does not deserialize through the current Node, Edge or Assertion types and does not
use the current canonical encoder to certify old hashes.

Check in original immutable byte fixtures and expected canonical byte/hash vectors with their
source revision before replacing current encoders. Verify payload-domain hashes over the exact
stored bytes, not reserialized JSON. Verify value-domain addresses with the matching frozen
encoder. The original validation receipt algorithm is its historical payload-domain construction;
the corrected algorithm is explicitly selected by the new receipt/event format. Unknown versions
and unsupported semantic fields refuse; original bytes remain available even when decoding fails.

Where verification reconstructs seed/1's original graph hash, it must reproduce seed/1's actual
bootstrap attribution rule: its retained input carried Proposed assertions and replay attributed
Accepted to the retained context's validator. Hashing its input as if it were the accepted graph
is not original verification. This historical verification establishes what bytes/state a source
format describes; it grants no new canonical authority. Raw old GraphDocument hashes likewise do
not establish that its assertions were ever admitted.

Old Retracted/Superseded assertions still need independently verifiable prior acceptance. Wrapping
properties does not manufacture validators, missing transactions, evidence, governing ontology or
event occurrences. Recognizing a legacy format, verifying an address, producing a proposed mapping,
and successfully migrating complete history are four different outcomes and must be reported as such.

### Proposed addition: occurrence identity reaches atomic initialization

The Seeded occurrence uses the same versioned record envelope as every other revision fact.
The kernel allocates its EventId once before invoking the initialization publication port, and
the identical record is reused across object-contention/internal publication retries. The store
does not mint or replace that EventId inside a retry. It verifies that the supplied Seeded payload
names the retained seed bytes and keeps Expected::NoStream on the revision stream on every attempt.
Adding occurrence identity does not turn a second initialization request into permission to append
another seed or weaken the existing no-losing-object guarantee. Request idempotency, backend IDs
and the domain EventId remain explicitly distinct and all recorded coordinates are preserved.

### Proposed addition: independent verification and publication prerequisites

Format detection, strict version decoding, canonical encoding and legacy verification are pure
operations over caller-supplied immutable byte records plus their recorded metadata. Their result
names verified addresses and any missing evidence. They do not open a source provider, recover a
journal, create tables/directories, clean caches/blobs, create a destination, or publish an event.
Frozen fixtures and an explicitly supplied immutable export may exercise this layer before a
live-store inventory capability exists. The result states exactly which supplied records were
covered; it does not imply that an export was complete or a live store was inspected.

A live-provider inventory requires a separately specified provider-supported read-only/snapshot
capability and a stable source boundary. Current normal open/read APIs are not proof of no writes.
For SQLite, inspection must account for committed WAL content and cannot copy only the main file
or use immutable-mode assumptions against an active changing database. For the file provider, a
stable committed manifest/history/blob boundary is required. Missing capability or an inconsistent
source produces a named refusal; opening for repair or making a writable clone is not silently
substituted for read-only source inspection.

New-format object publication also remains dependent on a coordinated Eventlog capability that
atomically commits blob bindings with object metadata and revision events. Format source and
occurrence tests can progress without that capability; success must not be reported as repair of
inline object payload placement or as completed seed/blob migration. Do not wire a new production
publication path through put_blob followed by append_group or add a new inline-payload fallback.

Complete history conversion and publication/cutover remain coupled to durable transaction
application, retained receipt/ontology evidence, the frozen migration manifest, verified address
mapping, and restart equivalence through the real kernel authority. Pure verification may finish
with a specific missing-evidence refusal while leaving every source byte intact. It must not mark
whole-store migration as complete.

## Work can be separated without weakening acceptance

| Lane | Can progress now after coordinator format decision | Still needed before claiming its dependent result |
|---|---|---|
| A. Pure format and verification | V2 graph/assertion/property/event types, serializers/encoders, seed admission conversion, receipt-domain correction; frozen V1 types, vectors and pure evidence inventory/refusals. | Amendment/ESS agreement; red-first shape/hash/semantic acceptance. No live migration claim. |
| B. Revision event provider adapter | Preserve recorded metadata, dispatch schema/name/envelope, append occurrence IDs, retry/content-conflict tests on fresh disposable SQLite/File logs. | Integration with seed initialization and its atomic publication path before claiming all six facts are covered. Fresh unseeded transport fixtures prove transport behavior only. |
| C. Live source inspection | Specify immutable export inventory contract and missing-evidence reporting without opening stores. | Provider-supported read-only/stable snapshot capability and actual authorized sources; preservation evidence must include opening as well as reads. |
| D. Blob publication | Prepare EKR metadata-only object format and dependency contract; no implementation claimed here. | Upstream atomic blob+append capability/release/pin, all-or-nothing provider proof, no-write checks against raw blob namespace as well as EKR streams. |
| E. Complete preserving migration | Plan frozen manifests/maps and deterministic replay comparison. | C and D plus durable writer, all retained historical evidence and actual mapped-history/restart acceptance. |

These are implementation boundaries, not permission to mark the present combined story implemented
after lane A. A and B touch some of the same source files and should be sequenced or assigned to
one owner; they are not safely parallel merely because their acceptance differs. No upstream
capability was found that would make C or D a local EKR flag change.

Provider evidence for C: EKR `eventlog.rs:148–169` calls the regular constructors and explicitly
creates the file-store directory. In pinned Eventlog, SQLite `src/lib.rs:105–136` uses
`Connection::open`, sets WAL and creates/updates tables. File `src/lib.rs:63–82` opens the journal
and immediately enters `transaction`; lines 110–114 clear snapshots and clean blobs. A search of
the pinned core/sqlite/file source found no read-only opening surface. Those observations establish
that the existing opens are unsuitable, not that it is impossible to add an upstream capability.

Provider evidence for D is retained in `object-blob-scope.md`: AppendGroup has no blob writes;
put_blob is an independent durable operation; a losing attempt can leave raw bytes even when EKR
metadata/lineage are absent. Cleanup-after-failure can delete another writer's shared blob and is
unsafe after unknown commit. Neither multiplicity nor format dispatch changes that conclusion.

## Acceptance to add or make explicit

All entries below are proposed acceptance, not executed test claims.

1. Before edits, freeze a real old Node/Edge with scalar, List, empty List and Record properties;
   a legacy graph; a complete seed/1 envelope; all six old event variants; and validation hash
   vectors. Record source revision and exact bytes/hash constants. New code verifies those same
   constants without regenerating them through current types. Seed-unit vectors must include its
   uncommitted handback revision/diff provenance until integration supplies a commit.
2. Graph/2 JSON/YAML round trips retain two values for Node and Edge, order, duplicates and nested
   Record/List values. Two equal duplicate values still count as two: One refuses, Many accepts;
   property-required checks remain independent. Two scalars, one two-element List, one empty List,
   and absence are distinguished as specified. Map insertion order does not change canonical bytes;
   sequence order and duplicate count do. Outer-list and inner-List encodings are unequal.
3. Seed/2's real kernel admission accepts multi-valued node/edge properties under Many and preserves
   every member through narrowing/widening. A wrong-kind, dangling reference, or Float in a later
   member refuses instead of inspecting only the first. The same applies recursively inside a
   member. Existing property-definition filing, opaque-constraint and ownership cases remain.
4. Canonical graph/2 and seed/2 explicit empty outer collections refuse with an entity/property
   diagnostic. Optional draft empty values/clear operations remain valid proposals; required empty
   values and undeclared clears refuse. The writer later proves that valid empty application
   removes the entry, while an empty List remains a single value. Do not claim application now.
5. Candidate's existing property count derives from actual vector length, with a focused regression
   or internal unit witness that fails if the old hard-coded 1 returns. Add a seeded multi-value
   control that survives an unrelated proposal unchanged. If current public validators expose no
   count-sensitive observation of an untouched Many property, report that limit rather than
   presenting a non-sensitive public test as coverage of the internal change.
6. V2 graph/seed/envelope nesting is exact; old scalars under graph/2, vector shapes under legacy
   tags, unknown versions/semantic fields, duplicate semantic map keys and malformed nested value
   tags refuse. User Record keys remain data. Strict duplicate-key handling needs explicit decoder
   coverage: parsing through serde_json::Value first may already discard duplicate keys.
7. The pure legacy mapper wraps each old value once. Scalar→one scalar and List→one List preserve
   exact old bytes/IDs and have independently checked old/new addresses. No live source is opened
   and no destination is created. Missing old operation vectors or previous acceptance/ontology/
   evidence returns a named refusal containing affected identity and missing evidence kind.
8. New assessment/lifecycle and valid-time semantics keep all of draft section 88's acceptance.
   New receipt hashes equal an independently constructed value-domain canonical wrapper and differ
   from the historical payload-domain algorithm; variations of transaction/basis affect them.
9. Fresh provider tests retain backend ID/schema/name/stream position and distinguish independent
   same-content occurrences from exact retries and same-ID/different-record conflicts for Proposed,
   Validated, Rejected and Stale. Include Seeded/Committed envelope paths and mixed schema/format/name
   refusals. Cover internal seed retries without weakening NoStream; defer only the upstream blob
   capability-dependent publication acceptance explicitly, not by substituting an unchecked seed.
10. C, D and E each require their own measured success/refusal evidence. Source preservation includes
    byte/metadata inventories at the complete source boundary; raw missing blobs/erasure never fall
    back to retained inline history. No end-to-end migration or P1 writer completion follows merely
    from these format tests passing.

The duplicate-map-key acceptance in item 6 is a suggested strengthening of strict decoding, not a
claim that the present seed story promised or already implements it. It should be accepted or
explicitly deferred by the coordinator before it enlarges the format dispatch. No unrelated decoder
refactor is proposed solely on this observation.

## Refreshed file scope

Existing story paths remain necessary for assessment/lifecycle, occurrences and receipt hashing.
The most important scope additions are `validate/candidate.rs`, store/kernel module exports, frozen
legacy modules, fixture directories, the ontology checking test, and the graph/store ESS projection.
No production ontology source change is inferred for multiplicity; its vector contract already fits.

```markdown
## Scope

Derived 2026-09-22 by `story-scoper` against the completed seed handback. Each line is cited or inferred.

- **Primary:** `crates/ekr-graph/src/node.rs`, `crates/ekr-graph/src/edge.rs`, `crates/ekr-graph/src/assertion.rs`, `crates/ekr-graph/src/events.rs`, `crates/ekr-graph/src/snapshot.rs` — cited; canonical multiplicity, assessment/lifecycle, event envelope and historical query semantics.
- **Seed and validation:** `crates/ekr-kernel/src/seed.rs`, `crates/ekr-kernel/src/transaction.rs`, `crates/ekr-kernel/src/validate/candidate.rs`, `crates/ekr-kernel/src/validate/provenance.rs`, `crates/ekr-kernel/src/commit.rs` — cited; full-vector conversion/counts, admission fields, hash basis and occurrence publication.
- **Store serialization and port:** `crates/ekr-store/src/snapshot.rs`, `crates/ekr-store/src/eventlog.rs`, `crates/ekr-store/src/log.rs` — cited; graph envelope/widening, recorded metadata/schema dispatch and atomic Seeded occurrence input.
- **Identity and exports:** `crates/ekr-core/src/identity.rs`, `crates/ekr-core/src/lib.rs`, `crates/ekr-store/src/lib.rs`, `crates/ekr-kernel/src/lib.rs` — inferred; EventId and exported version/inventory/error types.
- **Frozen legacy layer:** `crates/ekr-store/src/legacy.rs`, `crates/ekr-kernel/src/migration.rs`, `crates/ekr-kernel/tests/migration_inventory.rs`, `crates/ekr-store/tests/read_only_inventory.rs` — inferred; immutable DTO/encoder and caller-supplied-byte inventory modules, not normal source-provider open.
- **Existing regression families:** `crates/ekr-graph/tests`, `crates/ekr-kernel/tests`, `crates/ekr-store/tests`, `crates/ekr/tests/graph_assertion_serde.rs`, `crates/ekr/tests/graph_events_serde.rs`, `crates/ekr-ontology/tests/value_type_checking.rs` — cited; canonical field coverage, strict seed admission, provider identities, snapshots, schema projections and multiplicity checking.
- **Frozen vector location:** `crates/ekr-store/tests/fixtures/legacy` — inferred; checked-in original bytes and independent expected addresses, exposed as test data rather than current-type regeneration.
- **Coordinator documents:** `docs/epistemic-knowledge-runtime-design.md`, `systems/ekr/domains/graph.yaml`, `systems/ekr/domains/kernel.yaml`, `systems/ekr/domains/store.yaml` — cited; dated amendment, typed graph multiplicity/assessment, occurrence identity, document/record version contracts. Coordinator owns edits; no live migration is claimed by these declarations.
- **Confidence:** high — cited; concrete old scalar, draft vector, seed conversion, candidate counts, event metadata loss and provider opening behavior were read. Exact new module layout remains inferred as marked.
- **Would collide with:** graph/seed model changes, ordinary writer and receipt publication, store event metadata/blob migration, seed initialization, canonical/serde/source-surface tests, and normative ESS/design changes — cited; these share the listed source files and must be sequenced or have one owner.
```

The exported-layer placement is a recommendation, not a new crate requirement. `ekr-store` can own
old graph/event byte representations without validating them; `ekr-kernel` owns seed attribution,
receipt verification and any eligibility for a future migration. Keep the existing dependency
boundary: do not add an xtask/CLI→store bypass or a substitute canonical authority to inspect history.
There is no `systems/ekr/domains/core.yaml` in either inspected tree: the existing stable revision
identity vocabulary lives in `kernel.yaml`, even though its Rust implementation lives in ekr-core.
Use that existing ESS surface for EventId unless the coordinator explicitly creates a separate domain.
Store ESS adds document/version declarations here only. Blob object events belong to its separately
coordinated publication scope.

### Scope-entry commands for the coordinator (not executed)

These are a complete path list for the section above; existing cited entries may already be present.
Changing inferred→cited confidence is deliberate where source was now read (notably seed.rs).
Directory scopes already own contained test files; explicit new module/fixture paths identify the
proposed layout. Do not infer safe parallelism from unequal path strings: the CLI says it does not
normalize directory/file containment.

```sh
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-graph/src/node.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-graph/src/edge.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-graph/src/assertion.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-graph/src/events.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-graph/src/snapshot.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/seed.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/transaction.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/validate/candidate.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/validate/provenance.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/commit.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/src/snapshot.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/src/eventlog.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/src/log.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-core/src/identity.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-core/src/lib.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/src/lib.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/lib.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/src/legacy.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/migration.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/tests/migration_inventory.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/tests/read_only_inventory.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-graph/tests
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/tests
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/tests
aep plan artifact scope story:version-persisted-contracts --add crates/ekr/tests/graph_assertion_serde.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr/tests/graph_events_serde.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-ontology/tests/value_type_checking.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/tests/fixtures/legacy --inferred
aep plan artifact scope story:version-persisted-contracts --add docs/epistemic-knowledge-runtime-design.md
aep plan artifact scope story:version-persisted-contracts --add systems/ekr/domains/graph.yaml
aep plan artifact scope story:version-persisted-contracts --add systems/ekr/domains/kernel.yaml
aep plan artifact scope story:version-persisted-contracts --add systems/ekr/domains/store.yaml
```

## What remains unestablished / coordinator decisions

- The coordinator has not adopted outer order/duplicates and canonical empty-entry rejection; the
  text above is one concrete proposed contract, chosen to preserve current ordered draft values.
- Strict duplicate semantic-key rejection is proposed explicitly; it is not silently charged to
  an existing acceptance or represented as a measured runtime defect from this read-only task.
- The exact legacy module layout and inventory public API do not exist; names are inferred.
- No authoritative live legacy source, complete exported history, auxiliary operation/receipt
  archive, or source list was inspected. A converter's successful coverage cannot be asserted.
- No upstream read-only inventory or atomic blob+append capability was found in pinned 0.2.1;
  selecting/implementing/releasing them remains coordinated cross-repository work.
- The prototype format unit cannot prove application/removal of property values, durable receipts,
  complete migration or cutover. Those writer/publication obligations remain pending.
- Frozen vectors from the completed seed unit need its final integration commit as provenance;
  using the old opening HEAD alone would cite code that did not contain the seed format.
