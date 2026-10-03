# Supplied knowledge implementation checkpoint

This directory records development verification of story A, not release acceptance.
The coordinator implemented and reviewed the changes locally; this was not independent review.

The generated specification digest is
`49c3f391cccaef430fcdc2ab9e53d395790cb0cd546b10e41292fad4183775fa`;
the contract digest is
`8c63803964860a51eaf7284bd1ba96a1676e0adb7e274e4b8c9b873e94e76fca`.
The exact development generator is recorded in `generated/ess-generator.json`.
Its release pin is still pending the independently coordinated ESS release.

Recorded commands (with the compiler wrapper unset, two build jobs, no incremental compilation
or debug symbols):

- `cargo test --locked -p ekr-kernel --test knowledge_retention`: eight tests pass. Each
  exercises both native providers. They cover retries, exact retained bytes, concurrency,
  rejected interpretations, malformed local documents, migration refusal, reopen and
  preservation of provider faults through the generated behavior adapters.
- `cargo clippy --locked -p ekr-core -p ekr-ontology -p ekr-integrate -p ekr-kernel -p ekr-store -p ekr-observe -p ekr-sdk -p ekr --all-targets -- -D warnings`: passes.
- `cargo run --locked -p xtask --bin xtask -- contracts-check --ess <candidate> --development-candidate`:
  both complete generated trees match and the synthesized workspace compiles. The command
  explicitly disclaims released-generator acceptance.
- The boundary, recovery and surface logs preserve their individual suite outputs. Recovery
  injects response loss around real atomic provider writes, drops the handle and reopens;
  it does not kill a process inside a native transaction.
- `cargo test --locked -p ekr-store --lib abrupt_exit_retention_reopens_without_duplicates`:
  the parent test passes all eight process-exit combinations (two providers, two retained
  streams, immediately before/after the native atomic call). The child exits without
  destructors. This tests process boundaries, not an interruption inside the provider transaction.
- `cargo test --locked -p ekr --test conformance_knowledge`: four tests pass in three
  consecutive runs. The real CLI reopens each provider with full replay for every command.
  The target initializes only an empty provider namespace through the public Runtime API;
  it supplies no canonical seed or retained facts. Each provider reports three selected
  scenarios passed, zero failed/error/unsupported/skipped. The discard mutation reports all
  three named scenarios failed on each provider, with zero errors or unsupported observations.
  Positive read assertions and exact integer projections are separately guarded.
- The final changed-crate Clippy and member-only formatting checks pass. Fresh synthesis with
  the recorded development generator produces 118 scenarios, three authored sources and zero
  refusals; the committed full suite matches byte for byte.

`conformance/` retains the actual report/2 documents, diagnostic runs and one original-byte
suite-input carrier with the complete parent inventory. Only the three authored scenarios are
selected; the other 115 inventory scenarios are outside this claim. The plan's underscore names
map to the hyphenated ESS identifiers in each report. Final report clocks are anchored to the
execution date; simulated progression does not measure provider latency. The pinned ESS 0.36
runner admits and executes this suite, although its compiler refuses the new identity relations
and cannot regenerate it. Final verified-generator adoption and the CI gate are still required.

Local findings corrected during implementation: capture timestamps must use the generated
RFC 3339 wire shape; the existing migration command must refuse streams it cannot carry;
and JSON-number fixtures must pass through JSON text before YAML serialization when the
generated dependency enables arbitrary precision. The existing positive and refusal assertions
were preserved. Owners: these integration findings belong to the coordinator acting as implementor.

All six observation/incubation entry points now implement and invoke generated Behavior traits;
wire-to-semantic conversions preserve typed values and infrastructure failures. No new runtime
model is handwritten. The conformance adapter initially exposed two setup mistakes (an absent
provider namespace and a missing interpretation import envelope); both were corrected before
the recorded passing runs. Numeric transport conversion is explicitly lossless.

Final released-generator adoption and the combined `task check` remain open.
Stories B–F and the two end-to-end demonstrations are not delivered by this checkpoint.
Raw logs remain in task scratch; published copies replace machine-local paths with placeholders.
