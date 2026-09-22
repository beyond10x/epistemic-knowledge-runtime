# Bounded writer readiness — 2026-09-22

Read-only source inspection of coordinator HEAD `594efa87e939d667d3634b17fe8b224b3c1e355b` plus its current uncommitted Eventlog pin `4ee3dc23f0d02a5726a0e41d097477791f09efe2`. The synchronous store-runtime refusal is integrated; the Eventlog pin is not evidence that EKR has adopted atomic blobs. Inputs: `p1-writer-preparation.md`, the overriding `p1-durable-record-decisions.md`, and the corrected activation proposal's DESIGN §§88–91 and ESS. No builds, Cargo commands, AEP commands, provider access, worktree operations or repository edits. Only this report was written.

Owners: this inventory — read-only reviewer; contracts, dispatch and integration — coordinator; future implementation — its assigned source owner. All application/replay recommendations below are unexecuted acceptance requirements, not implementation evidence.

## Admitted operation inventory

`crates/ekr-kernel/src/transaction.rs:122` declares eleven variants. The fixed seven-validator pipeline is `validate/mod.rs:111`; successful validation narrows every value then seals at `:171`. Seven variants can presently pass. “Admitted” below means some correctly formed proposal passes all validators, not that the current Commit actually applies it.

| Index / variant | Current admission and exact application obligation |
|---|---|
| 0 CreateNode | Admitted with fresh identity, matching graph root, declared concrete node type, admissible/typed properties and candidate cardinality. Create its complete record; initialize `type_state` from the declared lifecycle's initial state, or None. Preserve every outer property member. |
| 1 UpdateProperty | Admitted for an existing or same-transaction-created node and declared property. Values replace the entire outer collection; empty clears an optional property by removing its key, while required zero refuses. Two different assignments to the same node/property refuse; identical assignments are not conflicting under the current rule. |
| 2 CreateEdge | Admitted with fresh identity, matching graph root, compatible surviving endpoints, declared edge type, typed outer properties and post-state outgoing-edge cardinality. Materialize the complete edge unless the same atomic set cancels it. |
| 3 DeleteEdge | Admitted for an existing edge or one created in the same set. Remove it from final structural state. Creating and deleting one edge is cancellation; an added or retained assertion referencing that edge makes the transaction refuse, even if the assertion is also retracted. |
| 4 AddAssertion | Admitted only with Proposed assessment, retained nonempty evidence, matching graph root, valid subject/predicate/object typing and resolvable references. The transaction evidence manifest must equal the union cited by added assertions. Apply canonical narrowing, establish Accepted with the actual validator set, Active lifecycle and trusted commit transaction-time fields. Do not copy a caller verdict or trust caller attribution. |
| 5 RetractAssertion(AssertionId) | Presently admitted once the target is an existing or same-set-added assertion. Current payload has no reason and the validators do not implement the new lifecycle conflicts. V2 must replace its payload at index5 with assertion identity plus reason, preserve assessment/evidence/proposer, set Retracted at the committed revision and close recorded_to without deleting history. The old payload cannot satisfy the accepted contract. |
| 6 DefineNodeType | Explicit P1 `unsupported-operation` refusal after precise structural diagnostics. No durable schema mutation. |
| 7 DefineEdgeType | Explicit P1 `unsupported-operation` refusal. No durable schema mutation. |
| 8 ModifyProperty | Explicit P1 `unsupported-operation` refusal. No durable schema mutation. |
| 9 MergeEntity | Explicit P1 `unsupported-operation` refusal when endpoints resolve; malformed references still give their named refusal. No durable merge. |
| 10 Invoke | Admitted when the target/type declares the operation, argument names/types match, applicable property constraints are absent, preconditions/emits are empty, and any transition is allowed. Apply the declared lifecycle transition from the existing state or declared initial state for newly created/state-less nodes. A declared operation with no transition/effects may legitimately leave graph content unchanged; do not invent property writes. Multiple lifecycle-moving Invokes on one node refuse. |

Admission sources: `validate/structural.rs:224–284` (competing writes and four unsupported families), `reference.rs:79–174` (references/cancellation/history), `types.rs:76–389` (all value paths, declarations and Invoke arguments), `cardinality.rs:45–126`, `ontology.rs:60–177` (constraints/effects/transition), `provenance.rs:75–111`, and Authorization in the fixed pipeline.

