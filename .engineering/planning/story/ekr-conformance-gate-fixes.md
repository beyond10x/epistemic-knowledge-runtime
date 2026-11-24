---
format: aep.planning-md/3
id: story:ekr-conformance-gate-fixes
kind: story
status: active
title: ESS gate covers the CLI, checks stored facts, user build skips the conformance compiler
summary: The ESS gate covers every specified CLI command, extraction conformance checks the stored fact, and a user build of the CLI does not compile the conformance compiler
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T08:29:12Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T08:29:12Z", actor: "human:timo", revision: 3}
---
## Outcome

The ESS gate covers every specified CLI command, extraction conformance checks the stored fact, and a user build of the CLI does not compile the conformance compiler.

## Work

Review of EKR 0.0.42, findings 40 41 42. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

40. The ESS gate omits five specified CLI commands. The three committed component suites exclude the same 31 CLI command outcomes and nine stage state cases. BeginStage, SealStage, PublishStage, AbandonStage and CompareStores have hand-written coverage. Connect the CLI component to File and SQLite and measure its executed command and entity scenarios separately.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/components.yaml#L95
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/conformance/suite.json#L465
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/conformance/suite.json#L524
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/Taskfile.yml#L100
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/epistemic-knowledge-runtime-design.md#L5892


41. Extraction conformance checks reports without checking the stored fact. The two admitted integration scenarios derive their observed outcome from the arrays the command returned. An incorrect fact or citation could leave those arrays unchanged. Read the persisted fact and evidence in the accepted case, and check the revision and surviving valid peer in the refusal case. Other hand-written extraction tests already inspect state.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/conformance/integrate.rs#L145
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/conformance/integrate-baseline.json#L19
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/conformance_integrate.rs#L370
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/extraction_cli.rs#L1154


42. Building the CLI for a user compiles the ESS conformance compiler. The ordinary locked Linux binary build compiled ess-compiler. The two unconditional conformance dependencies account for 12 packages reachable only through those edges. Gate the adapter behind an opt-in feature or a test-support crate, then exercise both the install and the conformance entry point.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/Cargo.toml#L23
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/lib.rs#L7
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/main.rs#L13
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/conformance.rs#L13
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/Cargo.toml#L22


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
