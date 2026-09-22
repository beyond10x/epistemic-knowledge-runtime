# Seed/Propose conformance contract preparation

Source: the story-scoper's retained scratch report and measured candidate at the
seed-wave opening. The accompanying `p1-conformance-document-contract.patch` is
an unapplied proposal, not a patch to apply without reconciliation. In particular,
its Seed summary predates the correction separating retained ontology from the
future populated revision root, and its seed format must follow the subsequent
persisted-contract version decision. Runtime assertions in this report remain
unexecuted until the writer, shared document handler and actual target exist.

Owners: coordinator for ESS changes; CLI/submission implementor for real document loading and identity binding; conformance implementor for fixtures and Runner; writer implementor for retained transaction state.

This is a measured scope proposal, not implementation or execution evidence. Only scratch specification/scenario copies were changed. No build, source/ESS/AEP edit, upstream modification or kernel scenario execution occurred.

## Decision

Use named document-path inputs for the already approved Seed and Propose verbs. The production handler must read the specified document, parse it into the real seed/transaction type, bind host authority, and invoke the kernel. The hashes and operation count remain derived persisted/output facts. The target stages reviewed fixture files at the exact input paths ESS synthesized and invokes that same handler; it never converts an input string into an unrelated hash.

The candidate changes Seed input to `seed_document: ekr.kernel.SeedDocumentPath` and Propose input to `transaction_document: ekr.kernel.TransactionDocumentPath`, both named String newtypes. This is a document-location contract, not arbitrary JSON or an untyped replacement for GraphTransaction. Seed bytes still decode through `SeedDocument::from_yaml` into the active seed unit's `ekr-seed/1` structure, including complete ontology, graph and evidence payloads. Proposal bytes must decode through one declared versioned parser into the actual `GraphTransaction<Value>` and its typed `GraphOperation` variants (`crates/ekr-kernel/src/transaction.rs:122`, `:163`). The precise proposal document envelope remains a CLI/writer decision; do not invent a second graph model in the target.

Remove the false `input.seed_hash` and `input.operations_hash` event bindings, and the proposal's caller-supplied count/hash/proposer assignments. Retain those event and entity fields, with values obtained from the actual parsed submission and persisted result. Make `malformed` an external outcome: it depends on parsing/structural admission of the referenced file, not a supplied integer. This preserves both outcomes while accurately stating why a predicate over a path cannot decide them.

This needs no seventh P1 CLI verb. A view is a library read surface, not a new command. Document loading belongs to the shared submission handler used by the six-verb CLI and target; filesystem paths do not need to enter deterministic validators.

## Measured results

Installed CLI reports `ess 0.26.0`. Its binary SHA-256 is `bf7ed60017e6634c736e7b2fcdf97d0402feadd8983b74eb7b74903593d57b0d`. Source behavior was inspected at requested ESS revision `a5f1bea13294510819b266561c83be9509e6ba57`, whose Cargo version is 0.26.0. The binary's build revision was not independently attested; its exact hash and the complete input/output artifacts are retained here.

| Scratch input | Validate | Synthesize/author | Generated | Authored | Outside | Refused |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| `baseline-spec` | exit 0 | exit 1 | 26 | 0 | 0 | 4 |
| `document-path-spec` | exit 0 | exit 0 | 30 | 0 | 0 | 0 |
| candidate plus `authored/` | same candidate | exit 0 | 30 | 1 | 0 | 0 |
| `unsupported-replacement.yaml` author probe | same candidate | exit 1 | 0 | 0 | 0 | 1 |

All emitted suites use `ess-conformance/5`. No scenario passed, failed or skipped: none was run. The four additional generated scenarios are the formerly refused post-Commit committed/stale and post-Validate rejected/validated operation-count invariants.

Exact suite byte SHA-256 values:

- `baseline-suite.json`: `584c44ba09c979fd103af46787e15a2516f4e496d3feda62298a226cfd1bc69d`.
- `document-path-suite.json`: `58a1633e2086d71cee680ee1e3f0559f576f77b49e11cd69ad30d29be5e89874`.
- `document-path-authored-suite.json`: `a6233bfe466cc55e12490bd4fe6d9c3e10d6af63a6f4fe0a4f58a910817eb3a2`.

`contract-candidate.patch` is the exact measured proposal. The authored scenario is a compilable example of real path inputs, event capture, and persisted operation-count/state readback; it is not a completed runtime fixture or sufficient hash-coverage suite.

## Authored fixtures cannot replace generated obligations

At the pinned ESS revision, authored documents have their own `ScenarioId::Authored`; their closed Document schema has no replacement or generated-input override (`crates/verify/ess-conformance/src/authored.rs:25`, `:253`, `:1540`). `coverage_build` synthesizes generated obligations and then appends authored candidates (`:129`, `:178`, `:220`). `build_with_known_generated` retains excluded generated obligations in inventory; it does not establish that a different authored scenario proves them.

The measured `replaces: ekr.kernel.Seed/outcome/seeded` probe refuses with ESS-AUTHOR-001: unknown field `replaces`, expected type/domain/scenario/summary/arrange/timeline/assert. Authored-only selection is therefore not a repair for invalid generated inputs. Keep all generated obligations and add authored scenarios beside them. Do not modify admitted suite JSON after synthesis or reclassify generated scenarios as outside merely to obtain a green report.

The pinned type model supports structs and tagged unions (`crates/specify/ess-domain/src/types.rs:630`), so a later direct structured operation protocol can use those. It is a larger contract decision than implementing the already approved document CLI. No generic `Json` field is needed for this unit.

## Retained transaction view

