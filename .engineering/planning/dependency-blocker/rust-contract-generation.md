---
format: aep.planning-md/3
id: dependency-blocker:rust-contract-generation
kind: dependency-blocker
status: open
title: Runtime contracts require a released recursive Rust generator
relations:
- blocks: release-plan:knowledge-inbox-schema-learning
- blocks: story:retain-supplied-knowledge
revision: 1
---
## Required prerequisite

Baseline specification validation passes, while the retained .engineering/reviews/knowledge-inbox-prerequisite/ess-0.36.0.json and ess-0.51.0.json show Rust target failures. Upstream issue https://github.com/beyond10x/ess/issues/400 records them. The existing explicit enum name/wire idiom addresses wire-label spelling without changing serialized labels; optional self-recursive structs still require a separately reviewed generator representation.

Clear only after an actual released tool synthesizes the amended EKR specification, generated code compiles, and EKR pins that verified release with a regeneration drift check. Upstream story:optional-recursive-rust owns the representation change. A local compiler patch or a declaration that validates is not the required release evidence. Do not hand-transcribe the new runtime models while blocked.