V2 adds **SupersedeAssertion at index11**, so the completed live operation set has twelve declared variants/eight admitted families, with the same four P1 refusals. Do not interpret “seven currently admissible variants” in the older preparation as permission to omit supersession from story completion. Both accepted documents explicitly require it.

## Reuse the agreed candidate, not vector-order semantics

`validate/candidate.rs:14–118` is the shared source of node identities/types, surviving edges, available deletion targets, node property counts and outgoing-edge sets. It inserts all creations first and applies updates/deletions afterward. `node_types` (`validate/mod.rs:205`) delegates to it. Reference, Types, Cardinality and OntologyConstraint already consume that shared view.

This is **not yet a complete graph applier**: it stores only each edge's type/source, not target/properties, and no assertion lifecycle or accepted assessment. Extract/extend its common projection rules for the pure kernel applier instead of building a second incompatible theory of final state. Keep these agreements:

- Same-set forward references are legal; operation permutation cannot change admission or final graph.
- Created-then-deleted edges remain valid deletion targets but disappear from final reference/cardinality indexes.
- Retraction preserves assertion references: deleting a referenced structural edge does not become valid merely by retracting its claim.
- CreateNode plus UpdateProperty on an initially absent property must use the final count. An explicitly supplied unequal create/update pair remains a structural conflict.
- Lifecycle Invoke uses the same ontology transition function and initial-state fallback as `validate/ontology.rs:159–176`; create-first application must not erase a validated transition.
- Current operation-vector order still participates in canonical transaction encoding. Equal resulting graphs under permutation do **not** imply equal transaction addresses (§91.5).

The new assertion candidate must additionally establish acceptance of newly added assertions before checking supersession eligibility. Validate the complete simultaneous lifecycle result, including replacement survival/readability, competing writes and cycles. Sequentially applying vectors would reject valid forward replacement or accept a replacement later retracted by the same transaction.

## Outer property multiplicity is an activation-wide change

Drafts already use `BTreeMap<PropertyId, Vec<V>>` (`transaction.rs:58–101`), and Types walks each member while Cardinality counts the outer vector. Stored `Node.properties` (`graph/node.rs:69`) and `Edge.properties` (`graph/edge.rs:46`) still use one scalar value.

Two important old-shape assumptions must not survive activation:

1. `validate/candidate.rs:46–54` counts every stored node property as exactly one. V2 must count the stored outer length.
2. `kernel/seed.rs:246–271` wraps every stored scalar as `vec![value.clone()]`. Applying that conversion unchanged to a V2 vector would conflate outer multiplicity with an inner List. Seed validation and narrowing must traverse the actual outer collection.

Accepted §89 is precise: preserve outer order and duplicates; absence is zero; an explicitly stored empty outer collection refuses; draft clearing removes the key. A single `Value::List([])` remains **one** outer value. Never flatten an inner List, keep only the first member, deduplicate, or silently store an empty outer collection. Reference/admissibility checks must reach late members, including nested NodeRef/Float paths. Legacy scalar-to-singleton conversion belongs only after original-byte/hash verification.

## Lifecycle, time and replay requirements already decided

The corrected DESIGN is internally consistent on the reviewed points. §88 preserves Accepted attribution separately from lifecycle; a superseded claim remains answerable inside its supported closed valid interval even though recorded_to is closed. Retraction suppresses all valid-time answers in that revision and later. The former and replacing subjects need not match (§65 Alice/Bob); normal typing/provenance still applies. Replacement must be distinct, Accepted in the fully validated candidate, begin at the boundary, survive and be eligible for valid_at(boundary). Self/cyclic replacement, unreadable/retracted replacement and conflicting lifecycle changes refuse. A boundary cannot precede the former start or extend an existing finite end.

New assertion recorded_from and lifecycle recorded_to come from the immutable trusted commit timestamp. Equal timestamps are legal; the current `TransactionTime::new` already admits equality (`assertion.rs:297–314`). Existing replacement assertions retain their original recorded_from. Earlier revision reconstruction retains its original graph rather than reading today's lifecycle state.

