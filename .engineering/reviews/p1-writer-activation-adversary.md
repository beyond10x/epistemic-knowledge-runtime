unit: planned V2 activation patch, SHA-256 1ee49a48fa1afbb6b24d8e1415e69a5135846233e7c34cecba5d9fe2ec4824a1
verdict: NEEDS-CHANGE — four concrete declaration defects before source dispatch
cases: executed 0→0, red 0
origin: introduced 1 / pre-existing 3 / undecided 0
wrote-outside-worktree: 9 paths, listed below
needs-coordinator: reconcile command retries, object schema dispatch and missing-revision refusal; resolve the installed ESS silent-outcome capability gap

`git --no-pager diff --stat` in the coordinator tree at handback:

```text
.engineering/planning/journal.jsonl              | 1 +
 .engineering/planning/story/source-guard-debt.md | 4 +++-
 2 files changed, 4 insertions(+), 1 deletion(-)
```

These are the coordinator's concurrent planning edits, observed unchanged across this review. This reviewer made no repository/AEP/source/test edits and did not apply the activation patch. The explicit assignment was a read-only contract review, so executable runtime cases and Cargo were not run. The one ESS scratch probe below is a compiler capability measurement, not a kernel test or runtime finding.

Owners: 4 contract findings, all coordinator-owned declaration/coordination issues; 0 production implementor findings. Three inherited command contracts conflict with the adopted durable behavior; one format-dispatch omission is introduced by the activation patch. The preparation worker translated the authoritative decisions, but those decisions must also be reflected consistently in executable command contracts before dispatch.

## Inputs and scope

Reviewed the exact five-file activation patch and proposed DESIGN §§88–91, graph/kernel/ontology/store declarations, the complete preparation/acceptance reports, authoritative p1-durable-record-decisions.md, p1-persisted-contract-amendment-draft.md, relevant original DESIGN §§8,19–21,34,36,56–57,65, current production types/seed/validation/store publication and frozen original codec boundaries. The existing generated kernel suite was inspected as data; it was not executed or edited.

Both the coordinator's p1-writer-activation.patch and scratch activation.patch had the header SHA-256. The recorded preparation baseline is a2593122b04c3b422cf685ba7fc0db2444e3b3a5. Coordinator HEAD at review was 4ae2023. Finding locations below refer to the **proposed files**, not to an applied repository patch. Baseline command snippets were compared separately to classify origin.

## Findings

| ID | Location | Severity | Verdict | Origin | Defect |
|---|---|---|---|---|---|
| F1 | systems/ekr/domains/kernel.yaml:938, with input at :915 and lifecycle commit transition | High | NEEDS-CHANGE | pre-existing | Public Commit retries are required to fail after the first successful commit, contradicting the adopted return-of-own-result contract. |
| F2 | systems/ekr/domains/kernel.yaml:841 | High | NEEDS-CHANGE | pre-existing | Seed unconditionally reports AlreadySeeded whenever any revision exists, including an exact retry whose retained Root0 must be returned. |
| F3 | systems/ekr/domains/store.yaml:96 and docs/epistemic-knowledge-runtime-design.md:3854 | High | NEEDS-CHANGE | introduced | Removing ObjectStored.bytes changes its persisted shape without assigning the new object-event schema/version or exact old/new dispatch. The schema2 rule currently names revision-log facts only. |
| F4 | systems/ekr/domains/kernel.yaml:879 and :894, with RejectionRecordV1.requested_basis at :396 | Medium | NEEDS-CHANGE | pre-existing | Validate accepts an arbitrary named revision but has no outcome for an absent revision; a complete rejection basis cannot be fabricated when that revision does not exist. |

### F1 — Commit must distinguish retained success from invalid transaction states

**What was measured:** The generated scenario
`ekr.kernel.GraphTransaction/state/Committed/refuses/ekr.kernel.Commit`
executes Propose, Validate, Commit, then Commit again with the same captured transaction_id and actor. It explicitly expects wrong-state and TransactionStateConflict on the second call, with no events. Commit has only transaction_id as input; no declared request coordinate distinguishes those calls.

Proposed DESIGN :3837–3839 and the adopted decisions require a successful commit retry to resolve its exact committed occurrence and return that occurrence's own retained result, including after the head advances. Returning the prior result needs no second event; it cannot also return the required wrong-state error.

