---
format: aep.planning-md/3
id: dependency-blocker:rust-contract-generation
kind: dependency-blocker
status: open
title: Runtime contracts require a released recursive Rust generator
relations:
- blocks: release-plan:knowledge-inbox-schema-learning
revision: 2
---
## Required prerequisite

Baseline specification validation passes, while the retained .engineering/reviews/knowledge-inbox-prerequisite/ess-0.36.0.json and ess-0.51.0.json show Rust target failures. Upstream issue https://github.com/beyond10x/ess/issues/400 records them. The existing explicit enum name/wire idiom addresses wire-label spelling without changing serialized labels; optional self-recursive structs still require a separately reviewed generator representation.

Clear only after an actual released tool synthesizes the amended EKR specification, generated code compiles, and EKR pins that verified release with a regeneration drift check. Upstream story:optional-recursive-rust owns the representation change. A local compiler patch or a declaration that validates is not the required release evidence. Do not hand-transcribe the new runtime models while blocked.

## Development versus release evidence

The generator representation and emitter defects are now resolved in exact candidate 09df87a6123a5ae50d752d2e3223fc19f596c9de: ess-synth package executes 378 passed/0 failed; fresh actual EKR compiles in both crate and workspace layouts; targeted recursion, construction, multiline documentation, acyclic byte-equality and unsupported-cycle controls pass. Evidence is retained under <cache>/ekr-knowledge-prereq-20261003/implementation. Independent adversary review and remote/release checks remain pending.

Coordinator correction to an earlier overstrict scheduling inference: the operator requires generation defects resolved before generating runtime models, and an exact verified release pin for delivery. The supplied plan does not require all downstream development to wait for upstream release packaging. Development may use this exact compiling candidate and must record its commit; it may not call that candidate a released tool or complete adoption against it. The blocker continues to block the release plan until actual release/artifact verification, final regeneration and exact pin/drift checks pass. No handwritten new models are permitted at any stage.