Current source deliberately disagrees with those **future activation** semantics:

- `graph/assertion.rs:543–620` derives lifecycle from the old assessment enum; it cannot preserve acceptance on retraction.
- `graph/snapshot.rs:75` filters `is_current && valid_time.contains`, excluding every old superseded record. V2 valid_at needs the explicit §88 predicate; do not restore the removed open-ended `active()` read.
- `kernel/commit.rs:61–115` keeps validation authority in process memory; `:277–330` writes three separate events and never applies operations.
- `store/log.rs:421–468` silently ignores unattested/stale commits; `:492–503` reconstructs a root with two zero placeholders and an operations-only transaction address.

These are scheduled replacements, not accepted-contract alternatives. The new authority must return `Result<AdmittedRevision, named refusal>` or an equivalent fallible kernel-owned result from complete immutable inputs. Both live commit and replay use the same pure validation/application/root computation. Retain and check the full proposal document, complete ValidationBasis, receipt addresses/occurrences, actual actors/profile, ontology/evidence and frozen timestamps. Every root field is independently recomputed. A malformed committed record refuses reopen instead of disappearing into a seed-only fold. Historical reads stop at the requested committed revision. No deserialized capability, process-local allowlist restoration or nested authority read through a provider transaction.

## Coordinated activation surface

These are exact current paths/callers, with inferred new modules distinguished:

| Surface | Files and required coordination |
|---|---|
| Stable occurrence identity | `crates/ekr-core/src/identity.rs`, `src/lib.rs`: introduce/export EventId; retain existing identity encodings. |
| Live graph representation | `crates/ekr-graph/src/assertion.rs`, `node.rs`, `edge.rs`, `canonical.rs`, `snapshot.rs`, `events.rs`, `root.rs`, `lib.rs`: assessment/lifecycle, outer vectors, complete canonical traversal, V2 occurrence envelope and read semantics. Review transient conversion callers in `transient.rs` as part of the generic property type change. |
| Strict graph/seed documents | `crates/ekr-store/src/snapshot.rs:26–90` and its value conversion helpers; `crates/ekr-kernel/src/seed.rs`: exact graph/2 inside seed/2 inside envelope/2; authority/context/time retention; seed own-result return; no original-format live ingestion. |
| Transactions and validation | `crates/ekr-kernel/src/transaction.rs`, `validate/mod.rs`, `candidate.rs`, `structural.rs`, `reference.rs`, `types.rs`, `cardinality.rs`, `ontology.rs`, `provenance.rs`, `authorization.rs`, `lib.rs`: reasoned retraction, supersession, exhaustive conversion/encoding/validator matches, full sealed basis and actual actor/profile. |
| Durable kernel orchestration | `crates/ekr-kernel/src/commit.rs` and `seed.rs`; inferred pure `apply.rs`, receipt/basis/authority modules and transaction-document parser as scoped by preparation: exact document retention, staged decisions, deterministic apply, retained own-result retries and full replay authority. Module names are not a new contract decision. |
| Ontology root | `crates/ekr-ontology/src/schema.rs` plus existing declaration encoder ownership: complete lossless ontology export/canonical form, all semantic fields including unused declarations, ordered by stable identities. No schema-ID-only hash. |
| Generic persistence | `crates/ekr-store/src/log.rs`, `eventlog.rs`, `objects.rs`, `lib.rs`: fallible authority/Initialize/publication ports, retained transaction records, historical selection, mandatory V2 record linkage, metadata-only ObjectStored/schema2 and atomic provider blobs. Update `append`, `initialize`, `put`, `store_graph`, `head/fold/replay` and readback together; retain runtime-context refusal before any provider/authority work. |
| Frozen original families | `crates/ekr-graph/src/legacy.rs`, `crates/ekr-kernel/src/legacy.rs`, `crates/ekr-store/src/legacy.rs` remain verification/migration types. Do not turn the scalar, old assessment, old event or historical validation-hash encoders into aliases of current types. Their pinned vectors/strict refusal cases must remain. |
| Coordinator-owned contract projection | Corrected DESIGN and `systems/ekr/domains/{kernel,graph,ontology,store}.yaml`, components/system as required, plus Cargo pin/lock. Runtime handlers used by future CLI/conformance must share the real document/receipt path, not a fixture authority. |