**What reaches it:** An ordinary caller retrying Commit after a lost success response or restart, using the only public command input it possesses. This is a directly generated command path, not a manufactured receipt or substitute authority.

**Origin:** The baseline already has the restrictive lifecycle/from set and wrong-state response (baseline kernel.yaml:386). The new activation retains that inherited declaration while adopting the stronger durable retry obligation. The coordinator independently confirmed the conflict and explicitly rejected a transport-only interpretation.

**Minimum correction:** Add an explicit successful retained-result path for Committed while preserving refusal for Proposed, Rejected and Stale. Return the original root/receipt and emit nothing; re-synthesize the corrected declaration instead of editing the generated scenario. Keep exact-byte/EventId mismatch refusal. Authored acceptance must repeat after an unrelated later head advance and verify the original result rather than the newest head.

### F2 — Seed's already-seeded condition also catches legitimate exact retries

**What was measured:** Seed input is only seed_document. Its already-seeded condition is exactly “a committed revision already exists,” and its output is AlreadySeeded. The new seeded outcome always creates Revision and emits Seeded; there is no silent successful result path. Proposed DESIGN :3837–3838 and the adopted Initialize contract require returning the original SeedResultV1 Root0 on a successful retry even after ordinary commits advance the head.

**What reaches it:** Seed succeeds, its response is lost or the caller restarts, and the same seed input/anchor is submitted again. Existing lineage makes the declared error branch true. Advancing the head first does not change the required own-result outcome.

**Origin:** Baseline kernel.yaml:272 already uses the unconditional existing-revision condition. The activation keeps it while requiring immutable seed retry results.

**Minimum correction:** Define an exact matching retained-seed/host-anchor success path that returns the original Root0 without events or object changes. A different seed or incompatible trusted anchor remains a refusal. The comparison must reuse the retained committed_at/result instead of sampling a new timestamp and making the retry different by construction. Re-synthesize the already-seeded condition and stage a genuinely different seed for that refusal scenario; do not force a same-input retry to fail merely to satisfy the old scenario. Add post-head-advance retry acceptance through the real shared Seed handler.

This is separate from F1 because Seed lacks Commit's existing transaction identity as a subject selector; the ESS/native representation needs its own honest binding.

### F3 — ObjectStored changes require their own explicit version dispatch

**What was measured:** Existing eventlog.rs:398 publishes `NewEvent::new(OBJECT_STORED, 1, body)`; initialization's :628–632 does the same. That schema1 body includes the required raw bytes field. The proposed store.yaml deletes bytes and retains only content_hash, storage_class, byte_len and stored_at. Proposed DESIGN :3534–3541 specifies schema2 for the six revision-log facts; it does not assign a schema to metadata-only object events. The new ObjectStored clause at :3854–3857 likewise omits this discriminator.

**What reaches it:** Every first content-object publication after activation, including seed envelope/evidence records, and every later inventory/reopen inspecting an existing object stream. Retaining the current schema1 constant while dropping bytes would reuse one persisted discriminator for incompatible meanings. Readers must not dispatch by permissively guessing from field absence.

**Origin:** The patch introduces the metadata-only body and coordinated blob path; existing source was consistent about its original schema1 inline body. The missing version assignment is therefore introduced in this declaration patch.

**Minimum correction:** State that metadata-only ObjectStored uses backend schema2; original inline ObjectStored/schema1 remains frozen verification/migration-only after activation. Refuse unknown versions and old/new shape or blob-binding mismatches. Explicitly retain schema1 for unchanged ObjectRetentionRaised semantics, rather than implying every store event moves together. Bind these exact per-event version rules in real provider readback/legacy refusal tests. The existing rule that EKR ContentHash and provider blob address must be verified separately is already present and should remain.

### F4 — Missing Validate revision cannot produce a legitimate RejectionRecord

**What was measured:** Validate inputs include an unrestricted revision number. Its only outcomes are validated, rejected and wrong-state. The rejected path requires at least one deterministic issue “against the named revision” and persists RejectionRecordV1 with a required complete ValidationBasisV1. RevisionNotFound exists but is used only by Snapshot.

**What reaches it:** A store at revision0 has a valid Proposed transaction, and the caller asks Validate against revision1 (or any absent revision). The transaction really is Proposed, so wrong-state is false. There is no prior root/receipt for revision1, so neither a valid receipt nor the mandatory requested_basis of a rejection record can be constructed truthfully.

