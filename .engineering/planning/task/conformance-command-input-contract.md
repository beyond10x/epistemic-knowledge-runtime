---
format: aep.planning-md/1
id: task:conformance-command-input-contract
kind: task
status: draft
title: Align ESS command inputs and observable transaction states with the real kernel
relations:
- derived_from: story:ess-conformance-kernel
- blocks: story:ekr-cli
revision: 1
---
## Measured prerequisites for executable conformance

Against coordinator 4032d00, ESS synthesis produces a declared-coverage inventory, but no scenario was executed. The retained preparation report records the command's own counts and refusals. The CLI only offers built-in targets; implement a Rust ConformanceTarget using the pinned ESS library and run its actual Runner, with suite/5 and report/2 evidence, not an interpreted model that returns the contract's answer.

The current spec is not yet a faithful command input contract. systems/ekr/domains/kernel.yaml declares Seed input seed_hash and Propose inputs operations_hash/evidence_hash plus counts, without the actual retained payload. Synthesis supplies ordinary String literals named after those fields; crates/ekr-core/src/hash.rs requires exact lowercase hexadecimal addresses, and the real kernel must resolve and verify the named payload. Translating these literals into unrelated hashes while echoing the original values in events would test the adapter's bookkeeping instead of the runtime. Reconcile the actual payload/handle protocol with the ESS declaration before CLI and target implementation; explicit authored scenarios may supply real payload fixtures, but must retain an exact complete coverage inventory rather than discard generated obligations.

The existing PendingTransactions view hides terminal states. The synthesis refusals say the operation_count invariant cannot be read after committed, stale, rejected or validated outcomes. Add an executable read of the retained transaction record across those states; do not delete the invariant or relabel refusal as outside scope.

External outcome controls must establish real circumstances: invalid seed input, a genuinely invalid proposed transaction, an intervening commit for staleness, or an actually missing identity. Report only observed kernel outcomes and persisted/read-back records. Never return the selected expected outcome from a control flag. Preserve actor authority outside the proposal payload.

Mutation evidence must show a named scenario fails when a kernel behavior is broken. All declared scenarios must execute and pass; no silent target skips. Add the pinned ESS library dependencies and test-runner entry point explicitly to story scope; the old assertion that no dependency is needed is false (crates/ekr/Cargo.toml declares none).
