---
format: aep.planning-md/2
id: review-result:p1-format-refresh
kind: review-result
status: active
title: Review format multiplicity and production activation split
relations:
- reviews: story:version-persisted-contracts
revision: 1
---
unit: persisted-contract amendment draft and proposed format refresh; static contract review
verdict: NEEDS-CHANGE
cases: executed 0→0, red 0; implementation and acceptance below are unexecuted
origin: introduced 1 / pre-existing 0 / undecided 0; origin refers to the proposed dispatch split
wrote-outside-worktree: 1 assigned report
needs-coordinator: choose the V2 production-publication boundary before dispatch and preserve writer prerequisites

No repository diff was authored. This review read the coordinator draft, the complete refresh,
the two relevant stories and current integrated seed/graph/kernel/store source. It ran no build,
probe or live-store operation and made no source, test, documentation or planning edits. This is
not another seed review, runtime defect claim or full-project approval.

## Finding: the pure lane needs an explicit production boundary

**Blocker for dispatch of the split as written; NEEDS-CHANGE; introduced by the proposed split.**
`<cache>/ekr-completion-20260922/persisted-contract/format-refresh.md:193` assigns current V2
graph/seed conversion to lane A, while :177–181 forbids production publication through an inline
payload fallback before atomic blob publication. The amendment draft :98–109 simultaneously makes
seed/1 legacy-verification/migration-only. These obligations are compatible only if the transition
point of the production seed API is specified; neither document chooses that point.

The reachable coupling is concrete: `crates/ekr-kernel/src/commit.rs:339–347` immediately passes
every successfully admitted SeedDocument into Initialize. The existing implementation constructs
ObjectRecord with `bytes: bytes.to_vec()` and publishes it as ObjectStored schema 1
(`crates/ekr-store/src/eventlog.rs:618–633`). Replacing the current graph/seed model in lane A while
leaving that caller wired would publish new-format seed bytes through precisely the inherited
inline path that the refresh says the new publication must not use. A pure validator passing does
not demonstrate that the public entry point remains unwired. No such implementation was executed.

**Acceptance correction:** choose and state one intermediate contract before implementation:

- Prefer preparatory V2 models/encoders/validation entry points which are not yet connected to the
  public production publication path; switch that path only with lane D's atomic content capability.
  Explain which existing API remains available during this preparation and do not claim the normal
  V2 cutover has already happened. Shared-current-type edits may need to be staged together with D.
- Alternatively, explicitly refuse V2 publication with a named publication-capability-unavailable
  result until D is available. Verify this through the public entry point and both provider lanes,
  with no object metadata, revision event or blob binding written. This is an intentional temporary
  availability change and must be recorded, not inferred by the implementor.

Do not resolve the gap with a substitute seed authority, a new inline ObjectStored fallback or
put_blob-then-append. Pure validation and record-transport tests can still progress separately.

Owners: 1 coordinator-owned dispatch-contract finding; 0 findings against the completed seed
implementation. The finding's origin is the proposed split, not a claim about historical code origin.

## Multiplicity and temporal semantics: no contradiction found

The chosen outer ordered vector is compatible with the current proposal representation:
NodeDraft, EdgeDraft and PropertyMutation already carry vectors and encode their order
(`transaction.rs:72,83,100,332–360`). The unordered operation-set interpretation at :174–178 does
not erase order within an operation's value list. Current cardinality counts members and checks
every member independently (`ekr-ontology/src/check.rs:160–187`); One permits zero/one and required
is a separate rule (`value.rs:29–48`). No existing uniqueness constraint was found that would
require deduplication.

Rejecting explicit empty outer vectors at the persisted canonical boundary is a new, explicitly
proposed representation rule. It does not contradict allowing zero-value proposals to produce
absence after validation/application. The refresh correctly leaves application proof to the
writer. One empty inner List remains one present value, while an empty outer vector represents
zero; the proposed array/tag examples distinguish them without overloading Value::List.

