# Conformance preparation at the seed opening

Command: ess verify conform synthesize --path systems/ekr --suite-format 5 --out <scratch>/suite.json --format json

Exit: 1. No scenarios executed. Declared inventory counts read from the output:

{
  "authored": 0,
  "generated": 26,
  "outside": 0,
  "refused": 4
}

Exact synthesis refusals:

refusal[ESS-SYNTH-011]: entity ekr.kernel.GraphTransaction has no scenario `ekr.kernel.GraphTransaction/invariant/after/ekr.kernel.Commit/committed`
  `operation_count >= 1` cannot be read after this branch: no view of `ekr.kernel.GraphTransaction` holds an instance in `Committed`
  help: declare a view that holds an instance in this state, or the invariant cannot be read after this branch
refusal[ESS-SYNTH-011]: entity ekr.kernel.GraphTransaction has no scenario `ekr.kernel.GraphTransaction/invariant/after/ekr.kernel.Commit/stale`
  `operation_count >= 1` cannot be read after this branch: no view of `ekr.kernel.GraphTransaction` holds an instance in `Stale`
  help: declare a view that holds an instance in this state, or the invariant cannot be read after this branch
refusal[ESS-SYNTH-011]: entity ekr.kernel.GraphTransaction has no scenario `ekr.kernel.GraphTransaction/invariant/after/ekr.kernel.Validate/rejected`
  `operation_count >= 1` cannot be read after this branch: no view of `ekr.kernel.GraphTransaction` holds an instance in `Rejected`
  help: declare a view that holds an instance in this state, or the invariant cannot be read after this branch
refusal[ESS-SYNTH-011]: entity ekr.kernel.GraphTransaction has no scenario `ekr.kernel.GraphTransaction/invariant/after/ekr.kernel.Validate/validated`
  `operation_count >= 1` cannot be read after this branch: no view of `ekr.kernel.GraphTransaction` holds an instance in `Validated`
  help: declare a view that holds an instance in this state, or the invariant cannot be read after this branch

Producer: ess 0.26.0

## Measured prerequisites for executable conformance

Against coordinator 4032d00, ESS synthesis produces a declared-coverage inventory, but no scenario was executed. The retained preparation report records the command's own counts and refusals. The CLI only offers built-in targets; implement a Rust ConformanceTarget using the pinned ESS library and run its actual Runner, with suite/5 and report/2 evidence, not an interpreted model that returns the contract's answer.

The current spec is not yet a faithful command input contract. systems/ekr/domains/kernel.yaml declares Seed input seed_hash and Propose inputs operations_hash/evidence_hash plus counts, without the actual retained payload. Synthesis supplies ordinary String literals named after those fields; crates/ekr-core/src/hash.rs requires exact lowercase hexadecimal addresses, and the real kernel must resolve and verify the named payload. Translating these literals into unrelated hashes while echoing the original values in events would test the adapter's bookkeeping instead of the runtime. Reconcile the actual payload/handle protocol with the ESS declaration before CLI and target implementation; explicit authored scenarios may supply real payload fixtures, but must retain an exact complete coverage inventory rather than discard generated obligations.

The existing PendingTransactions view hides terminal states. The synthesis refusals say the operation_count invariant cannot be read after committed, stale, rejected or validated outcomes. Add an executable read of the retained transaction record across those states; do not delete the invariant or relabel refusal as outside scope.

External outcome controls must establish real circumstances: invalid seed input, a genuinely invalid proposed transaction, an intervening commit for staleness, or an actually missing identity. Report only observed kernel outcomes and persisted/read-back records. Never return the selected expected outcome from a control flag. Preserve actor authority outside the proposal payload.

Mutation evidence must show a named scenario fails when a kernel behavior is broken. All declared scenarios must execute and pass; no silent target skips. Add the pinned ESS library dependencies and test-runner entry point explicitly to story scope; the old assertion that no dependency is needed is false (crates/ekr/Cargo.toml declares none).
