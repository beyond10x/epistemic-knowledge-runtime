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

- `cargo test --locked -p ekr-kernel --test knowledge_retention`: seven tests pass. Each
  exercises both native providers. They cover retries, exact retained bytes, concurrency,
  rejected interpretations, malformed local documents, migration refusal and reopen.
- `cargo clippy --locked -p ekr-core -p ekr-ontology -p ekr-integrate -p ekr-kernel -p ekr-store -p ekr-observe -p ekr-sdk -p ekr --all-targets -- -D warnings`: passes.
- `cargo run --locked -p xtask --bin xtask -- contracts-check --ess <candidate> --development-candidate`:
  both complete generated trees match and the synthesized workspace compiles. The command
  explicitly disclaims released-generator acceptance.
- The boundary, recovery and surface logs preserve their individual suite outputs. Recovery
  injects response loss around real atomic provider writes, drops the handle and reopens;
  it does not kill a process inside a native transaction.

Local findings corrected during implementation: capture timestamps must use the generated
RFC 3339 wire shape; the existing migration command must refuse streams it cannot carry;
and JSON-number fixtures must pass through JSON text before YAML serialization when the
generated dependency enables arbitrary precision. The existing positive and refusal assertions
were preserved. Owners: these integration findings belong to the coordinator acting as implementor.

Generated command-behavior adapters, final released-generator adoption, bound implementation
conformance reports, native crash verification and the combined `task check` remain open.
Stories B–F and the two end-to-end demonstrations are not delivered by this checkpoint.
Raw logs remain in task scratch; published copies replace machine-local paths with placeholders.