**Origin:** The absent-revision outcome gap is present in the baseline Validate declaration. Complete durable basis/terminal-record requirements make its consequence explicit; this report does not claim a newly executed runtime panic.

**Minimum correction:** Declare named RevisionNotFound before validation, leave the transaction Proposed and emit no Rejected fact when the basis is absent. Existing historical revisions may still supply complete bases and later become Stale at Commit; do not replace that behavior with a blanket “must equal head” validation restriction. Add both absent-revision no-write and older-existing-basis controls.

## Installed ESS capability probe

Applied the ESS specification workflow to an independent scratch copy only. Read the available source's SubjectState, StateChange and Preserves definitions and its subject-state tests. The source contains `preserves` as an ess/6 operation, but source availability does not establish the installed binary's capability.

The scratch copy changed its root format from ess/1 to ess/6 and added this branch beside Commit's original outcomes:

```yaml
- name: already-committed
  when_subject_state: Committed
  preserves: ekr.kernel.GraphTransaction
  instance: transaction_id
  summary: Return this transaction's retained result without changing state or emitting an event.
```

Command from this report directory:

```sh
ess specify validate --path retry-probe
```

Observed with `ess 0.26.0`: **exit1**, retained verbatim in retry-validation.log:

```text
retry-probe was refused:
  - domains/kernel.yaml: unknown field `preserves`, expected one of `name`, `when`, `when_subject_state`, `when_state_changes`, `external`, `wrong_state`, `refuses`, `creates`, `moves`, `updates`, `instance`, `emits`, `payload`, `sets`, `error`, `summary`, `refs`
```

This does not prove the candidate branch's later state-partition semantics: deserialization stopped first. No suite was generated from this refused probe. The minimum required capability is an explicit successful, observable outcome that preserves a selected existing subject's fields/state and emits no event, without making every invalid state succeed. A published compatible compiler/runner or a supported equivalent must be established before using it. Global wrong_state/refuses:false would also accept Proposed/Rejected/Stale for Commit and is not a repair. Seed additionally needs a truthful retained-subject/identity binding for its document-only public input; that was not designed in this bounded review.

The coordinator reports a newer published ESS 0.28.0 release (2026-09-21), with clean source at 16aa8c7617214420d7d7f2108d0a896a5ed14eb0, and is preparing an isolated verified executable. This review's refusal is an **installed-0.26 capability gap**, not evidence that a published capability is missing or that upstream implementation is required. Version0.28 was not executed in this review.

The coordinator owns capability adoption and the exact declaration correction. Do not hand-edit the old suite, claim source-only ESS support as installed support, or narrow public retries to an internal transport loop.

## Controls and bounded conclusions

The reviewed patch already distinguishes exact proposal bytes from canonical value hashes, retains all transaction operation payloads, keeps the frozen original hash scheme, adds record_hash to event canonical encoding, carries prior receipt/context/time in validation basis, preserves full ontology and registered authority, and separates graph projections from kernel receipt implementations. The canonical value byte projection is explicitly admitted-only; invalid Float proposals remain exact YAML bytes. No additional concrete field-loss or upward Rust dependency defect was found in those bounded checks.

The original graph/seed/event formats remain frozen verification/migration-only in the stated contract. No production activation, runtime conformance, successful migration or provider atomicity is certified here. Generated ESS success and proposed runtime case names remain preparation, not execution.

## Retained evidence and handback

All writes are beneath `<cache>/ekr-completion-20260922/writer-contract-preparation/adversary/`:

1. report.md
2. static-observations.json
3. retry-validation.log
4. retry-probe/system.yaml
5. retry-probe/components.yaml
6. retry-probe/domains/kernel.yaml
7. retry-probe/domains/graph.yaml
8. retry-probe/domains/ontology.yaml
9. retry-probe/domains/store.yaml

The JSON observations record the exact generated retry command/error sequence and the static version/basis facts. Only the scratch system/kernel declarations differ from the copied proposal. No live stores, credentials, private source material, build target, lease, compiler process, repository or AEP mutation was created by this review. One attempted git diff from the non-repository scratch directory returned usage/129; the handback diff above was then obtained from the actual coordinator tree with exit0.

Return for coordinator correction before production source dispatch, followed by re-synthesis and bounded independent re-review of the corrected declarations.