Wrapping legacy values exactly once preserves old scalar/List/empty-List information, including
order and duplicate inner members. It cannot recover multiple values an old writer never retained;
the named missing-history refusal is required, not permission to call a partial snapshot a migration.

The refresh preserves the corrected §88 distinction between assessment, lifecycle, valid time and
revision-selected transaction history. Nothing new contradicts design §§14,34,36,65. The existing
draft's supersession eligibility and exact seed nesting remain necessary acceptance; they were not
executed by this static review.

## Two bounded acceptance recommendations, unexecuted

1. **Freeze old transaction/operation encoding as well as old graph records.** Refresh :122–134
   names GraphV1/NodeV1/EdgeV1/AssertionV1/events/seeds, and :220–224 requests validation vectors.
   Explicitly include GraphTransactionV1/GraphOperationV1 and their transitive canonical inputs,
   or a stated equivalent verifier over original retained canonical bytes. Current AddAssertion
   delegates directly to Assertion::encode (`transaction.rs:294–296`), and the old validation hash
   hashes transaction encoding plus revision in the payload domain (:221–224). Freezing only node
   creation vectors would miss a changed assertion encoder inside a transaction. Require at least
   an old AddAssertion transaction with its original operation payload, canonical bytes, operation
   address and validation hash; new encoding must not certify it. This strengthens the already
   stated frozen-encoder obligation; it is not a measured runtime failure.
2. **Specify Initialize's complete occurrence input, not only an EventId argument.** The refresh's
   “identical record” wording correctly requires the complete Seeded payload to stay fixed. Make
   the port receive that complete record, check that it is Seeded and its seed_hash matches the
   validated retained bytes, and preserve RevisionId as well as EventId across retries. Current
   Initialize constructs RevisionId and Seeded internally (`eventlog.rs:574–579`); leaving payload
   construction there while adding only EventId would not establish complete-record retry identity.
   Add a wrong-event-kind, wrong-seed-hash and same-EventId/different-payload refusal selection.
   Keep EventId occurrence identity separate from atomic-group transport identity: object-contention
   retries may alter object append entries, so they cannot blindly reuse a group key with changed
   group content. This needs agreement with the atomic publication owner before D is wired.

## Dependency and preservation disposition

Separating immutable-byte format verification from live source inventory is sound. A result must
identify supplied records and missing evidence, never imply export completeness or inspected live
stores. The refresh retains original bytes, backend IDs/name/schema/position, old addresses and
manifest-mapped new identities; unsupported/missing evidence remains an explicit refusal. It also
correctly prevents recovery of erased V2 content from historical inline payloads.

Preserve these dependencies when splitting the current combined story:

- The writer stays blocked on integrated A/B format/occurrence semantics, C source-inspection
  preservation evidence, and D atomic blob publication. These are not discharged by a pure fixture
  verifier returning a missing-evidence refusal.
- Complete preserving migration E depends on the writer plus C/D, because restart equivalence
  requires real application and receipts. Do not also make the writer depend on completed E; that
  creates a dependency cycle. Keep E as the final preservation/cutover acceptance and preserve the
  existing combined story's obligations until their new owner/dependency is recorded.
- Current writer story depends only on the combined version-persisted-contracts story for these
  corrections (`commit-and-revision-lineage.md:13`); narrowing that story without adding explicit
  C/D owners/edges would remove the barrier. No planning changes were performed here.

Minor wording correction: refresh :196 and its closing caveat use “release/pin” or “releasing” for
the upstream prerequisite. The coordinator's current instruction accepts a verified published main
commit. Say “verified published capability at an exact commit and coordinated downstream pin”; do
not create a tag/release approval dependency that was not requested.

All proposed additions still require coordinator adoption and executable acceptance. The sole output
is `<cache>/ekr-completion-20260922/persisted-contract/format-refresh-review.md`; no lease or process
was needed for this read-only review.

```findings
- file: format-refresh.md
  line: 193
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The pure format lane changes seed admission without specifying how the existing inline production publication path remains unavailable to V2 until atomic blob publication is ready.
```