Add an unfiltered `ekr.kernel.Transactions` view over GraphTransaction, `read_your_writes`, projecting transaction_id, proposer, operations_hash, operation_count, evidence_hash, validated_against, validation_hash and implicit lifecycle state. Keep PendingTransactions' Proposed filter unchanged. ESS validated this exact declaration and synthesized all four missing invariant scenarios.

The implementation must read these rows from retained kernel transaction records across Proposed, Validated, Committed, Rejected and Stale. A HashMap maintained solely by the test adapter is not that view. Derive operation_count from the retained operation payload or a verified persisted count, and retain original hashes and validation attribution after terminal outcomes. Reopen/readback belongs in runtime acceptance because synthesis alone cannot establish durability. This is a concrete writer prerequisite, not permission for the target to fabricate state from the last selected outcome.

The synthesized outcome scenarios also assert that Transactions contains the captured transaction id (committed additionally carries validated_against=1). The invariant-only scenarios ask that every row satisfies operation_count >= 1 and that the view is nonempty. Because revision-1 setup may retain another transaction, that invariant alone does not prove the just-completed row survived; retain the identity-specific outcome assertions and authored reopened-row checks as well.

## Exact target arrangement

Use a committed fixture manifest keyed by the exact admitted scenario ids. It selects concrete seed/transaction documents and required starting revision; it never contains results for execute/query to return. At `begin_scenario`, create a fresh temporary runtime and fixture directory. Stage only a reviewed allowlist of relative regular-file paths contained beneath that directory; reject absolute paths, parent traversal and symlinks. Fixture setup writes nowhere else. Production document reads remain governed by the real CLI path contract; do not impose the fixture writer's restrictions on arbitrary user reads by accident.

Observed generated inputs are `seed_document`, `seed_document-1` and `transaction_document`. The second seed path occurs in Seed's seeded scenario and must hold a real document for its second invocation. Do not generate files at unchecked arbitrary input strings. Unknown fixture paths must fail setup visibly.

ESS also supplies Validate.against=1 and Snapshot.at=1. Normal transaction/snapshot setup must reach revision 1 by a real validated commit after seed revision 0. Never remap 1 to 0. Seed scenarios start empty, except the already-seeded control performs a real initial seed. Explain's generated assertion id must be present in an actual validated fixture for explained and absent for not-found; do not translate it to another id.

For external controls, choose and establish real conditions:

- **invalid-seed:** a staged document with a concrete dangling reference or another deterministic bootstrap violation; actual Seed must refuse without persistence.
- **malformed proposal:** a staged empty-operation or parse-invalid transaction; the real submission path refuses it.
- **rejected validation:** the scenario manifest stages a structurally admissible proposal with an unresolved node reference at revision 1 before Propose runs. ConfigureExternalOutcome occurs after Propose, so it must not replace that already retained transaction. The control confirms the fixture condition; Validate produces the actual refusal and recorded state. In particular, deleting a node at revision 2 would not prove rejection when the command still asks to validate revision 1.
- **stale commit:** after the candidate validates at revision 1, execute an independent valid commit to revision 2 through the real kernel. The original commit attempt must observe staleness and its retained state/event.
- **missing revision/assertion:** prepare an actual absent identity/revision. No fabricated target error or modified request is needed.

The manifest should list and cover every generated external-control use; a new scenario without a reviewed fixture fails rather than being answered by a catch-all expected-outcome flag. Setup event history must be separated by actual observation cursors from command observations, while retained graph/transaction state remains real.

Bind ESS actor roles to independently supplied host execution contexts. The proposal document's proposer must equal its authenticated submitter or be refused; never choose a convenient id to satisfy the validator. Seed operator/validator and transaction validator identities come from host context, not YAML. This closes the CLI-owned `task:the-proposer-field-is-unauthenticated`; `validate/authorization.rs` explicitly says it currently compares one host identity with an unauthenticated payload identity. A real implementation of that binding is a prerequisite, not something conformance fixture bookkeeping can provide on its behalf.

## Restoring useful checks and finishing this unit

Removing incorrect input-to-hash mappings reduces those generated equality assertions. Compensate with authored known-fixture checks that compare actual emitted hashes, proposer and retained count to reviewed fixture constants, plus reopened Transactions readback. Fix host identities and versioned serialization when recording those vectors. A deliberately changed document must change the persisted operation hash; a bogus reported hash must fail a named authored scenario. The checked-in example currently checks only count/state because no authoritative final writer/seed hash vectors were computed in this read-only task.

Required mutation evidence after implementation: drop reference validation and watch the rejection scenario fail; remove stale detection and watch stale fail; corrupt a terminal operation_count to zero and watch its generated invariant fail; erase the committed transaction row and watch `ekr.kernel.Commit/outcome/committed` fail its captured-id containment; misreport a persisted hash and watch its authored check fail. These are required future observations, not results measured here. Use the pinned Rust ConformanceTarget and actual Runner, retain exact suite bytes plus report/2, and require all 30 generated plus the final authored set to execute and pass with zero unsupported/skipped/error. Counts may grow as the real contract is completed; preserve the inventory rather than fixing the current number forever.

Implementation surfaces: coordinator-owned `systems/ekr/domains/kernel.yaml`; shared CLI/submission handler and conformance adapter under `crates/ekr`; kernel retained transaction query and submission authority; writer's durable transaction records; `systems/ekr/conformance` scenarios/fixtures; `crates/ekr/tests/conformance.rs`; pinned ESS library dependencies in Cargo manifests; conform-check gate. Do not add a new upload/stage command. Recheck final seed document and transaction formats before recording the contract; the active seed implementation was read but is not a completed immutable dependency.

Remaining uncertainty: final proposal parser/envelope; final durable record read API and occurrence semantics; completed host submitter binding; authoritative fixture hashes; actual Runner execution. The scratch validation establishes expressibility and complete synthesis only.
