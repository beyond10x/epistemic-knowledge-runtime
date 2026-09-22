unit: EKR retained-replay activation proposal, compared with correction/proposal on 2026-09-22
verdict: nothing found
cases: executed 0→0, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 static review report
needs-coordinator: compiler synthesis and real-handler acceptance remain required before activation

No repository diff was authored. This is a static review of the prepared documents, not a compiler run, source implementation review or runtime gate. Only this report was written; no source, AEP, build, provider or worktree action occurred.

No concrete contradiction was found in the bounded changes. Original activation F1 (Commit retry/state partition) and F2 (document-only Seed retry/subject identity) are now represented in the draft rather than left as UNMAPPED comments. This does not establish that the pending compiler accepts or correctly synthesizes them.

## Compared scope

A recursive read-only diff against the predecessor correction proposal showed changes only in:

- `systems/ekr/system.yaml:1`: ess/6 becomes ess/7.
- `systems/ekr/domains/kernel.yaml`: Seed replay, Commit guarded success/replay/default refusal, and the complete Revisions view.
- `docs/epistemic-knowledge-runtime-design.md`: frozen parser profile and activation/retry observation requirements in §91.

The graph, ontology, store and component files were unchanged. The diff command exited 1 because those expected differences exist; no compiler or test status is inferred from it.

## Retry and subject checks

| Requirement | Static result and exact draft citation |
|---|---|
| F1: Commit returns its own retained result | `kernel.yaml:945–950` retains only transaction_id input and CommitReceiptV1 response. `:981–984` declares Committed plus `replays: committed`, explicitly returning this transaction's own receipt after head advancement without event/object publication. No current-head lookup or newly supplied identity is declared. |
| F1: other states still refuse | GraphTransaction has exactly Proposed, Validated, Committed, Rejected and Stale (`kernel.yaml:690–706`). The normal committed branch is explicitly Validated (`:952–955`); retained-commit is explicitly Committed (`:981–983`); the ordinary error default names TransactionStateConflict for the remaining three (`:985–987`). No broad wrong-state success or additional accepted state was added. |
| External stale result remains distinct | `kernel.yaml:967–980` retains the external canonical-head-change outcome and its existing stale transition, whose source is Validated (`:704–706`). It is not the ordinary default or the Committed replay. Actual compiler partition and runtime race handling still need execution. |
| F2: Seed remains document-only | `kernel.yaml:831–836` contains only seed_document input and SeedResultV1 response. `:854–857` replays seeded with the same parsed full Seed2 input and actual bootstrap/host authority context. No revision, caller authority hash or synthetic identity input was added. |
| F2: original revision identity exists before retry | The original seeded outcome creates Revision with revision_id, publishing that generated identity in Seeded (`kernel.yaml:838–848`). `replays: seeded` refers to that origin. DESIGN `:3932–3934` explicitly binds Seed to the original emitted revision identity and Commit to the original transaction input. This depends on the new compiler preserving the origin binding, not inventing a binding from the input path. |
| Seed differences and time allocation | `kernel.yaml:850–857` distinguishes different parsed input/context from retained success. DESIGN `:3704–3711` requires comparison before new occurrence/time allocation, original Root0 and committed_at even after head moves, no writes, and whitespace-insensitive logical seed comparison. The separate exact transaction-byte requirement remains intact. |
| Complete original seed subject snapshot | Revision declares identity, eight data fields and Committed state (`kernel.yaml:729–753`). Revisions exposes all ten corresponding entries (`:1165–1188`): revision_id, number, parent, ontology_root, knowledge_root, evidence_root, agent_root, transaction_id, committed_at and state. It has no filter. It does not reuse the partial CurrentRevision view at `:1190` as the claimed complete snapshot. |
| Complete Commit subject snapshot | Transactions (`kernel.yaml:1130–1161`) exposes GraphTransaction's identity, all twelve declared data fields and lifecycle state (`:653–681`). It is unfiltered; terminal states remain observable. PendingTransactions remains a separate filtered view, not the replay snapshot source. |
| Actual result and adapter fidelity | SeedResultV1 and CommitReceiptV1 retain their complete declared payloads (`kernel.yaml:327–374`). DESIGN `:3937–3946` requires actual kernel-derived responses, complete real query observations, lossless typed Integer conversion and exact JSON decoding or Unsupported. It does not equate native typed parity with arbitrary JSON numeric-spelling parity. |

