---
format: aep.planning-md/1
id: task:conformance-command-input-contract
kind: task
status: archived
title: Align ESS command inputs and observable transaction states with the real kernel
relations:
- derived_from: story:ess-conformance-kernel
revision: 5
---
## Measured prerequisites for executable conformance

Against coordinator 4032d00, ESS synthesis produces a declared-coverage inventory, but no scenario was executed. The retained preparation report records the command's own counts and refusals. The CLI only offers built-in targets; implement a Rust ConformanceTarget using the pinned ESS library and run its actual Runner, with suite/5 and report/2 evidence, not an interpreted model that returns the contract's answer.

The current spec is not yet a faithful command input contract. systems/ekr/domains/kernel.yaml declares Seed input seed_hash and Propose inputs operations_hash/evidence_hash plus counts, without the actual retained payload. Synthesis supplies ordinary String literals named after those fields; crates/ekr-core/src/hash.rs requires exact lowercase hexadecimal addresses, and the real kernel must resolve and verify the named payload. Translating these literals into unrelated hashes while echoing the original values in events would test the adapter's bookkeeping instead of the runtime. Reconcile the actual payload/handle protocol with the ESS declaration before CLI and target implementation; explicit authored scenarios may supply real payload fixtures, but must retain an exact complete coverage inventory rather than discard generated obligations.

The existing PendingTransactions view hides terminal states. The synthesis refusals say the operation_count invariant cannot be read after committed, stale, rejected or validated outcomes. Add an executable read of the retained transaction record across those states; do not delete the invariant or relabel refusal as outside scope.

External outcome controls must establish real circumstances: invalid seed input, a genuinely invalid proposed transaction, an intervening commit for staleness, or an actually missing identity. Report only observed kernel outcomes and persisted/read-back records. Never return the selected expected outcome from a control flag. Preserve actor authority outside the proposal payload.

Mutation evidence must show a named scenario fails when a kernel behavior is broken. All declared scenarios must execute and pass; no silent target skips. Add the pinned ESS library dependencies and test-runner entry point explicitly to story scope; the old assertion that no dependency is needed is false (crates/ekr/Cargo.toml declares none).

## Concrete document and transaction-view scope

The read-only story-scoper's measured report is retained at
`.engineering/waves/p1-conformance-document-contract.md`, with an explicitly
unapplied candidate patch beside it. Its reported synthesis changes the original
four refusals to executable obligations by introducing a retained Transactions
view and real document-path inputs. This is synthesis evidence, not runtime
execution. The complete generated inventory must remain alongside authored cases.

Implement the approved Seed and Propose verbs through one shared document handler:
parse the actual versioned payload, bind independent host identity, and derive
hashes/counts from it. Do not let the conformance target translate placeholder
hashes, overwrite retained proposals, remap revision numbers or fabricate outcomes.
Fixture arrangements must create real rejection/staleness circumstances, and the
Transactions view must read retained kernel records after terminal outcomes and
restart. This is also a writer prerequisite; adapter-only state is insufficient.

The measured patch is not yet adopted. Reconcile its opening seed format and
outdated seed-root summary with the completed seed and persisted-contract work
before editing the normative ESS. Known-fixture hash/readback assertions and the
named mutations in the report must execute through the actual ESS Runner before
this task can close.

## Wave p1-13 rescope

Rescoped in wave p1-13 (2026-09-23).

**Done:** the CLI half. Seed and Propose read document bytes through one shared handler; hashes
and counts are derived from the payload by the kernel; the proposer is bound to the host operator.
Held by `crates/ekr/tests/retraction_example.rs` and
`crates/ekr/tests/adversary_p1_13_cli_exit_contract.rs` (wave p1-13 gate: 730 passed).

**Still open, and the only thing this task now covers (wave p1-14):** the ESS half — a retained
Transactions view readable across terminal states and restart, real document-path inputs for
Seed and Propose in `systems/ekr/domains/kernel.yaml` instead of hash placeholders, and the
Rust `ConformanceTarget` executing them. It now blocks `story:ess-conformance-kernel` instead of
`story:ekr-cli`.

## Wave p1-14 close

Archived in wave p1-14 (2026-09-23) after the plan review of 72779f9.

The ESS half this task still claimed is already declared: document-path inputs for Seed and
Propose (`systems/ekr/domains/kernel.yaml` :77-83, :949-950, :986-987) and the all-states
Transactions view (:1270-1304). What remains, executing them through a Rust ConformanceTarget, is
the acceptance of `story:ess-conformance-kernel` itself, so the blocks edge was a cycle.