A reviewed published Eventlog commit is the accepted prerequisite; the preparation's older “released version” wording is superseded by the coordinator's explicit pin decision. Upstream availability does not authorize temporary inline V2 objects or put-then-append. Unknown commit outcome never authorizes deletion.

## Acceptance to migrate intentionally versus retain

**Expected compile breaks** are useful signals of the coordinated type change: scalar Node/Edge property literals; Assertion literals missing lifecycle; live uses of `ValidationState::Retracted/Superseded`; `GraphOperation::RetractAssertion(id)`; exhaustive live GraphOperation matches/eleven-variant rosters; old flat revision-event constructors and `Initialize(bytes, at)`/boolean authority mocks once the new ports land. Update live fixtures to their new meaning, while frozen `legacy::*` fixtures stay original. Do not keep an old public writer overload merely to make these compile.

**Replace obsolete expectations deliberately:**

- `store/tests/fold_rules.rs::two_of_the_five_sub_roots_are_the_placeholder_and_not_derived`: replace with complete ontology/authority root sensitivity, including unused declarations/profile changes.
- `fold_rules.rs::a_commit_whose_validation_the_authority_does_not_stand_behind_does_not_advance_the_lineage` and silent stale-commit controls: retain refusal intent but require a named replay failure, not a successful seed head.
- `kernel/tests/seed.rs::repeated_initialization_preserves_the_lineage_and_writes_no_second_object`: same full parsed seed/context/anchor now returns retained Root0; different input/anchor still refuses without a losing object. Add head-advanced retry and whitespace-equivalent parsed seed controls.
- Old `graph/tests/adversary_snapshot_and_assertion.rs::is_current_is_exactly_acceptance_and_an_open_transaction_time`, its source-clause guard and live assessment/status fixtures must be reconciled with stored lifecycle and §88's superseded-valid-time exception. Keep the no-open-ended-read behavior and earlier-revision distinction.
- `graph/tests/revision_events.rs` current event fixtures/encoding field expectations and store event-idempotency cases must use EventId/record_hash and exact occurrence equality. Equal content with different EventIds is two facts; identical occurrence retry is one.
- The current `kernel/tests/commit_path.rs::a_validated_transaction_commits_and_the_lineage_advances` proves only a revision number/head. Strengthen to changed graph, retained proposal, real authority and actual new-process restart through both providers; never call that old green case the writer acceptance.

**Retain substantive guarantees:** all seven membrane refusal families and permutation controls (notably `adversary_p1_07.rs` cancellation/reference/lifecycle cases); float-at-depth/admissibility equivalence; undeclared optional empty assignment refusal; canonical/reference compile-fail tests and kernel-only writer ownership; seed no-write, context/ontology/evidence tampering and concurrent losing-seed protections; complete current and frozen canonical-family rosters/hash-sensitivity cases; metadata retention monotonicity; runtime-context constructor/public-I/O/Drop behavior. Update fixture shapes without weakening those assertions.

**Required writer assertions:** every admitted family applies or has a named pre-seal refusal; Many/order/duplicates versus single empty inner List; optional clear versus required refusal; new+cancel edge agreement; initial state plus supported Invoke; accepted AddAssertion and same-set supersession under permutation; reasoned retraction preserving attribution; latest §65 handover versus earlier-revision reconstruction; complete roots and corrupt-input named replay refusals; proposed/validated/rejected/stale state survives restart; stale versus noncommitting stream contention; own-result Commit and Seed retry after head advancement; atomic blob/event publication and interruption reconciliation through both real providers.

## Dispatch residue

No new contradiction among the accepted DESIGN decisions was identified by this bounded source inspection. The current code's seven-family/old-format limitations are explicit activation work. The corrected proposal remains **not adoption-ready** for the already-known ESS retained-result capability: kernel.yaml comments at 854 and 983 still mark F2 Seed and F1 Commit UNMAPPED, and its old Commit wrong_state branch contradicts successful Committed retry until that capability repair lands. Do not treat a compiling partial ESS proposal as a complete normative activation contract. No new alternative or unrelated subsystem is required by this report.

Only outside path written: this assigned `writer-readiness/report.md`. No build or long-running process was started.