The public command set remains Seed, Propose, Validate, Commit, Snapshot and Explain in the unchanged components declaration. Revisions is a declared query surface, not an invented seventh CLI command.

## Frozen parser comparison

DESIGN `:3713–3765` agrees with the coordinator's `.engineering/waves/p1-durable-record-decisions.md`, “Preserve the exact submitted document” and “Frozen parser profile for transaction-document/1”:

- Same versioned YAML envelope, typed operation tags and limited JSON-compatible syntax; one UTF-8 document, strict semantic fields and actual typed map-key uniqueness.
- Same inclusive limits: 262144 raw bytes, depth 32, 32768 expanded nodes, 4096 map/sequence entries, 65536 string bytes, 4096 key bytes, 1048576 cumulative string bytes, 1–256 operations and at most 1024 evidence input elements.
- Byte cap before unrestricted parsing/reading; budgeted representation and explicit container shape checks before strict typed decoding from the original bytes. Empty plain scalars cannot impersonate empty required collections. Arbitrary unique Record names, explicit empty inner Lists/Records and merge-key text remain data; no YAML merge expansion.
- Aliases charge their expanded position/content with checked counters. The admitted profile is selected by document version in proposal, validation and replay. A stricter host upload cap changes only ingress; changing the frozen profile requires an explicit version decision.
- The eager YAML event loader is acknowledged. The text does not falsely promise exact allocator-byte bounds or pre-loader event/scalar quotas from post-loader visitors.
- Exact supplied bytes, including whitespace and well-formed nonfinite Float spellings, remain retained. Float semantic refusal belongs to validation; a JSON round trip cannot manufacture the original document.

The coordinator decision additionally incorporates the readiness report's detailed counting conventions and boundary controls. The implementation must use those adopted conventions for depth, expanded nodes, strings/keys and evidence input elements; the shortened numeric list in DESIGN is not permission to choose a different counting model.

## Still unexecuted

1. Run the exact candidate through the verified released ess/7 compiler and inspect its synthesized suite. Confirm both replay scenarios capture a real original invocation, bind its original identity, observe every subject field/state before retry, compare the full response, require zero direct events and avoid ConfigureExternalOutcome for replay. The compiler must select the complete Revisions/Transactions observations rather than silently using the partial views.
2. Confirm generated ordinary Commit refusal cases cover Proposed, Rejected and Stale individually, with unchanged subject and no events; Committed must not fall into that refusal. Keep the external stale branch distinct and verify its Validated source. These declarations alone do not prove the new default partition implementation.
3. Execute the real public handlers through both providers after restart and unrelated head advancement. Measure original receipt/time/occurrence and object/event counts. Seed controls must cover equivalent whitespace, changed parsed input and changed actual bootstrap/authority context. Commit controls must cover all three refused states and the original transaction's result.
4. Execute frozen-profile exact/over-limit, duplicate, shape, alias, NaN retention and changed-ingress-limit replay cases, plus the actual adapter's adjacent-large-i64/endpoints mutation controls. Nothing in this review certifies parser allocation ordering or adapter losslessness.

These remaining obligations are already explicit in DESIGN `:3947–3950` and `:3960–3982`; no new scope is demanded by this review. The draft correctly remains unactivated until that evidence exists.

Owners: the coordinator owns adoption, compilation/synthesis and the original F1/F2 contract correction; the compiler unit owns its actual replay/default semantics; EKR writer/shared-handler/query and conformance owners supply the real runtime observations and authored acceptance. This review supplies no approval or verifier evidence and does not erase the original findings.

```findings
[]
```
